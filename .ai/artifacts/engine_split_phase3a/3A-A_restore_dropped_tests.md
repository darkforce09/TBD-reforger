# 3A-A — restore the dropped test coverage

**Run this first. It is a regression fix, and it is small. It is also the whole of your scope —
do not touch anything else.**

## Read the arithmetic before you touch anything

An earlier draft of this brief claimed three tests were dropped. **Only one was.** Measured:

```
#[test] in apps/website/frontend/src/editor/     at a4b86912d : 1117
#[test] in apps/website/frontend/src/editor/     at HEAD      : 1000     -> 117 lost
#[test] in apps/website/map-engine/src/editing/  at HEAD      :  116     -> 116 gained
                                                                            net -1
```

Two of the three **arrived under engine-native names** and are inside that 116:

| original frontend name | live engine name |
|---|---|
| `no_los_doc_writes` | `editing/tools/line_of_sight/tests/session_local.rs::the_line_of_sight_tool_never_writes_the_document` |
| `no_ruler_doc_writes` | `editing/tools/ruler/tests/session_local.rs::the_ruler_never_writes_the_document` |

Both already scrub the engine-side sources through `crate::source_scrub::strip_rust_lexical_noise`
and assert the same banned-token list. **Do not recreate `no_los_doc_writes` or
`no_ruler_doc_writes`.** Doing so would land a duplicate test of an invariant that is already
enforced. Operator decision, explicit: **repair, do not duplicate.**

## What is actually broken — this is your work, all four items

### 1 · One test is genuinely gone

`the_exporter_grid_ref_is_the_map_furnitures_own_label_text`, formerly in
`apps/website/frontend/src/editor/state/commands_hotkeys.rs`. It is the net -1. Recover the
original with:

```
git show a4b86912d:apps/website/frontend/src/editor/state/commands_hotkeys.rs
```

It is behavioural, not a scrub: it builds an `OrthoCamera`, calls
`toolbelt::{edge_eastings, edge_northings}`, and asserts the clipboard exporter's grid reference
equals the map furniture's own edge label text for an entity standing on that intersection. It
also pins one between-the-lines read: `format_grid_ref(1250.0, 4800.0) == "012 048"`.

**It must live frontend-side.** It spans the wall:

| symbol | side |
|---|---|
| `editor::layout::{DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX}` | frontend |
| `editor::panels::toolbelt::{edge_eastings, edge_northings, GRID_STEP_M}` (`toolbelt.rs:248,285`) | frontend |
| `OrthoCamera` | engine |
| `format_grid_ref` (`editing/commands/selection_digest.rs:170`) | engine |

The frontend may import the engine; the engine may never import the frontend. `format_grid_ref`
is `pub` under `pub mod selection_digest` in `editing/commands/mod.rs`, so it is reachable as
`website_map_engine::editing::commands::selection_digest::format_grid_ref` with no re-export
work needed. Put the test where it can still assert that equality and **give it one line saying
why it lives there**.

### 2 · Both engine scrubs have an identical hole: `mod.rs`

The ruler scrub covers 5 of the directory's 6 `.rs` files; the LOS scrub covers 9 of 10. `mod.rs`
is outside both. The original frontend tests scrubbed one whole file each, so they had no such
hole. Close it in both.

### 3 · `editor_ops` was dropped from both banned lists

The originals banned `["MissionDocCore","move_entities","add_slot","store.rs","hydrate",
"after_local_edit","editor_ops"]`. The engine versions ban `data::store` instead of `store.rs`
— that is **stronger**, keep it — but silently dropped `editor_ops`. Restore it to both lists.

It is safe today: the only `editor_ops` occurrences under `map-engine/src/editing/` are doc
comments at `commands/selection_digest.rs:40`, `:67` and `tools/selection/gesture.rs:65`, and the
scrub blanks comments before matching. Restoring the token catches a live-code reintroduction
tomorrow, which is the entire point of the guard.

### 4 · Comments cite test names that do not exist

These read as lies today:

| site | cites |
|---|---|
| `apps/website/map-engine/src/editing/tools/line_of_sight/tests/capture.rs:152` | `no_los_doc_writes` |
| `apps/website/map-engine/src/editing/tools/line_of_sight/tests/capture.rs:158` | `no_los_doc_writes` |
| `apps/website/frontend/src/editor/panels/toolbelt.rs:1369` | `no_ruler_doc_writes` |

Repoint them at the live engine test names. A fourth,
`apps/website/map-engine/src/editing/commands/selection_digest.rs:24`, cites the test from item 1
— it becomes true once you restore that test, so leave the citation and make it accurate.

**Then sweep the whole repo**: no comment anywhere may cite a `#[test]` name that does not exist.
Sweep it, do not spot-check it.

## Why this matters more than one test name

These scrubs are the proof that phase 3A is honest — that the tools compute and never mutate the
document. They are `include_str!` source scrubs, so they are bound to file paths: when a tool
moves, the scrub target moves with it or the guard silently stops guarding. Losing the invariant
to the very refactor it polices is not acceptable.

The same failure mode is waiting in phase 3B: `canvas/render_sync.rs` is `include_str!`d by
`mission_editor_tests/t802_hover_cursor.rs:21`, `t808_symbology_feed.rs:23` and
`t784_comment_glyph.rs:33`.

## Done when

- The exporter grid-ref test exists, runs, passes, and carries one line saying why it lives where
  it does.
- `mod.rs` is inside both tool scrubs.
- `editor_ops` is back in both banned lists.
- No comment anywhere cites a test that does not exist.
- You show the arithmetic and it balances. Restoring the test frontend-side makes it
  1117 -> 1001 lost 116, gained 116.

```
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
```

Baseline at `bf7c366d7`: map-engine 1290 passed / 0 failed / 2 ignored, frontend 1316 / 0.
Counts may only rise.
