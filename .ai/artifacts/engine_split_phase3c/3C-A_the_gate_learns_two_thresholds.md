# Phase 3C A: enforce the two file-length ceilings

Read `00_rules_every_agent_obeys.md` first. The Phase 3C plan pasted in the current task overrides its Phase 3B-specific instructions. Work on `main`; preserve all pre-existing dirty files and stage only files you edit. Do not run Cargo concurrently with another agent. Prefix every Cargo command with `CARGO_TARGET_DIR=target-container` and run from the repository root.

## Implementation

- In `xtask/src/node_free.rs`, make SIZE-3 fail production Rust files over 500 raw lines and test files over 1000 raw lines. A test file has a `/tests/` path component or an `_tests.rs` basename. Retire the SIZE-1 warning.
- Extend the walk to existing `apps/website/*/tests` roots and `apps/ticketboard/src`; the latter has eight violations and must be counted. Exclude only `apps/website/api/src/contract/generated/**` from SIZE-3 because schema-codegen owns it.
- Reject `MC-perf` expiry on SIZE-3 while retaining it for SIZE-2. Fail on orphan allowlist rows, including dead paths. Keep the walk fail-closed and non-vacuous.
- Update `.coding-standards-allowlist.yaml`: grandfather every existing non-frontend violation, with `2027-01-31` for api, tools, crates, ticketboard and `2027-06-30` for xtask. Remove generated, missing, and now-under-ceiling rows. Leave frontend violations unallowlisted.
- Add focused tests for threshold boundary, test classification, test-root coverage, generated exclusion, MC-perf distinction, and orphan detection. Existing fixture expiry dates in 2026-11-13 remain valid today but should not become brittle.
- Wire `cargo xtask verify file-length` into the GitHub CI language-gates job. Rewrite the normative threshold text in `docs/platform/CODING_STANDARDS.md`, the allowlist header, and the eight stale xtask module headers named in the Phase 3C plan.

## Evidence and stop condition

- Run `cargo test -p xtask` (record known baseline failures by name), `cargo xtask verify file-length`, `cargo test -p website-frontend`, and `cargo fmt --all -- --check`, serially, with the target-dir prefix.
- The new gate must report exactly 42 unallowlisted frontend SIZE-3 violations and no others. Report each verification result verbatim.
- Do not touch `apps/website/frontend/src/v2/tests/doc_audit/allowlist.rs`; the coordinator owns it.
- Tell the coordinator when editing is complete and list the exact files changed. Do not commit; the coordinator owns serialized Git operations.
