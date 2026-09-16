# Phase 3A remaining work — four briefs, eight agents

Phase 3A pulls the editor's engine logic out of `apps/website/frontend/src/editor/` and into
`apps/website/map-engine/src/editing/` and `data/store/operations/`. A first agent landed eight
commits and was stopped partway; the work left over is split here.

## Naming

These briefs sit **inside phase 3A**. They are named `3A-A` … `3A-D` so they can never be
misread as phases 3A/3B/3C/3D. Each brief is further split into agent-sized units
(`3A-B1`, `3A-B2`, …) because a single subagent cannot carry a whole brief — the first 3A agent
was stopped for exactly that. Commit-subject suffix is `(3A)` throughout.

## Briefs

| brief | scope | rough size |
|---|---|---|
| [3A-A](3A-A_restore_dropped_tests.md) | Restore the dropped test coverage | small |
| [3A-B](3A-B_persist_split.md) | `state/{persist,hydrate}.rs` → `editing/persist/` | 2,878 LOC |
| [3A-C](3A-C_operations_move_engine_side.md) | Move the additive + engine-only operations into `data/store/operations/` | 1,721 LOC |
| [3A-D](3A-D_operations_delete_adapters.md) | Delete the 13 adapters, unwind the façade, repoint call sites | 2,096 LOC, 338 call sites |

## Agent order

`3A-A` is independent and fixes a regression — run it first. `3A-C` must precede `3A-D`: the
adapters cannot be deleted until what they adapt lives engine-side. Within `3A-D` the façade
unwind is last.

| # | agent | scope | LOC |
|--:|---|---|--:|
| 1 | `3A-A` | Restore the one genuinely dropped test; repair the two engine scrubs; sweep dead test-name citations | small |
| 2 | `3A-B1` | `state/persist.rs` → `editing/persist/`; IndexedDB transport stays frontend | 1,719 |
| 3 | `3A-B2` | `state/hydrate.rs` → `editing/persist/`; authed HTTP GET stays frontend | 1,159 |
| 4 | `3A-C1` | 7 engine-only `entity/` impls → `data/store/operations/` (additive) | 1,026 |
| 5 | `3A-C2` | 3 additive host-state files → `data/store/operations/` (additive) | 695 |
| 6 | `3A-D1` | Delete the 5 flat adapters, repoint call sites | 644 |
| 7 | `3A-D2` | Delete the 8 `entity/` adapters, repoint call sites | 1,354 |
| 8 | `3A-D3` | Unwind the 98-LOC façade, repoint the rest, delete `state/operations/` | 98 + tail |

One agent per row, run one at a time, never in parallel. Each row's gate must be green before
the next launches.

Every agent reads [`00_rules_every_agent_obeys.md`](00_rules_every_agent_obeys.md) first.
[`01_original_full_3a_brief.md`](01_original_full_3a_brief.md) holds the complete per-file audit
the briefs draw on.

## `state/operations/` reconciles exactly

4,320 LOC in the directory plus the 98-LOC façade = 4,418:

| bucket | LOC | fate |
|---|--:|---|
| 7 engine-only impls | 1,026 | → engine (`3A-C1`) |
| 3 additive host-state files | 695 | → engine (`3A-C2`) |
| 5 flat adapters | 644 | deleted (`3A-D1`) |
| 8 `entity/` adapters | 1,354 | deleted (`3A-D2`) |
| façade | 98 | deleted (`3A-D3`) |
| `batch.rs` + `context/` | 601 | **stays frontend** — phase 3B moves it |

## State these briefs assume

Commit `bf7c366d7`. All green:

```
cargo xtask verify engine-layers                  ENGINE-LAYERS: PASS (all 8 rules)
  rule 5 — 0 site(s) across 60 .rs file(s) under editing/
  rule 6 — 0 import(s) across 377 .rs file(s) and the manifest
cargo test -p website-map-engine --all-features    1290 passed; 0 failed; 2 ignored
cargo test -p website-frontend                     1316 passed; 0 failed
```

Already landed and not to be revisited: `editing/picking.rs`, `editing/tools/*`,
`editing/commands/*`, `editing/history/*`, `editing/host.rs`, `editing/batch.rs`, and gate
rules 5 and 6.

## Known red before any of this starts — not yours, not a regression

```
cargo xtask verify file-length   exit 1, 9 unallowlisted SIZE-3   (phase 3C clears these)
cargo test -p xtask              8 flaky failures, all pinning a missing apps/mod/** script
```

The eight names are listed in [`00_rules_every_agent_obeys.md`](00_rules_every_agent_obeys.md).
Counts drift run to run (818/10, then 820/8) — judge by name set, never by count.
Full detail: [`../../../docs/platform/engine_split_phase3_baseline.md`](../../../docs/platform/engine_split_phase3_baseline.md).
