# Phase 3A remaining work — four scoped agent briefs

Phase 3A pulls the editor's engine logic out of `apps/website/frontend/src/editor/` and into
`apps/website/map-engine/src/editing/`. A first agent landed eight commits and was stopped
partway; the work left over is split here into four briefs small enough to run one at a time.

## Order

`A` is independent and fixes a regression — run it first. `C` must precede `D`: the adapters
cannot be deleted until what they adapt lives engine-side.

| brief | scope | rough size |
|---|---|---|
| [A](A_restore_dropped_tests.md) | Restore three tests the first agent deleted | small |
| [B](B_persist_split.md) | `state/{persist,hydrate}.rs` → `editing/persist/` | 2,878 LOC |
| [C](C_operations_move_engine_side.md) | Move the additive + engine-only operations into `data/store/operations/` | ~1,500 LOC |
| [D](D_operations_delete_adapters.md) | Delete the 13 adapters, unwind the façade, repoint call sites | ~2,800 LOC, ~500 call sites |

Every agent reads [`00_rules_every_agent_obeys.md`](00_rules_every_agent_obeys.md) first.
[`01_original_full_3a_brief.md`](01_original_full_3a_brief.md) holds the complete per-file audit
that A–D draw on.

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
cargo xtask verify file-length   exit 1, 9 unallowlisted SIZE-3   (Phase 3C clears these)
cargo test -p xtask              8 flaky failures, all pinning a missing apps/mod/** script
```

The eight names are listed in [`00_rules_every_agent_obeys.md`](00_rules_every_agent_obeys.md).
Counts drift run to run (818/10, then 820/8) — judge by name set, never by count.
Full detail: [`../../../docs/platform/engine_split_phase3_baseline.md`](../../../docs/platform/engine_split_phase3_baseline.md).
