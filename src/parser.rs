use crate::{
    graph::{LinkEdge, LinkType},
    urls,
};
use scraper::{Html, Selector};
use url::Url;

/// Extract HTTP(S) hyperlink occurrences in document order using the document base URL.
pub fn links(html: &str, source: &Url, root: &Url, subdomains: bool) -> Vec<LinkEdge> {
    let document = Html::parse_document(html);
    let base = Selector::parse("base[href]")
        .ok()
        .and_then(|selector| {
            document
                .select(&selector)
                .next()
                .and_then(|e| e.value().attr("href"))
                .and_then(|href| urls::resolve(source, href))
        })
        .unwrap_or_else(|| source.clone());
    let Ok(selector) = Selector::parse("a[href]") else {
        return Vec::new();
    };
    document
        .select(&selector)
        .filter_map(|element| {
            let to = urls::resolve(&base, element.value().attr("href")?)?;
            let kind = if urls::internal(root, &to, subdomains) {
                LinkType::Internal
            } else {
                LinkType::External
            };
            Some(LinkEdge {
                from: source.clone(),
                to,
                anchor: element
                    .text()
                    .collect::<String>()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" "),
                kind,
                rel: element
                    .value()
                    .attr("rel")
                    .unwrap_or_default()
                    .split_whitespace()
                    .map(str::to_ascii_lowercase)
                    .collect(),
            })
        })
        .collect()
}

/// Legacy parser API, now backed by an HTML5 parser.
pub struct Parser {
    html: String,
}
impl Parser {
    pub fn new(html: String) -> Self {
        Self { html }
    }
    pub fn parse_tags(&mut self) -> Vec<String> {
        let document = Html::parse_document(&self.html);
        let Ok(selector) = Selector::parse("a") else {
            return Vec::new();
        };
        document.select(&selector).map(|e| e.html()).collect()
    }
    pub fn parse_links(&mut self) -> Vec<String> {
        let document = Html::parse_document(&self.html);
        let Ok(selector) = Selector::parse("a[href]") else {
            return Vec::new();
        };
        document
            .select(&selector)
            .filter_map(|e| e.value().attr("href").map(str::to_owned))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn html5_links_keep_occurrences_and_metadata() {
        let root = Url::parse("https://example.com/").unwrap();
        let edges = links("<base href='/docs/'><a href=../a rel='nofollow'>Hello <b>world</b></a><a href='../a'>again</a><a href='#x'>skip</a><a href='https://other.org?a=1&amp;b=2'>external", &root, &root, false);
        assert_eq!(edges.len(), 3);
        assert_eq!(edges[0].anchor, "Hello world");
        assert_eq!(edges[0].to.as_str(), "https://example.com/a");
        assert_eq!(edges[0].rel, vec!["nofollow"]);
        assert_eq!(edges[2].kind, LinkType::External);
        assert!(edges[2].to.as_str().contains("&b=2"));
    }
    #[test]
    fn inline_markup_does_not_invent_anchor_spaces() {
        let root = Url::parse("https://example.com/").unwrap();
        let edges = links("<a href='/'>foo<b>bar</b>!</a>", &root, &root, false);
        assert_eq!(edges[0].anchor, "foobar!");
    }
}
