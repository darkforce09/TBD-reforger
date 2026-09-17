# Continuous Integration Tasks (`xtask/src/commands/ci`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Entrypoints for local and remote Continuous Integration test pipelines.

---

## Submodules

- **`ci_local.rs`**: Replays the full multi-lane CI check suite locally (`cargo xtask ci ci-local`).
- **`ci_frontend.rs`**: Frontend check suite: formatting, clippy wasm32, unit tests, Trunk release build (`cargo xtask mk ci-local-leptos`).
- **`tasks.rs`**: Task table mapping CI target names to ordered execution graphs.
