# ahref

ahref is a Rust CLI crawler that turns a website into a directed link graph.

![Website pages connected into a directed link graph](docs/assets/hero.png)

[![Rust checks](https://github.com/tenqz/ahref/actions/workflows/rust.yml/badge.svg)](https://github.com/tenqz/ahref/actions/workflows/rust.yml)
[![MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

Crawl internal HTML pages, preserve link anchors, and inspect site structure through degree, depth and internal PageRank. Export a versioned graph for your own tools, Gephi or Graphviz. Runs locally with no account or API key.

## Install

Build this version from the checkout with Rust 1.90 or newer:

```bash
git clone https://github.com/tenqz/ahref.git
cd ahref
cargo install --path . --locked
ahref --version
```

After version 1.0.0 is published to crates.io, install it with `cargo install ahref --version 1.0.0 --locked`. Earlier published versions are HTML parsing helpers and do not provide this crawler. A local release build is not a published release.

## Crawl a website

```bash
ahref crawl https://example.com
ahref crawl https://example.com --max-pages 1000 --max-depth 10 --concurrency 5
ahref crawl https://example.com/sitemap.xml --output graph.json
ahref crawl https://example.com --format graphml --output site.graphml
```

A bare hostname uses HTTPS. A sitemap index or `.xml.gz` sitemap is also accepted. Sitemap URLs and the origin homepage enter the initial queue together and are fetched with bounded concurrency. Links found in their HTML can discover additional pages outside the sitemap. `--max-pages` limits the whole crawl; `--max-depth` limits link following from each seed. Being a seed does not imply depth zero: pages known only from sitemap have `depth: null` until reached through links from the homepage.

Use this graph for internal linking analysis, orphan candidate detection, site architecture visualization, internal PageRank, broken link discovery and further graph analysis.

## Work offline

```bash
ahref analyze graph.json
ahref analyze graph.json --damping 0.85 --output analyzed.json
ahref export graph.json --format graphml --output site.graphml
ahref export graph.json --format dot --output site.dot
dot -Tsvg site.dot -o site.svg
```

GraphML opens in Gephi. `--output -` writes a graph to stdout; an explicit `--format` without a path does the same. Human-readable statistics then go to stderr, so JSON can be piped safely. Without output/format, crawl prints only the compact report.

## Options

| Option | Default | Meaning |
| --- | --- | --- |
| `--max-pages` | `1000` | Page request budget, including redirects and HTTP errors |
| `--max-depth` | `10` | Hyperlink hops from each crawl seed |
| `--concurrency` | `5` | Maximum in-flight page requests |
| `--timeout` | `20` | Per-request timeout in seconds, including body |
| `--user-agent` | `ahref/1.0.0` | HTTP user agent and robots product token |
| `--include-subdomains` | off | Include descendants of the starting hostname |
| `--respect-robots` | `true` | Respect robots rules; accepts explicit `false` |
| `--ignore-robots` | off | Explicit alternative opt-out |
| `--max-body-bytes` | `5242880` | Decoded response limit, including decompressed sitemaps |
| `--max-sitemaps` | `100` | Sitemap document/redirect budget |
| `--max-sitemap-urls` | `100000` | Maximum independently known sitemap pages |
| `--damping` | `0.85` | Internal PageRank damping in `[0, 1)` |
| `--format` | `json` when saving | `json`, `graphml`, `dot` |
| `--output` | none | Output path; `-` means stdout |

Start with low concurrency on small servers. There are no aggressive retries. Robots is cached per origin: unavailable/5xx/429 rules block that origin with a warning; other 4xx responses mean no rules. Robots matching uses Allow/Disallow and wildcard rules. The nonstandard `Crawl-delay` directive is not implemented.

## Read the results correctly

- Nodes are unique normalized HTTP(S) URLs; edges are individual `<a href>` occurrences with text and `rel`. Repeated links remain visible. Known assets and external links are recorded but not fetched.
- Degree counts distinct neighbors. Link counts include repeated occurrences. PageRank uses unique internal page neighbors, includes `nofollow` hyperlinks, redistributes dangling mass uniformly and sums to one.
- URL fragments, default ports, hostname case, dot segments and unreserved percent escapes are normalized. Query order/values, path case, trailing slash and HTTP/HTTPS remain distinct because servers may treat them differently.
- Redirects are separate node metadata, not fabricated hyperlinks. They cost zero for depth; PageRank uses actual hyperlinks only. Every redirect destination must pass scope and robots checks before fetching.
- Broken links have an observed HTTP status of 400 or greater, directly or through a checked redirect chain. External links, timeout failures and pending pages are **not assumed broken**.
- Orphans are sitemap URLs with no incoming hyperlink from another internal URL in the observed graph. This is a candidate finding, especially on a truncated crawl. Weakly-linked pages have one such distinct incoming source. A self-link does not disqualify an orphan.
- Dead ends are successfully fetched HTML pages without outgoing internal hyperlinks. Unfetched/blocked/error pages are not dead ends. Depth averages exclude unreachable pages.

JavaScript is not executed. Unknown resources are skipped by Content-Type; only `text/html` and `application/xhtml+xml` are parsed. Missing Content-Type is conservatively skipped. HTTP charset declarations are decoded; HTML-only meta charset detection is not implemented. No external status checking or automatic sitemap discovery is performed. The scope is the input hostname (plus optional subdomains), so `www` and apex are not automatically merged.

See the [schema reference](docs/schema.md) for fields and guarantees, and [architecture](docs/architecture.md) for the implementation. This release focuses on link graphs; it includes no AI, GSC, history or SEO recommendations.

## Rust library

```rust,no_run
use ahref::{crawl, CrawlOptions, export::{self, Format}};

#[tokio::main]
async fn main() -> ahref::Result<()> {
    let graph = crawl("https://example.com", CrawlOptions::default()).await?;
    export::write(&graph, Format::Json, std::io::stdout())?;
    Ok(())
}
```

`Graph::analyze()` recomputes metrics without network access. The old `Parser`, `get_a_tags` and `get_url_from_tags` helpers remain available; tag serialization now follows HTML5 conventions instead of preserving original quote formatting. The old positional HTML CLI is replaced by subcommands.

## Development

```bash
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
cargo doc --locked --no-deps
```

Tests serve a synthetic site on loopback with an ephemeral port. They need no public websites, credentials or services. See [testing](docs/testing.md), [contributing](CONTRIBUTING.md), [security](SECURITY.md), [release instructions](docs/releasing.md) and the [changelog](CHANGELOG.md).

Licensed under [MIT](LICENSE). Maintainer: [Oleg Patsay](https://opatsay.com/).
