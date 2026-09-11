use crate::{urls, Error, Result};
use url::Url;

pub struct Sitemap {
    pub index: bool,
    pub urls: Vec<Url>,
}
pub fn parse(text: &str, base: &Url) -> Result<Sitemap> {
    let document = roxmltree::Document::parse(text)
        .map_err(|e| Error::Invalid(format!("invalid sitemap: {e}")))?;
    let root = document.root_element();
    let index = match root.tag_name().name() {
        "sitemapindex" => true,
        "urlset" => false,
        _ => {
            return Err(Error::Invalid(
                "expected sitemap urlset or sitemapindex".into(),
            ))
        }
    };
    let entry = if index { "sitemap" } else { "url" };
    let namespace = root.tag_name().namespace();
    // Extension elements (for example image:loc) are not page locations.
    let matches = |node: &roxmltree::Node<'_, '_>, name: &str| {
        node.is_element()
            && node.tag_name().name() == name
            && node.tag_name().namespace() == namespace
    };
    let mut urls = Vec::new();
    for node in root.children().filter(|n| matches(n, entry)) {
        if let Some(loc) = node
            .children()
            .find(|n| matches(n, "loc"))
            .and_then(|n| n.text())
        {
            if let Some(url) = urls::resolve(base, loc) {
                urls.push(url);
            }
        }
    }
    urls.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    urls.dedup();
    Ok(Sitemap { index, urls })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn namespaces_entities_and_index() {
        let base = Url::parse("https://example.com/sitemap.xml").unwrap();
        let map = parse("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>https://example.com/?a=1&amp;b=2</loc><image:loc xmlns:image='images'>https://example.com/image</image:loc></url></urlset>", &base).unwrap();
        assert_eq!(map.urls.len(), 1);
        assert_eq!(map.urls[0].query(), Some("a=1&b=2"));
        assert!(
            parse(
                "<sitemapindex><sitemap><loc>/a.xml</loc></sitemap></sitemapindex>",
                &base
            )
            .unwrap()
            .index
        );
        assert!(parse("<urlset>", &base).is_err());
    }
    #[test]
    fn extension_locations_are_not_page_urls() {
        let base = Url::parse("https://example.com/sitemap.xml").unwrap();
        let map = parse("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9' xmlns:image='images'><url><image:loc>/wrong</image:loc><loc>/page</loc></url><url><image:loc>/also-wrong</image:loc></url></urlset>", &base).unwrap();
        assert_eq!(map.urls, vec![base.join("/page").unwrap()]);
    }
}
