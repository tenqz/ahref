# Graph schema 1.0

The JSON interchange format contains `schema_version`, `site`, `nodes` and `edges`.
The schema major version changes when existing fields or their meanings become incompatible.
Consumers should tolerate additive fields. `Graph::validate` rejects unknown schema versions,
duplicate/non-normalized URLs, incorrect membership and missing edge/redirect endpoints.

## Top level

| Field | Type | Meaning |
| --- | --- | --- |
| `schema_version` | string | Exactly `1.0` |
| `site.root` | URL | Start page, or origin homepage for sitemap input |
| `site.include_subdomains` | boolean | Hostname membership rule |
| `site.damping` | number | PageRank parameter, `0 <= d < 1` |
| `site.summary` | object | Recomputed aggregate metrics |
| `site.warnings` | string array | Limits, sitemap failures and other incomplete observations |
| `nodes` | object array | Unique URL records, including resources and external targets |
| `edges` | object array | Individual hyperlinks, including repeated identical links |

## Nodes

| Field | Type | Meaning |
| --- | --- | --- |
| `url` | string | Absolute normalized HTTP(S) URL without fragment or credentials |
| `kind` | string | `internal` or `external` |
| `status_code` | integer or null | Observed HTTP status, never an invented status for a timeout |
| `depth` | integer or null | Shortest root hyperlink distance; redirect hops cost zero |
| `state` | string | Fetch outcome from the table below |
| `is_html` | boolean or null | Successful response content classification; null if unknown |
| `sitemap` | boolean | Independently known through sitemap |
| `redirect_target` | string or null | Immediate resolved HTTP(S) Location destination |
| `error` | string or null | Human-readable failure detail, not a stable machine error code |
| `in_degree` / `out_degree` | integer | Distinct incoming/outgoing URLs, including self-links |
| `internal_incoming_links` | integer | Incoming occurrences from internal sources to an internal target |
| `internal_outgoing_links` | integer | Internal outgoing occurrences |
| `external_outgoing_links` | integer | External outgoing occurrences |
| `pagerank` | number or null | Internal page rank, or null for external/resource nodes |

| State | Meaning |
| --- | --- |
| `pending` | Discovered but unrequested: scope, depth, page budget or redirect limit |
| `fetched` | Received a response, including HTTP error responses |
| `redirect` | Received a recognized redirect and resolved its Location |
| `blocked_robots` | Excluded by robots rules or unavailable robots |
| `skipped_resource` | Known resource extension or successful non-HTML response |
| `failed` | Network, decoding/decompression or response-size failure |

`pages` counts internal candidates excluding known/confirmed resources. Thus it may exceed
the request budget. PageRank includes pending, blocked, redirect and error page candidates,
and is therefore an estimate on the observed graph. It excludes assets/external targets.
Repeated links to a neighbor do not multiply its weight. `rel=nofollow` is metadata, not a
PageRank filter. Dangling pages redistribute their mass uniformly. Iteration stops at L1
delta below `1e-12` or 1000 iterations; `pagerank_converged` reports whether it converged.

## Edges

Each edge has `from`, `to`, `anchor`, `kind` and `rel`. Endpoints refer to node URLs;
`kind` describes the target's membership. Anchor text is decoded HTML text with normalized
whitespace; `rel` is a lowercase token array. Source is the actual fetched page URL.
`<base href>` affects relative resolution. Anchors-only and non-HTTP(S) links are omitted.
Resource hyperlinks remain edges even though their targets are not crawled.

## Summary

`pages`, `fetched_pages`, `pending_pages`, `failed_pages`, `blocked_pages` count internal
page candidates by the definitions above. `edges`, `internal_links`, `external_links`
count occurrences. `external_domains` counts distinct external hostnames, not registrable
domains. `broken_urls` counts observed HTTP error URLs; `broken_links` also follows redirect
chains to an observed error. Redirect loops and untested destinations are not classified
as broken HTTP responses. `redirects` counts nodes with a resolved redirect target.

`orphan_pages` is the number of sitemap candidates other than the root without an incoming
hyperlink from another internal URL. `weakly_linked_pages` has exactly one such source.
Redirects do not fabricate incoming hyperlinks. `dead_end_pages` only counts successful
HTML responses without internal outgoing links. `max_depth` and `average_depth` use
reachable page candidates; `unreachable_pages` counts candidates with null depth.

The crawler processes shortest-depth waves in URL order; output nodes/edges are sorted.
With identical responses and options, successful graph output does not depend on response
completion order. Live websites, timeouts and error strings can still vary between runs.
No crawl timestamps or historical snapshots are part of this schema.

## Other formats

GraphML preserves URL, membership, status, depth, rank, state, degree, redirect target,
anchor and rel. DOT is a visualization export containing labels, rank and rel. Both retain
parallel hyperlink edges. JSON is the lossless format for round trips; `analyze` and
`export` consume JSON only. XML-invalid control characters are stripped from GraphML text.
