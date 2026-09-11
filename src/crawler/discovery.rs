//! Bounded traversal of explicitly supplied sitemap documents.
use super::{add_node, fetcher, robots::Robots, sitemap, CrawlOptions};
use crate::{urls, PageNode};
use reqwest::Client;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use url::Url;

pub(super) async fn discover(
    start: Url,
    root: &Url,
    options: &CrawlOptions,
    client: &Client,
    robots: &mut Robots,
    nodes: &mut BTreeMap<String, PageNode>,
    warnings: &mut Vec<String>,
) -> BTreeSet<String> {
    let mut sitemap_seeds = BTreeSet::new();
    let mut maps = VecDeque::from([start]);
    let mut seen = BTreeSet::new();
    while let Some(url) = maps.pop_front() {
        if seen.contains(url.as_str()) {
            continue;
        }
        if seen.len() >= options.max_sitemaps {
            warnings.push("sitemap document limit reached".into());
            break;
        }
        seen.insert(url.to_string());
        if !urls::internal(root, &url, options.include_subdomains) {
            warnings.push(format!("out-of-scope sitemap ignored: {url}"));
            continue;
        }
        if !robots.allowed(&url, options).await {
            warnings.push(format!("sitemap blocked by robots.txt: {url}"));
            continue;
        }
        let response = match fetcher::fetch(client, &url, options.max_body_bytes, false).await {
            Ok(r) => r,
            Err(e) => {
                warnings.push(format!("sitemap {url}: {e}"));
                continue;
            }
        };
        if matches!(response.status, 301 | 302 | 303 | 307 | 308) {
            if let Some(target) = response
                .location
                .as_deref()
                .and_then(|v| urls::resolve(&url, v))
            {
                maps.push_back(target);
            } else {
                warnings.push(format!("sitemap redirect without valid Location: {url}"));
            }
            continue;
        }
        if !(200..300).contains(&response.status) {
            warnings.push(format!("sitemap {url}: HTTP {}", response.status));
            continue;
        }
        match sitemap::parse(&response.text, &url) {
            Ok(map) => {
                for target in map.urls {
                    if map.index {
                        // Bound queued documents as well as processed documents.
                        if maps.len() + seen.len() < options.max_sitemaps {
                            maps.push_back(target);
                        } else if !warnings
                            .iter()
                            .any(|w| w == "sitemap document limit reached")
                        {
                            warnings.push("sitemap document limit reached".into());
                        }
                    } else if urls::internal(root, &target, options.include_subdomains)
                        && urls::html_candidate(&target)
                    {
                        if sitemap_seeds.len() >= options.max_sitemap_urls {
                            if !warnings.iter().any(|w| w == "sitemap URL limit reached") {
                                warnings.push("sitemap URL limit reached".into());
                            }
                            break;
                        }
                        sitemap_seeds.insert(target.to_string());
                        add_node(nodes, root, target.clone(), options.include_subdomains);
                        if let Some(n) = nodes.get_mut(target.as_str()) {
                            n.sitemap = true;
                        }
                    }
                }
            }
            Err(e) => warnings.push(format!("sitemap {url}: {e}")),
        }
    }
    sitemap_seeds
}
