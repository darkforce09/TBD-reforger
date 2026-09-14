# `src/v2` migration log

The frontend is being reorganised, folder by folder, into the domain-driven tree described in
[`README.md`](README.md). Each phase moves a slice of the legacy tree into `src/v2`, decomposes the
files it touches to the size limits, evacuates inline tests into sibling `tests/` folders, and
rewrites the documentation from the code as it stands. **No behaviour changes.**

## Rules every phase follows

- **Move, never copy.** A file exists in exactly one place at any moment; when a legacy folder
  empties, it and its `mod.rs` are deleted.
- **Size limits.** Production files stay under 500 lines, test files under 1000. No allowlist rows
  are ever added.
- **No inline test modules.** A production file declares its tests with
  `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
- **Fresh documentation.** Legacy comments are input for understanding only; every `//!` header and
  `///` block in `src/v2` is written from the code as it is today.
- **Gates green after every work item:** native `cargo check`, the `wasm32-unknown-unknown` check,
  `cargo test -p website-frontend --bin website-frontend`, and `cargo fmt`.

## Test-count floor

The native suite counted **1449 passed; 0 failed; 0 ignored** before the first move. Every later run
must be at or above that, with nothing newly failing or ignored.

## Phases

| Phase | commit | tests before→after | notes |
|-------|--------|--------------------|-------|
| Phase 1 | `4f67010bd` | 1449 → 1450 | `core/` retired: `api/`, `auth/`, `ui/`, `utils/`, `test_support/`. The OAuth callback page moved to `pages/account/`. One new test: the v2 documentation audit. |
| Phase 1 fix-up | follows `7a2abf8ed` | 1450 → 1450 | A stray `\(\s*\)` pass late in Phase 1 had emptied parentheses inside string literals; the compiler could not see those. Restored 26 weakened guard-test needles (SSE, client and scrubber batteries), rewrote six comment lines left with stripped-reference residue, and gated two wasm-only imports the split had exposed to the native build. |
