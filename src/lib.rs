//! Crawl a website into a portable directed hyperlink graph.
//!
//! ```no_run
//! use ahref::{crawl, CrawlOptions, export::{self, Format}};
//!
//! # async fn example() -> ahref::Result<()> {
//! let graph = crawl("https://example.com", CrawlOptions {
//!     max_pages: 100,
//!     concurrency: 3,
//!     ..Default::default()
//! }).await?;
//! export::write(&graph, Format::Json, std::io::stdout())?;
//! # Ok(())
//! # }
//! ```
//!
//! Offline analysis is available through [`Graph::analyze`]. HTTP failures become node
//! outcomes; invalid options, invalid graph input and output failures return [`Error`].
pub mod crawler;
pub mod error;
pub mod export;
pub mod graph;
pub mod parser;
pub mod urls;

pub use crawler::{crawl, CrawlOptions};
pub use error::{Error, Result};
pub use graph::{Graph, LinkEdge, LinkType, PageNode};
pub use parser::Parser;

/// Compatibility helper. HTML is serialized by the HTML5 parser.
pub fn get_a_tags(html: String) -> Vec<String> {
    Parser::new(html).parse_tags()
}

/// Compatibility helper returning raw href attributes in document order.
pub fn get_url_from_tags(html: String) -> Vec<String> {
    Parser::new(html).parse_links()
}
