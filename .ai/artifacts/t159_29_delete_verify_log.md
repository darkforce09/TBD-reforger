# T-159.29.1–.3 — React deletion phase — verify log

**Operator go:** 2026-07-17 ("delete the react"). **Worktree:** `.ai/artifacts/worktrees/TBD-T-159`
· branch `t-159-leptos-ui`. **Executor:** claude-code (solo session). **Result: PASS —
`apps/website/frontend/` is deleted; the Leptos SPA is the only frontend.**

Order was freeze → re-home → delete, so the oracle was captured while it still existed and every
step left the branch green.

## T-159.29.1 — V oracle freeze @ `48f43436` (tag T-159.29.1)

`gate_v_suite.mjs` (new): `freeze` captures the normalized DOM (gate_v's dom.js serializer, scope
`#root>:first-child`, viewport 1440×900, frozen clock + fixture-intercepted fetches, two-identical-
serializations stability loop) + a PNG for every leaf route from the built React dist into
`t159_gates/v/oracle-freeze/` (committed; .gitignore carve-out — **non-regenerable after the
delete**); `verify` re-captures from the Leptos dist and diffs against the goldens — the permanent
V regression gate, wired into `make leptos-gates`.

- **Coverage:** 25 of routes.csv's 26 rows. `/missions/:id/edit` excluded by design (editor DOM =
  one canvas + docked shell; its gate is the 15 CDP smokes). React's always-mounted empty sonner
  toaster portal (2nd `#root` child) sits outside the scope.
- **First verify found 6 divergences; 5 were Leptos drift and were FIXED** (React-pinned goldens
  kept): missions/personnel `value=""` controlled-input attr; mortar default-value attrs;
  serverintel `url("…");` style quoting; eventmgr two-text-node month header + synthetic pad-cell
  ids removed. The 6th (content) is the intended T-159.25 mock→live rebuild — golden re-sourced
  from Leptos via the suite's `accept` mode, React reference kept at `content.react.dom.json`,
  note recorded in `manifest.json`.
- **Result: 25/25** (24 React-pinned byte-equal + 1 accepted). Oracle dist identity sha
  `2aa6a2f251e9…` in the manifest. React dist was current (built Jul 15 > last src change Jul 14).

## T-159.29.2 — parity-oracle re-home @ `9cc4364f` (tag T-159.29.2)

Disposition of the 24 wasm-importing vitest files (16 `_wasm/*.parity.test.ts` + 8 tactical-map):

- **TS-vs-wasm differential parity** (chunkMath, cluster-vs-Supercluster, demGrid/seaBand/contours,
  meters, interaction/orthoCamera-vs-deck, slotIndex-vs-rbush, doll, forest, hillshade, glyphLod,
  slotGpu, wasmDoc/ydoc, t152_4/t152_5): **retired with the TS side** — the oracle in those tests
  IS the deleted React/npm implementation; both sides now derive from the Rust crates, natively
  pinned (map-engine-core 233→**234** tests incl. real-Everon fixtures; camera ULP suites;
  doc/store; mission/compile) + the 15 CDP editor smokes on the live wasm build.
- **The one oracle-independent pin vitest held exclusively** — the full-island census — re-homed:
  `world::store::tests::full_island_census_matches_pinned_inventory` parses ALL 315 committed
  chunks and asserts **1623 prefabs / 1,216,109 instances / 315 chunks / 888 roads / 36 forest
  regions / 625 TBDD density grids** (decode-smoked). 1.76 s.

## T-159.29.3 — the deletion (this commit)

- **`git rm -r apps/website/frontend`** — 299 tracked files; node_modules/dist purged from disk.
  Also deleted: `scripts/website/verify-wgpu-gpu.mjs` + its make target (drove the React dev
  harness; superseded by `selfcheck_editor.mjs` + the smokes) and the driver's oracle-era
  `smoke.mjs`.
- **CI:** ci.yml `frontend` job (npm ci/format/lint/build/test + `make wasm`) deleted —
  `website-leptos` is the SPA job. contracts.yml `frontend-doc-lint` job deleted; `codegen-drift`
  diffs only `apps/website/src/contract/generated`.
- **Codegen:** `codegen.mjs` is Rust-only (TS projection + kit-aliases Vite copy removed;
  `json-schema-to-typescript` dropped from tbd-schema deps, lockfile −184 lines). Re-ran: zero
  drift in generated Rust.
- **Makefile:** `web`, `map-assets-link`, `wasm` (pkg build — no consumer; `wasm-ci` crate gates
  stay), `verify-wgpu-gpu` deleted; `build` = backend + `leptos-build`; `ci-local` swaps
  `ci-local-frontend` → **`ci-local-leptos`** (fmt + clippy wasm32 + cargo test + trunk release,
  mirroring ci.yml); **`leptos-gates` gains the frozen V-suite** after the editor smokes.
- **Gate scripts:** citations TS-6 no-ops via its existsSync guard (annotated; 0 exports checked);
  t090 gate 7 resolves `npm run`/make refs against a **frozen allowlist** of the retired FE
  scripts/targets (historical specs stay archival, gate stays strict for new refs) — re-ran: all
  12 gates OK. verify-file-length GENERATED_PREFIXES emptied. verify-height-labels wasm oracle
  skip message updated (math pinned by core cargo tests). verify-monorepo-migration V14 →
  leptos cargo check. Ticket-tool VERIFY hints → `make ci-local-leptos`.
- **Config:** `.gitignore` + `.editorconfig-checker.json` frontend entries removed;
  deploy-staging rsync exclude removed; leptos Cargo.toml note; t159_gates README frozen-era
  rewrite; CLAUDE.md operational sections corrected (frontend line, layout, run-locally,
  conventions, doc-table; historical §Status bullets left as history).

## Gates (all post-delete)

| Gate | Result |
|------|--------|
| `cargo check -p reforger-backend` (SQLX_OFFLINE) | clean |
| `cargo fmt -p website-leptos --check` · clippy wasm32 | clean · **12 = baseline** |
| `cargo test -p website-leptos` | **46 passed** |
| `cargo test -p map-engine-core --all-features` | **234 passed** |
| `trunk build --release` | ✅ |
| `make leptos-gates` (selfcheck + 15 smokes + frozen V-suite) | **all green · V 25/25** |
| `make schema-validate` + t090 spec gates | OK (12/12) |
| schema codegen re-run | zero drift |
| Makefile parse (`make -n`) | ok |

## Pre-existing reds discovered (NOT introduced here; unchanged by the delete)

- `verify-contract-citations`: 1 dangling citation — `TBD_LoadoutEquipComponent.c` cites
  `loadout-export.schema.json#/properties/gear`, stale since the T-068.10.4 v2 schema restructure.
  Mod scripts are executor-gated (`workbench`), so not fixed here; belongs to the T-068 lane.
- `./scripts/ticket check`: registry rot (T-149 missing fields, T-154 order, T-161 spec on main).

## Residual references (deliberate)

- `scripts/map-assets/verify-t152-cartographic.mjs` loads the old React wasm pkg path — a manual
  T-152-lane tool (that program runs in its own worktree); revisit at merge.
- `scripts/lib/ticket_registry.py` T-091-era brief templates quote old FE commands — archival
  strings for closed tickets.
- CLAUDE.md §Status history + old specs/verify-logs cite `apps/website/frontend` — history, kept.

## Still HELD for operator (the flip — NOT done)

Prod `SPA_DIST_DIR` + `FRONTEND_URL`/`ALLOWED_ORIGINS`/Discord-redirect at the API origin +
staging soak + real 142 MB save + rollback plan (git: the pre-delete tree is `T-159.29.2`; any
React dist rollback now comes from git history, not a build).
