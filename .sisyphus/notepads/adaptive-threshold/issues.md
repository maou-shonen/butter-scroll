# Issues

## [2026-03-25] Session Start

### Pre-existing LSP error (NOT introduced by us)
- `src/config.rs:121` — lifetime error: "method was supposed to return data with lifetime `'2` but it is returning data with lifetime `'1`"
- This exists before any of our changes. Must NOT break further; verify `cargo test` still passes.
