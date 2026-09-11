# Releasing ahref

Version 1.0.0 is prepared in this tree. Publication is a separate maintainer action.

1. Review the branch and its atomic commits, and confirm the CI matrix passes on Linux,
   macOS and Windows. Keep the manifest version, default user agent and changelog aligned.
2. Run the full [testing gate](testing.md), then `cargo package --locked` from a clean
   checkout. Inspect `cargo package --list`; fixtures and library sources must be included.
3. Change the changelog heading from Unreleased to the actual release date, commit it,
   and push the reviewed branch through the usual PR process.
4. After merge, create and push the annotated `v1.0.0` tag. The release workflow verifies
   that the tag matches Cargo metadata, reruns checks and produces a Linux x86_64 archive,
   SHA256 checksum and verified `.crate` artifact. Artifacts appear on the Actions run.
5. Download and inspect the artifacts, then create a GitHub Release and attach them.
   The workflow intentionally needs no write token and does not publish automatically.
6. If the maintainer owns the crates.io package, run `cargo publish --dry-run --locked`
   and then `cargo publish --locked` using their own credentials. Do not put registry tokens
   in repository files. Verify `cargo install ahref --version 1.0.0 --locked` afterward.

Release archives are currently built for Linux x86_64; other platforms can build from
source and are covered by CI. Do not claim a platform passed until its CI job finishes.

## Compatibility

This release replaces the old positional HTML CLI with subcommands and raises the minimum
Rust version to 1.90. Existing parsing helper names remain, but returned tags are HTML5
serialized. JSON schema versions are independent of crate versions; breaking interchange
changes require a new schema version and migration documentation.
