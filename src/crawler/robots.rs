use super::{fetcher, CrawlOptions};
use crate::urls;
use reqwest::Client;
use std::collections::BTreeMap;
use url::Url;

pub(super) struct Robots {
    pub(super) client: Client,
    pub(super) rules: BTreeMap<String, String>,
    pub(super) warnings: Vec<String>,
}
impl Robots {
    pub(super) async fn allowed(&mut self, url: &Url, options: &CrawlOptions) -> bool {
        if !options.respect_robots {
            return true;
        }
        let origin = url.origin().ascii_serialization();
        if !self.rules.contains_key(&origin) {
            let mut robots_url = url.clone();
            robots_url.set_path("/robots.txt");
            robots_url.set_query(None);
            let mut rule = None;
            for _ in 0..=5 {
                match fetcher::fetch(
                    &self.client,
                    &robots_url,
                    options.max_body_bytes.min(512_000),
                    false,
                )
                .await
                {
                    Ok(r) if (200..300).contains(&r.status) => {
                        rule = Some(r.text);
                        break;
                    }
                    Ok(r) if (300..400).contains(&r.status) => {
                        if let Some(next) = r
                            .location
                            .as_deref()
                            .and_then(|v| urls::resolve(&robots_url, v))
                        {
                            // Do not turn robots retrieval into an out-of-scope crawl.
                            if next.host_str() == url.host_str() {
                                robots_url = next;
                                continue;
                            }
                        }
                        break;
                    }
                    Ok(r) if (400..500).contains(&r.status) && r.status != 429 => {
                        rule = Some(String::new());
                        break;
                    }
                    _ => break,
                }
            }
            let rule = rule.unwrap_or_else(|| {
                self.warnings.push(format!(
                    "robots.txt unavailable for {origin}; blocked this origin (retry later)"
                ));
                "User-agent: *\nDisallow: /".into()
            });
            self.rules.insert(origin.clone(), rule);
        }
        let Some(rule) = self.rules.get(&origin) else {
            return false;
        };
        let agent = options
            .user_agent
            .split(['/', ' '])
            .next()
            .unwrap_or("ahref");
        robotstxt::DefaultMatcher::default().one_agent_allowed_by_robots(rule, agent, url.as_str())
    }
}
