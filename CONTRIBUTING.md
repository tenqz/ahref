# Contributing

Keep ahref focused on turning sites into link graphs. AI, search-console integrations,
content analysis, history and recommendation systems belong in separate layers.

Use Rust 1.90+ and run the checks in [testing](docs/testing.md). Keep public APIs documented,
explain non-obvious decisions in comments, and add unit/integration tests for new behavior.
Tests must run against local fixtures without internet access or credentials.

## Atomic commits

Follow the project's ACDD practice:

- One commit is one complete, verified result. Prefer one file per commit when it can
  deliver that result independently.
- Keep required companion files together: a bug fix and regression test, a module and
  its wiring, or dependency changes and lockfile. Explain the dependency in the commit body.
- Every commit should build and pass the checks available at that point.
- Separate unrelated changes, even if they affect the same file.
- Describe the intended step before editing, verify it, then commit it before the next step.
- Use Conventional Commits: `feat(crawler): ...`, `fix(url): ...`, `test: ...`, `docs: ...`.
- Review the sequence before merging. Prefer focused pull requests.

Branch prefixes: `feat/`, `fix/`, `docs/`, `ci/`. Describe the observable behavior changed
and the checks run in each PR. Update README/schema documentation when behavior changes.
Do not commit crawl output containing private URLs, credentials, `.env` or build artifacts.

Report ordinary bugs through GitHub issues with a minimal HTML/sitemap example, command,
version and expected/actual result. Report security issues privately as described in
[SECURITY.md](SECURITY.md). Participation follows [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
