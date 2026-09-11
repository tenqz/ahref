use super::{pagerank::pagerank, *};
use crate::{urls, Error, Result};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

impl Graph {
    /// Reject unsupported schemas and inconsistent URL identities or endpoints before analysis.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != "1.0" {
            return Err(Error::Invalid(format!(
                "unsupported schema_version: {}",
                self.schema_version
            )));
        }
        if !self.site.damping.is_finite() || !(0.0..1.0).contains(&self.site.damping) {
            return Err(Error::Invalid("damping must be in [0, 1)".into()));
        }
        if urls::normalize(self.site.root.clone()).as_ref() != Some(&self.site.root) {
            return Err(Error::Invalid(
                "site root must be a normalized HTTP(S) URL".into(),
            ));
        }
        let mut nodes = BTreeMap::new();
        for node in &self.nodes {
            let kind = if urls::internal(&self.site.root, &node.url, self.site.include_subdomains) {
                LinkType::Internal
            } else {
                LinkType::External
            };
            if nodes.insert(node.url.as_str(), node).is_some()
                || urls::normalize(node.url.clone()).as_ref() != Some(&node.url)
                || node.kind != kind
            {
                return Err(Error::Invalid(format!(
                    "duplicate, non-normalized or misclassified node: {}",
                    node.url
                )));
            }
        }
        for node in &self.nodes {
            if node
                .redirect_target
                .as_ref()
                .is_some_and(|u| !nodes.contains_key(u.as_str()))
            {
                return Err(Error::Invalid(
                    "redirect target is missing from nodes".into(),
                ));
            }
        }
        for edge in &self.edges {
            if !nodes.contains_key(edge.from.as_str())
                || nodes
                    .get(edge.to.as_str())
                    .is_none_or(|n| n.kind != edge.kind)
            {
                return Err(Error::Invalid(
                    "edge has a missing endpoint or invalid kind".into(),
                ));
            }
        }
        Ok(())
    }

    /// Recompute metrics from occurrences; degree and PageRank use unique neighbors.
    pub fn analyze(&mut self) -> Result<()> {
        self.validate()?;
        self.nodes
            .sort_by(|a, b| a.url.as_str().cmp(b.url.as_str()));
        self.edges.sort_by(|a, b| {
            (a.from.as_str(), a.to.as_str(), &a.anchor, &a.rel).cmp(&(
                b.from.as_str(),
                b.to.as_str(),
                &b.anchor,
                &b.rel,
            ))
        });
        let index: BTreeMap<String, usize> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.url.to_string(), i))
            .collect();
        let len = self.nodes.len();
        let mut incoming = vec![BTreeSet::new(); len];
        let mut outgoing = vec![BTreeSet::new(); len];
        for node in &mut self.nodes {
            node.in_degree = 0;
            node.out_degree = 0;
            node.depth = None;
            node.pagerank = None;
            node.internal_incoming_links = 0;
            node.internal_outgoing_links = 0;
            node.external_outgoing_links = 0;
        }
        for edge in &self.edges {
            let (a, b) = (index[edge.from.as_str()], index[edge.to.as_str()]);
            incoming[b].insert(a);
            outgoing[a].insert(b);
            if self.nodes[a].kind == LinkType::Internal {
                if edge.kind == LinkType::Internal {
                    self.nodes[a].internal_outgoing_links += 1;
                    self.nodes[b].internal_incoming_links += 1;
                } else {
                    self.nodes[a].external_outgoing_links += 1;
                }
            }
        }
        for i in 0..len {
            self.nodes[i].in_degree = incoming[i].len();
            self.nodes[i].out_degree = outgoing[i].len();
        }
        // 0-1 BFS: redirects preserve depth, hyperlinks add one hop.
        let mut queue = VecDeque::new();
        if let Some(&root) = index.get(self.site.root.as_str()) {
            self.nodes[root].depth = Some(0);
            queue.push_back(root);
        }
        while let Some(a) = queue.pop_front() {
            let Some(depth) = self.nodes[a].depth else {
                continue;
            };
            let mut next: Vec<(usize, usize)> = outgoing[a].iter().map(|&b| (b, 1)).collect();
            if let Some(target) = &self.nodes[a].redirect_target {
                next.push((index[target.as_str()], 0));
            }
            for (b, cost) in next {
                if self.nodes[b].is_internal_page()
                    && self.nodes[b].depth.is_none_or(|d| d > depth + cost)
                {
                    self.nodes[b].depth = Some(depth + cost);
                    if cost == 0 {
                        queue.push_front(b);
                    } else {
                        queue.push_back(b);
                    }
                }
            }
        }
        let pages: Vec<usize> = (0..len)
            .filter(|&i| self.nodes[i].is_internal_page())
            .collect();
        let page_index: BTreeMap<usize, usize> =
            pages.iter().enumerate().map(|(i, &n)| (n, i)).collect();
        let adjacency: Vec<Vec<usize>> = pages
            .iter()
            .map(|&a| {
                outgoing[a]
                    .iter()
                    .filter_map(|b| page_index.get(b).copied())
                    .collect()
            })
            .collect();
        let (ranks, iterations, converged) = pagerank(&adjacency, self.site.damping);
        for (&n, rank) in pages.iter().zip(ranks) {
            self.nodes[n].pagerank = Some(rank);
        }
        let mut summary = Summary {
            pages: pages.len(),
            edges: self.edges.len(),
            pagerank_iterations: iterations,
            pagerank_converged: converged,
            ..Summary::default()
        };
        let mut depths = Vec::new();
        for &i in &pages {
            let n = &self.nodes[i];
            if let Some(d) = n.depth {
                depths.push(d);
            } else {
                summary.unreachable_pages += 1;
            }
            if n.state == FetchState::Fetched {
                summary.fetched_pages += 1;
            }
            if n.state == FetchState::Pending {
                summary.pending_pages += 1;
            }
            if n.state == FetchState::Failed {
                summary.failed_pages += 1;
            }
            if n.state == FetchState::BlockedRobots {
                summary.blocked_pages += 1;
            }
            if n.is_html == Some(true)
                && n.status_code.is_some_and(|s| (200..300).contains(&s))
                && n.state == FetchState::Fetched
                && n.internal_outgoing_links == 0
            {
                summary.dead_end_pages += 1;
            }
            let other_incoming = incoming[i]
                .iter()
                .filter(|&&a| a != i && self.nodes[a].kind == LinkType::Internal)
                .count();
            if n.sitemap && n.url != self.site.root && other_incoming == 0 {
                summary.orphan_pages += 1;
            }
            if n.url != self.site.root && other_incoming == 1 {
                summary.weakly_linked_pages += 1;
            }
        }
        summary.max_depth = depths.iter().copied().max().unwrap_or(0);
        summary.average_depth = if depths.is_empty() {
            0.0
        } else {
            depths.iter().sum::<usize>() as f64 / depths.len() as f64
        };
        summary.internal_links = self
            .edges
            .iter()
            .filter(|e| e.kind == LinkType::Internal)
            .count();
        summary.external_links = self.edges.len() - summary.internal_links;
        summary.external_domains = self
            .nodes
            .iter()
            .filter(|n| n.kind == LinkType::External)
            .filter_map(|n| n.url.host_str())
            .collect::<BTreeSet<_>>()
            .len();
        summary.broken_urls = self
            .nodes
            .iter()
            .filter(|n| n.status_code.is_some_and(|s| s >= 400))
            .count();
        let broken = broken_targets(&self.nodes, &index);
        summary.broken_links = self
            .edges
            .iter()
            .filter(|e| broken[index[e.to.as_str()]])
            .count();
        summary.redirects = self
            .nodes
            .iter()
            .filter(|n| n.redirect_target.is_some())
            .count();
        self.site.summary = summary;
        Ok(())
    }
}

// Memoize terminal status along redirect paths, including loops, in O(V log V).
fn broken_targets(nodes: &[PageNode], index: &BTreeMap<String, usize>) -> Vec<bool> {
    let mut known = vec![None; nodes.len()];
    for start in 0..nodes.len() {
        if known[start].is_some() {
            continue;
        }
        let mut path = BTreeSet::new();
        let mut current = start;
        let broken = loop {
            if let Some(value) = known[current] {
                break value;
            }
            if !path.insert(current) {
                break false;
            }
            if nodes[current].status_code.is_some_and(|s| s >= 400) {
                break true;
            }
            match &nodes[current].redirect_target {
                Some(target) => current = index[target.as_str()],
                None => break false,
            }
        };
        for node in path {
            known[node] = Some(broken);
        }
    }
    known.into_iter().map(|v| v.unwrap_or(false)).collect()
}
