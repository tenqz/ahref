# Changelog

## 1.0.0 — Unreleased

- Replace the positional HTML CLI with `crawl`, `analyze` and `export` commands.
- Add bounded async HTML crawling, sitemap/index/gzip support and default robots handling.
- Preserve directed hyperlinks with anchor and rel metadata, HTTP outcomes and redirects.
- Add distinct-neighbor degree, root depth, orphan/weak-link candidates, dead ends,
  checked broken links and internal PageRank.
- Export schema 1.0 JSON, GraphML and DOT, with offline analysis and conversion.
- Replace regex parsing with HTML5 parsing; retain library compatibility helpers.
- Add deterministic fixture tests, CI, release instructions and community documents.

Breaking changes: CLI syntax, Rust 1.90 minimum, and HTML tag serialization formatting.
Existing 0.3 helper names remain available. Crates.io publication and a release tag are
separate maintainer actions; this entry is not evidence of a published release.
