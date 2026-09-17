# Phase 3C T: close the ratchet

Read `00_rules_every_agent_obeys.md` and the user-pasted Phase 3C plan. Once B–S are integrated, confirm `apps/website/frontend/src/v2/tests/doc_audit/allowlist.rs::ROWS` is empty, `.coding-standards-allowlist.yaml` has no frontend or orphan rows, and all production/test files meet the new thresholds.

Move `apps/website/audit.md` to `docs/specs/website_reorg/` and repoint its four named citations. Run the full acceptance commands from the plan, serially, with `CARGO_TARGET_DIR=target-container`. Run the Chrome gates with Postgres and the development API available. Compare the v-suite route result set to `docs/platform/engine_split_phase3_baseline.md`; no new failing route is allowed. Record command output and any baseline-only failures by exact test name. Stage only authored files and commit on `main` with the `(3C)` subject suffix.
