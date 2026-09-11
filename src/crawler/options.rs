use crate::{Error, Result};
use std::time::Duration;

#[derive(Debug, Clone)]
/// Explicit crawl budgets. Default robots behavior is conservative on unavailable rules.
pub struct CrawlOptions {
    /// Maximum page requests, including errors and redirect hops (not robots/sitemaps).
    pub max_pages: usize,
    /// Hyperlink hops from each seed; sitemap-only components have their own crawl budget.
    pub max_depth: usize,
    pub concurrency: usize,
    pub timeout: Duration,
    pub user_agent: String,
    pub include_subdomains: bool,
    pub respect_robots: bool,
    pub max_body_bytes: usize,
    pub max_sitemaps: usize,
    pub max_sitemap_urls: usize,
    pub damping: f64,
}
impl Default for CrawlOptions {
    fn default() -> Self {
        Self {
            max_pages: 1000,
            max_depth: 10,
            concurrency: 5,
            timeout: Duration::from_secs(20),
            user_agent: "ahref/1.0.0".into(),
            include_subdomains: false,
            respect_robots: true,
            max_body_bytes: 5 * 1024 * 1024,
            max_sitemaps: 100,
            max_sitemap_urls: 100_000,
            damping: 0.85,
        }
    }
}
impl CrawlOptions {
    /// Validate budgets before making any network requests.
    pub fn validate(&self) -> Result<()> {
        if self.max_pages == 0
            || self.concurrency == 0
            || self.timeout.is_zero()
            || self.max_body_bytes == 0
            || self.max_sitemaps == 0
            || self.max_sitemap_urls == 0
        {
            return Err(Error::Invalid(
                "page, concurrency, timeout, body and sitemap limits must be positive".into(),
            ));
        }
        if self.user_agent.trim().is_empty() || !(0.0..1.0).contains(&self.damping) {
            return Err(Error::Invalid(
                "provide a user agent and damping in [0, 1)".into(),
            ));
        }
        Ok(())
    }
}
