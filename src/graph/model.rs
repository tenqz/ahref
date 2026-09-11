use crate::urls;
use serde::{Deserialize, Serialize};
use url::Url;

/// Site membership is determined by hostname and the include-subdomains option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    Internal,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Fetch outcome, kept separate from HTTP status and graph reachability.
pub enum FetchState {
    Pending,
    Fetched,
    Redirect,
    BlockedRobots,
    SkippedResource,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A unique normalized HTTP(S) URL, including unvisited external URLs and assets.
pub struct PageNode {
    pub url: Url,
    pub kind: LinkType,
    pub status_code: Option<u16>,
    /// Shortest hyperlink distance from site.root; redirects cost zero, unreachable is None.
    pub depth: Option<usize>,
    pub state: FetchState,
    pub is_html: Option<bool>,
    /// Whether this URL was independently discovered in an explicitly supplied sitemap.
    pub sitemap: bool,
    pub redirect_target: Option<Url>,
    pub error: Option<String>,
    /// Number of distinct incoming source URLs, including self-links.
    pub in_degree: usize,
    /// Number of distinct outgoing target URLs, internal and external.
    pub out_degree: usize,
    pub internal_incoming_links: usize,
    pub internal_outgoing_links: usize,
    pub external_outgoing_links: usize,
    /// Internal-only rank; internal page ranks sum to one. Resources/external URLs use None.
    pub pagerank: Option<f64>,
}
impl PageNode {
    /// Construct an unvisited node; known file extensions are excluded from crawling.
    pub fn new(url: Url, kind: LinkType) -> Self {
        let state = if urls::html_candidate(&url) {
            FetchState::Pending
        } else {
            FetchState::SkippedResource
        };
        Self {
            url,
            kind,
            status_code: None,
            depth: None,
            state,
            is_html: None,
            sitemap: false,
            redirect_target: None,
            error: None,
            in_degree: 0,
            out_degree: 0,
            internal_incoming_links: 0,
            internal_outgoing_links: 0,
            external_outgoing_links: 0,
            pagerank: None,
        }
    }
    /// Include known and pending internal page candidates, excluding confirmed resources.
    pub fn is_internal_page(&self) -> bool {
        self.kind == LinkType::Internal
            && self.state != FetchState::SkippedResource
            && self.is_html != Some(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// One occurrence of an HTML hyperlink. Duplicates intentionally retain anchor/rel data.
pub struct LinkEdge {
    pub from: Url,
    pub to: Url,
    pub anchor: String,
    pub kind: LinkType,
    pub rel: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Site-wide statistics for the observed graph; see docs/schema.md for exact semantics.
pub struct Summary {
    pub pages: usize,
    pub fetched_pages: usize,
    pub edges: usize,
    pub internal_links: usize,
    pub external_links: usize,
    pub external_domains: usize,
    pub broken_links: usize,
    pub broken_urls: usize,
    pub redirects: usize,
    pub dead_end_pages: usize,
    pub orphan_pages: usize,
    pub weakly_linked_pages: usize,
    pub max_depth: usize,
    pub average_depth: f64,
    pub unreachable_pages: usize,
    pub pending_pages: usize,
    pub failed_pages: usize,
    pub blocked_pages: usize,
    pub pagerank_iterations: usize,
    pub pagerank_converged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Crawl scope, analysis settings and warnings about incomplete observations.
pub struct Site {
    pub root: Url,
    pub include_subdomains: bool,
    pub damping: f64,
    pub summary: Summary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Stable schema 1.0 interchange object, independent of a graph-library implementation.
pub struct Graph {
    pub schema_version: String,
    pub site: Site,
    pub nodes: Vec<PageNode>,
    pub edges: Vec<LinkEdge>,
}
