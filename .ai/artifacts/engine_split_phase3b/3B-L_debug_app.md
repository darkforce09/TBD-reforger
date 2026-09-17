# 3B-L — `v2/apps/debug/`: the routed diagnostics workspace

Read `00_rules_every_agent_obeys.md` first. Every rule there applies to this brief.

`src/pages/debug/` is not residue of a v2 namesake — it is live, routed code that never migrated.
`router.rs` serves `/debug/building-viewer` and `/debug/world-los` from it. CLAUDE.md's atlas puts
it under `v2/apps/debug/` as the diagnostics testbench, where today there are only READMEs.

## The move

| from `src/pages/debug/` | to `src/v2/apps/debug/` |
|---|---|
| `building_viewer.rs` (2,717) | `building_viewer.rs` |
| `world_los.rs` (683) | `world_los.rs` |
| `world_los_scene.rs` (312) | `world_los_scene.rs` |
| `building_interior.rs` (541) | `building_interior.rs` |
| `building_interior_tests.rs` (471) | `tests/building_interior.rs` |
| `mod.rs` | `mod.rs` |

The test file moves into a directory literally named `tests`, which is what keeps it out of the
documentation audit, and its declaration moves with it — at the **bottom** of the file that
declares it, since `class_r_scrub::live_code()` blanks a file from its first `#[cfg(test)]` to EOF.

`v2/apps/debug/` currently holds three stale `README.md` files describing a subfolder shape that
does not match the atlas. Delete them; brief 3B-M writes the new set against the landed tree.
`v2/apps/mod.rs` gains `pub mod debug;`.

**Then delete the whole legacy `src/pages/` tree, and `mod pages;` from `main.rs`.** Once
`pages/debug/` leaves, that tree is two husks: `pages/mod.rs` declaring nothing but its two
children, and `pages/operations/mod.rs` declaring nothing at all — brief 3B-J emptied it when the
ORBAT and faction dialogs moved. At HEAD the only references to `crate::pages` in the entire crate
are `main.rs:9`, and `app_routes.rs`'s two `crate::pages::debug::…` view paths, which you are
repointing anyway. Verify that yourself before deleting — `rg 'crate::pages' apps/website/frontend/src`
filtered to exclude `crate::v2::pages` must come back with nothing but those three lines — then
delete the directory. A module husk left behind is exactly what Phase 3A deleted `state/picking/`
for, and leaving one here would strand the last `pages/` reference in a tree that has fully moved.

Nothing is split. `building_viewer.rs` (2,717) and `world_los.rs` (683) get dated allowlist rows
and are Phase 3C's subject.

## The three route artifacts move in lockstep

- `router.rs`'s `ROUTES` table: the **path strings and component names do not change** —
  `/debug/building-viewer` → `BuildingViewerPage`, `/debug/world-los` → `WorldLosPage`. Only where
  the component is declared changes.
- `app_routes.rs` mounts them by full path (`view=crate::pages::debug::building_viewer::
  BuildingViewerPage` and the world-LoS equivalent). Repoint both.
- `tools/tbd-tools/fixtures/t159/manifests/routes.csv` is diffed **byte-equal** against
  `router.rs`'s table. Since neither path nor component name changes, this file should need **no
  edit at all** — prove that rather than assume it: paste the two `/debug/` rows and the result of
  the gate's own comparison.

## Documentation debt

These four files land under `src/v2` for the first time, so the audit's rules 1 and 3 apply and are
not exemptible: a `//!` header in the house shape on each, and a doc comment on every `pub` item
they leave undocumented. Derive the list by running the audit.

## Pins

`building_interior_tests.rs` holds a cross-file pin into the map engine, anchored by brief 3B-A —
update its suffix if its subject moved (it has not) and its own path where something names it.
Grep the crate, `xtask/` and `tools/` for `pages/debug` and paste the empty result.

## Verification — run once, at the end, from the repo root

```
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
```

Expected: frontend pass count unchanged, 0 failed; wasm32 clean; fmt silent.

Then the route oracles, which are the only instrument that can see a routing mistake:

```
CARGO_TARGET_DIR=target-container cargo run -q -p tbd-tools --bin gate -- v-suite verify
```

This gate is **red at baseline** — 21 of 25 routes fail on stale oracles, deferred as T-986 by
operator word. Acceptance is a diff against `docs/platform/engine_split_phase3_baseline.md`:
**set equality on the 21 failing route names** (the set may shrink, never grow), and the four clean
routes — `notfound`, `eventmgr`, `callback`, `login` — must still pass. A pass that becomes a
failure is a hard stop: report it and stop, do not re-freeze an oracle.

Paste every output verbatim, including the full failing-route list you diffed.

Commit directly to `main`:

```
refactor(engine-split): the diagnostics testbench becomes a v2 app (3B)
```
