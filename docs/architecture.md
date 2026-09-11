# Architecture

The library owns crawling and analysis; the CLI only parses arguments, reads/writes files
and presents a summary. No module depends on a concrete graph-library type.

```text
src/
  main.rs                 CLI commands and human output
  lib.rs                  Public API and tested usage example
  error.rs                Typed operation errors
  urls.rs                 HTTP URL normalization, resolution and scope
  parser.rs               HTML5 parsing, anchors and compatibility helpers
  crawler/
    options.rs            Budgets and validation
    fetcher.rs            Async bounded HTTP bodies, charset and gzip handling
    robots.rs             Per-origin robots cache and rule matching
    sitemap.rs            XML urlset/index parsing
    discovery.rs          Bounded traversal of sitemap documents and indexes
    mod.rs                Deterministic queue orchestration and result merging
  graph/
    model.rs              Versioned serializable domain records
    metrics.rs            Validation, degree, shortest depth and findings
    pagerank.rs           Sparse internal power iteration
  export.rs               JSON, GraphML and DOT serialization
```

## Scheduling and memory

The queue stores normalized URL identities and crawl depth. Shortest-depth waves are
processed in sorted URL order. Page fetching runs through a bounded async stream; results
are parsed immediately and merged in sorted order. HTML bodies are dropped after parsing.
The graph stores link occurrences plus per-node records, not page bodies. Memory therefore
scales with graph size plus at most the configured concurrent body budget.

There is one page request per URL identity. Errors and redirect hops consume the same
global page budget. Robots/sitemaps have separate bounded document/body limits. Known
resources and external destinations are never scheduled. Redirects are fetched manually
so a redirect cannot silently escape scope or bypass robots. Redirect chains stop after
ten followed hops, and visited identities stop cycles.

The homepage and all accepted sitemap URLs are queued together at seed depth zero.
They share the initial bounded fetch wave and the global page request budget; discovered
hyperlinks can extend the queue beyond the sitemap. Duplicate URLs still receive one
request. Queue depth measures distance from the nearest seed for crawl-budget purposes;
it is not presented as root depth. Final root depth is recomputed with a 0–1 breadth-first
traversal of hyperlinks/redirects, so a sitemap-only orphan remains unreachable.

## Analysis choices

The graph preserves duplicate hyperlinks; degree and PageRank use unique-neighbor sets.
Redirect metadata contributes to reachability and checked broken destinations, but not
to the hyperlink adjacency used for PageRank. External and resource nodes remain exportable
without being treated as internal pages. This separation keeps the stored observations
usable for future analyses with different weighting or redirect policies.

PageRank uses a sparse adjacency list with O(V + E) work per iteration. Sorting and
balanced maps add logarithmic costs. There are no dense V-by-V matrices. The code favors
explicit data and bounded work over premature parallel graph optimization.

## Failure boundaries

Invalid options/input and export errors fail the operation. Ordinary HTTP/body errors
become node state/error fields. A failed sitemap becomes a warning while the homepage
can still be crawled. Unavailable robots conservatively blocks its origin. A successful
CLI exit means the report was produced, not that every URL was fetched successfully;
automation should inspect summary states and warnings.
