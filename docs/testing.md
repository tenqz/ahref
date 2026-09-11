# Testing

Run the complete local gate with Rust 1.90+:

```bash
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps
```

Unit tests cover URL identity, parser behavior, sitemap namespaces/entities, and PageRank
with cycles/dangling nodes. Library documentation includes a compiled usage example.

Integration tests start a Tokio TCP server on `127.0.0.1:0`. They use
`tests/fixtures/index.html` and explicit synthetic routes. Assertions cover relative/query
links, duplicate anchors, cycles, redirects, HTTP 404, external scope, robots denial and
Allow overrides, unknown binary content, sitemap orphan candidates, root depth, page and
depth limits, timeouts, body limits, deterministic output and concurrent request ceilings.
JSON round trips and XML parsing exercise exports, including escaping and Unicode.

Tests use reserved external domains and assert that external nodes remain unrequested.
They do not depend on public servers or DNS availability. The installed CLI is exercised
separately from library tests so argument validation and stdout/stderr behavior remain
part of the contract.

For a bug fix, first add a fixture/assertion that fails for the reported behavior. Keep
the fix and regression test in one commit. Do not assert response timing or incidental
internal implementation details when an observable graph property can be asserted.
