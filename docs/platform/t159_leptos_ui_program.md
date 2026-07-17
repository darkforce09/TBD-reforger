# T-159 — Leptos UI rewrite program

**Status:** program hub · **MODE (2026-07-17): single-session solo finish** — the Fable 5 audit +
finish plan was operator-approved; Claude Code executes code + verify logs + docs + commits directly
(no per-slice Cursor Mode B pass). Plan of record: `~/.claude/plans/you-are-fable-5-vast-bird.md`
(operator copy) — stream ladder T-159.24 → T-159.29 below.
**ACTIVE:** **T-159.26** (Mission Creator editor completion) · **Latest:** **T-159.25** · **Worktree:**
`.ai/artifacts/worktrees/TBD-T-159/` @ `t-159-leptos-ui`

## Finish-program streams (audit-derived, 2026-07-17)

| Stream | Scope | Status |
|--------|-------|--------|
| **T-159.24** prep | Trunk `/api` + `/map-assets` proxies · `make leptos*` · `api_put/patch/delete/post_ok` client verbs · **140 MB upload spike PASS** (940 ms via Trunk proxy → 404 after full read; no direct-:8080 bypass needed) | **shipped** |
| **T-159.25** suite live-wire | toasts · suite mutations (Settings/ORBAT/Missions/Approvals/Personnel/EventMgr/Mortar/Content, live dev-login proofs) · SSE telemetry · Server Intel + Operations Calendar populated · CreateMissionDialog · live `smoke_mutations` gate | **shipped** (`.25a`–`.25e`) |
| **T-159.26** editor completion | **.23 Attributes** (spec `t159_23_attributes_modal.md`, tags separately) · server-hydrate/conflict/dirty (data-safety) · Mission Settings · ORBAT dock · strip parity · Ctrl+C/V/Delete/Space · VirtualOutliner + tree ops + 367k scale smoke · `run_all.mjs` aggregate runner | queued |
| **T-159.27** Arsenal + registry | rules/doll/itemDetail/migrate native-tested + vitest goldens as `cargo test` · registry compat · ArsenalTab UI + canvas2d doll + Faction Manager | queued |
| **T-159.28** map-asset host | `world_assets`: TBDS basemap · DEM PNG (CUR Z) · `.tbd-sat` · world-chunk residency streaming · CI 2-chunk fixture · GPU-readback gates. **Operator visual checkpoint after.** | queued |
| **T-159.29** cutover build-out | backend ServeDir SPA + COOP/COEP + `/map-assets` · CI website-leptos job · env/OAuth flip docs · oracle freeze. **Default flip + React deletion = operator go only.** | queued |

## Progress (latest first)

| Milestone | Status |
|-----------|--------|
| **T-159.24** prep: proxies + make targets + client verbs + 140 MB spike | tag **T-159.24** |
| **T-159.22.1** Undo step-boundary gate (driver fix; core OK) | `ce73c5bc` |
| **T-159.22** Outliner + Asset palette | `0154b4e9` |
| **T-159.23** Attributes modal | folded into **T-159.26** (spec unchanged, tags separately) |

### Verify logs (recent)

- [`.ai/artifacts/t159_24_verify_log.md`](../../.ai/artifacts/t159_24_verify_log.md) — prep gates
  11/11 smokes ×2 (baseline + post-change), stash-diff zero new warnings, spike table
- [`.ai/artifacts/t159_22_1_verify_log.md`](../../.ai/artifacts/t159_22_1_verify_log.md) — CDP
  double-fire; core invariant held
- [`.ai/artifacts/t159_22_verify_log.md`](../../.ai/artifacts/t159_22_verify_log.md) — docks;
  **§defect conclusion superseded** by .22.1

### Process (supersedes the per-slice Mode B pace note)

Doc sync happens **in the same session as the code**, once per stream. Gate discipline unchanged:
cargo check wasm32 + clippy stash-diff (zero new lints) + `trunk build --release` + full editor
smoke suite before every stream commit; live-backend proofs via dev-login. Stop-doing list from the
audit: per-feature Mode B round-trips, per-slice handoff files, new port-pair per smoke, ε=0 DOM
goldens for populated data states, operator sign-off between micro-slices, re-litigating the undo
core (closed by .22.1).

## Slice index

| Slice | Status |
|-------|--------|
| **T-159.22** | shipped `0154b4e9` |
| **T-159.22.1** | shipped `ce73c5bc` |
| **T-159.24** | **shipped** — `t159_24_verify_log.md` |
| **T-159.25** | **shipped** — `t159_25_verify_log.md` (`.25a`–`.25e`) |
| **T-159.26** | **ACTIVE** — editor completion (Attributes rides here) |
| **T-159.27** … **T-159.29** | queued |
