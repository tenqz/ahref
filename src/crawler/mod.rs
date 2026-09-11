mod discovery;
mod fetcher;
mod options;
mod robots;
mod sitemap;
pub use options::CrawlOptions;
use robots::Robots;

use crate::{
    graph::{FetchState, Site, Summary},
    parser, urls, Error, Graph, LinkType, PageNode, Result,
};
use futures_util::{stream, StreamExt};
use reqwest::Client;
use std::collections::{BTreeMap, BTreeSet};
use url::Url;

fn add_node(nodes: &mut BTreeMap<String, PageNode>, root: &Url, url: Url, subdomains: bool) {
    let kind = if urls::internal(root, &url, subdomains) {
        LinkType::Internal
    } else {
        LinkType::External
    };
    nodes
        .entry(url.to_string())
        .or_insert_with(|| PageNode::new(url, kind));
}

/// Crawl internal HTML. External URLs and known assets are recorded, never fetched.
pub async fn crawl(input: &str, options: CrawlOptions) -> Result<Graph> {
    options.validate()?;
    let input = if input.contains("://") {
        input.to_owned()
    } else {
        format!("https://{input}")
    };
    let start = urls::normalize(Url::parse(&input)?)
        .ok_or_else(|| Error::Invalid("start URL must use HTTP(S) without credentials".into()))?;
    let is_sitemap = start.path().ends_with(".xml") || start.path().ends_with(".xml.gz");
    let mut root = start.clone();
    if is_sitemap {
        root.set_path("/");
        root.set_query(None);
    }
    let client = Client::builder()
        .user_agent(&options.user_agent)
        .timeout(options.timeout)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let mut robots = Robots {
        client: client.clone(),
        rules: BTreeMap::new(),
        warnings: Vec::new(),
    };
    let mut nodes = BTreeMap::new();
    add_node(&mut nodes, &root, root.clone(), options.include_subdomains);
    let mut warnings = Vec::new();
    let sitemap_seeds = if is_sitemap {
        discovery::discover(
            start,
            &root,
            &options,
            &client,
            &mut robots,
            &mut nodes,
            &mut warnings,
        )
        .await
    } else {
        BTreeSet::new()
    };
    // Sorted breadth-first waves give deterministic scheduling independent of response timing.
    let mut frontier = BTreeMap::from([(root.to_string(), (0usize, 0usize))]);
    let mut edges = Vec::new();
    let mut requests = 0;
    let mut seeded = false;
    loop {
        if frontier.is_empty() {
            if seeded {
                break;
            }
            seeded = true;
            // Additional sitemap components are crawl seeds, not zero-depth graph roots.
            for url in &sitemap_seeds {
                frontier.insert(url.clone(), (0, 0));
            }
            if frontier.is_empty() {
                break;
            }
        }
        let min_depth = frontier.values().map(|v| v.0).min().unwrap_or(0);
        let keys: Vec<_> = frontier
            .iter()
            .filter(|(_, v)| v.0 == min_depth)
            .map(|(k, _)| k.clone())
            .collect();
        let mut wave = Vec::new();
        for key in keys {
            if let Some(value) = frontier.remove(&key) {
                wave.push((key, value));
            }
        }
        let mut jobs = Vec::new();
        for (key, (depth, hops)) in wave {
            let Some(node) = nodes.get(&key) else {
                continue;
            };
            if node.state != FetchState::Pending
                || node.kind != LinkType::Internal
                || depth > options.max_depth
            {
                continue;
            }
            if requests + jobs.len() >= options.max_pages {
                break;
            }
            let url = node.url.clone();
            if !robots.allowed(&url, &options).await {
                if let Some(node) = nodes.get_mut(&key) {
                    node.state = FetchState::BlockedRobots;
                }
                continue;
            }
            jobs.push((url, depth, hops));
        }
        requests += jobs.len();
        let mut results = stream::iter(jobs.into_iter().map(|(url, depth, hops)| {
            let client = &client;
            let limit = options.max_body_bytes;
            async move {
                let result = fetcher::fetch(client, &url, limit, true).await;
                (url, depth, hops, result)
            }
        }))
        .buffer_unordered(options.concurrency);
        let mut completed = Vec::new();
        while let Some(result) = results.next().await {
            // Parse and drop each body immediately; only graph records await ordered merge.
            let (url, depth, hops, result) = result;
            let parsed = result.map(|response| {
                let links = if !response.skipped && (200..300).contains(&response.status) {
                    parser::links(&response.text, &url, &root, options.include_subdomains)
                } else {
                    Vec::new()
                };
                (response.status, response.location, response.skipped, links)
            });
            completed.push((url, depth, hops, parsed));
        }
        completed.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        for (url, depth, hops, result) in completed {
            match result {
                Err(e) => {
                    if let Some(n) = nodes.get_mut(url.as_str()) {
                        n.state = FetchState::Failed;
                        n.error = Some(e.to_string());
                    }
                }
                Ok((status, location, skipped, links)) => {
                    if let Some(n) = nodes.get_mut(url.as_str()) {
                        n.status_code = Some(status);
                        n.state = FetchState::Fetched;
                    }
                    if matches!(status, 301 | 302 | 303 | 307 | 308) {
                        if let Some(target) =
                            location.as_deref().and_then(|v| urls::resolve(&url, v))
                        {
                            add_node(
                                &mut nodes,
                                &root,
                                target.clone(),
                                options.include_subdomains,
                            );
                            if let Some(n) = nodes.get_mut(url.as_str()) {
                                n.state = FetchState::Redirect;
                                n.redirect_target = Some(target.clone());
                            }
                            if hops < 10 {
                                frontier
                                    .entry(target.to_string())
                                    .and_modify(|v| *v = (*v).min((depth, hops + 1)))
                                    .or_insert((depth, hops + 1));
                            } else {
                                warnings.push(format!("redirect hop limit reached at {url}"));
                            }
                        } else if let Some(n) = nodes.get_mut(url.as_str()) {
                            n.error = Some("redirect has no valid HTTP(S) Location".into());
                        }
                    } else if (200..300).contains(&status) {
                        if let Some(n) = nodes.get_mut(url.as_str()) {
                            n.is_html = Some(!skipped);
                            if skipped {
                                n.state = FetchState::SkippedResource;
                            }
                        }
                        for edge in links {
                            add_node(
                                &mut nodes,
                                &root,
                                edge.to.clone(),
                                options.include_subdomains,
                            );
                            frontier
                                .entry(edge.to.to_string())
                                .and_modify(|v| *v = (*v).min((depth + 1, 0)))
                                .or_insert((depth + 1, 0));
                            edges.push(edge);
                        }
                    }
                }
            }
        }
        if requests >= options.max_pages {
            warnings.push(format!("page request limit reached ({requests})"));
            break;
        }
    }
    warnings.extend(robots.warnings);
    warnings.sort();
    warnings.dedup();
    let mut graph = Graph {
        schema_version: "1.0".into(),
        site: Site {
            root,
            include_subdomains: options.include_subdomains,
            damping: options.damping,
            summary: Summary::default(),
            warnings,
        },
        nodes: nodes.into_values().collect(),
        edges,
    };
    graph.analyze()?;
    if graph.site.summary.pending_pages > 0 {
        graph
            .site
            .warnings
            .push("some pages were not fetched; findings apply only to the observed graph".into());
    }
    Ok(graph)
}
