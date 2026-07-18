# T-159 — Leptos UI rewrite program

> **T-171 note:** paths in this hub that say `apps/website-leptos/` or treat `apps/website/frontend/` as the deleted React tree **pre-date the T-171 nest**. Live SPA = `apps/website/frontend/` (pkg `website-frontend`). See [`WHERE_DOES_X_GO.md`](WHERE_DOES_X_GO.md).

**Status:** program hub · **MODE (2026-07-17): single-session solo finish** — the Fable 5 audit +
finish plan was operator-approved; Claude Code executes code + verify logs + docs + commits directly
(no per-slice Cursor Mode B pass). Plan of record: `~/.claude/plans/you-are-fable-5-vast-bird.md`
(operator copy) — stream ladder T-159.24 → T-159.29 below.
**ACTIVE:** none — **program complete incl. the React deletion** (operator go 2026-07-17):
React `apps/website/frontend/` was deleted at T-159.29.3; Leptos was then at `apps/website-leptos/`
and nested to `apps/website/frontend/` at **T-171**. Deletion phase
`.29.1` (V oracle freeze, 25/25) → `.29.2` (census pin → cargo test) → `.29.3` (delete + CI/Make/
codegen purge) — [`t159_29_delete_verify_log.md`](../../.ai/artifacts/t159_29_delete_verify_log.md).
**Residual (operator):** the prod default flip only (`SPA_DIST_DIR` + OAuth origin + soak). ·
**Latest:** **T-159.29.3** · **Worktree (historical):** `.ai/artifacts/worktrees/TBD-T-159/`

## Finish-program streams (audit-derived, 2026-07-17)

| Stream | Scope | Status |
|--------|-------|--------|
| **T-159.24** prep | Trunk `/api` + `/map-assets` proxies · `make leptos*` · `api_put/patch/delete/post_ok` client verbs · **140 MB upload spike PASS** (940 ms via Trunk proxy → 404 after full read; no direct-:8080 bypass needed) | **shipped** |
| **T-159.25** suite live-wire | toasts · suite mutations (Settings/ORBAT/Missions/Approvals/Personnel/EventMgr/Mortar/Content, live dev-login proofs) · SSE telemetry · Server Intel + Operations Calendar populated · CreateMissionDialog · live `smoke_mutations` gate | **shipped** (`.25a`–`.25e`) |
| **T-159.26** editor completion | **.26a** Attributes (tag T-159.23) · **.26b** server-hydrate/conflict/dirty (data-safety, live gate) · **.26c** keyboard (Del/Space/Ctrl+C/V) + Mission Settings (environment). ORBAT squad tree (needs T-071 squad creation) + VirtualOutliner @367k folded forward | **shipped** (.26a–.26c) |
| **T-159.27** Arsenal | ArsenalTab (12 kind-rows = React `LOADOUT_ROWS`) → canonical `SlotLoadoutV2` via `editor_ops::set_loadout`; faithful `picksToLoadout`/`loadoutToPicks` incl. `summary` + optic/magazine sticky pass-through (regression-guarded). **+ native-compile fix** (`MissionEnv` → `dto.rs`; CI `cargo test` red → green). Smart Forge (compat edge rows/paper-doll/weight/Faction Manager) folded forward | **shipped** |
| **T-159.28** map-asset host | `world_assets` hillshade MVP: manifest → DEM PNG → Rust `dem::` decode+hillshade → `tex_layer_*` (role 1). Satellite (`.tbd-sat`) + world-chunk streaming folded forward | **shipped** |
| **T-159.29** cutover build-out | backend ServeDir SPA + COOP/COEP + `/map-assets` · CI website-leptos job · env/OAuth flip docs · oracle freeze. Deletion executed at **.29.1–.3** (operator go); **prod default flip = operator only.** | **shipped** |

## Progress (latest first)

| Milestone | Status |
|-----------|--------|
| **T-159.29.3** React deletion + npm CI/Make/codegen purge | tag **T-159.29.3** · all gates green post-delete |
| **T-159.29.2** full-Everon census pin → cargo test | tag **T-159.29.2** · core 234 tests |
| **T-159.29.1** V oracle freeze (25 routes; 5 parity fixes; content accepted) | tag **T-159.29.1** · V-suite 25/25 |
| **T-159.27** Arsenal loadout tab (canonical `SlotLoadoutV2`) + native-compile fix | tag **T-159.27** · 15/15 smokes · 46 native tests |
| **T-159.29** cutover build-out (SPA serve + CI job + oracle freeze; flip/delete HELD) | tag **T-159.29** |
| **T-159.28** map-asset host (hillshade MVP) | tag **T-159.28** |
| **T-159.26** editor completion (.26a–.26c) | tag **T-159.26** |
| **T-159.25** suite live-wire (.25a–.25e) | tag **T-159.25** |
| **T-159.24** prep: proxies + make targets + client verbs + 140 MB spike | tag **T-159.24** |
| **T-159.22.1** Undo step-boundary gate (driver fix; core OK) | `ce73c5bc` |
| **T-159.22** Outliner + Asset palette | `0154b4e9` |
| **T-159.23** Attributes modal | folded into **T-159.26** (spec unchanged, tags separately) |

### Verify logs (recent)

- [`.ai/artifacts/t159_29_delete_verify_log.md`](../../.ai/artifacts/t159_29_delete_verify_log.md) —
  the React deletion phase (.29.1 freeze 25/25 · .29.2 census 234 tests · .29.3 delete + purge)
- [`.ai/artifacts/t159_27_verify_log.md`](../../.ai/artifacts/t159_27_verify_log.md) — Arsenal:
  15/15 editor smokes, 46 native tests, clippy 12 = baseline, wasm 7,153,883 B; native `cargo test`
  red → green (MissionEnv relocation)
- [`.ai/artifacts/t159_29_verify_log.md`](../../.ai/artifacts/t159_29_verify_log.md) — cutover
  build-out (SPA serve live table, CI job, oracle freeze; flip/delete HELD)
- [`.ai/artifacts/t159_28_verify_log.md`](../../.ai/artifacts/t159_28_verify_log.md) — map-asset
  host hillshade MVP
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
| **T-159.26** | **shipped** — editor completion (Attributes rides here; tag T-159.23) |
| **T-159.27** | **shipped** — `t159_27_verify_log.md` (Arsenal + native-compile fix) |
| **T-159.28** | **shipped** — `t159_28_verify_log.md` (map-asset host, hillshade MVP) |
| **T-159.29** | **shipped** — `t159_29_verify_log.md` (cutover build-out) |
| **T-159.29.1–.3** | **shipped** — `t159_29_delete_verify_log.md` (**React deleted**; only the prod flip stays operator-gated) |
