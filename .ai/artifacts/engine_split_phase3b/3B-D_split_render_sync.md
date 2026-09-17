# 3B-D — `canvas/render_sync.rs` splits along the line it was already drawn on

Read `.ai/artifacts/engine_split_phase3b/00_rules_every_agent_obeys.md` first. Every rule there
applies to this brief.

At this point the editor tree lives at `apps/website/frontend/src/v2/apps/editor/`, still in its
old internal shape. This brief is the one split Phase 1 deliberately left alone.

## The file

`v2/apps/editor/canvas/render_sync.rs`, 998 lines: 22 functions, 3 structs, 1 enum, 8 consts. It
is misnamed — it holds no render sync. It is a pick/lane helper belt, and **every function in it is
already pure**: document JSON and numbers in, lane vertex arrays, hit answers and id universes out.
It imports no `leptos`, no `web_sys`, no `wasm_bindgen`, and reaches the engine only through
`website_map_engine::{data::store, overlay::symbology}`. That is why almost all of it belongs in
`website-map-engine` and why moving it costs no behaviour.

Its internal blocks: connections 50–194, comments 195–466, hover 467–602, routing 603–798,
selection/SoA 799–942, zone/paste 943–998. Re-derive those bounds; they shift as soon as you edit.

## Where each piece goes

**To `apps/website/map-engine/src/editing/`** — grouped by subject, per Law 5, every new file under
the engine's 500-line production ceiling (the engine is at zero violations and must stay there):

| functions | destination |
|---|---|
| `ConnSegment`, `CONN_LINE_RGBA`, `CONN_LINE_SELECTED_RGBA`, `CONN_PICK_PX`, `connection_segments`, `connection_lane_verts`, `pick_connection` | `editing/lanes/connections.rs` |
| `CommentPoint`, `COMMENT_PICK_PX`, `comment_points`, `comment_lane_xy`, `comment_lane_ids`, `pick_comment`, `dragged_comment_points`, `comment_drag_lane_xy` | `editing/lanes/comments.rs` |
| `marker_lane_fields` | `editing/lanes/markers.rs` |
| `RouteTarget`, `route_target`, `route_availability` | `editing/routing.rs` |
| `selectable_ids`, `crewed_slot_ids`, `map_render_keep_indices`, `map_render_slot_soa`, `filter_slot_soa_excluding`, `zone_centre`, `plain_paste_anchor` | `editing/selection_universe.rs` |

`editing/lanes/mod.rs` declares the three lane modules; `editing/mod.rs` declares `lanes`,
`routing` and `selection_universe` in the style of the eight declarations already there.
`website_map_engine::` prefixes inside the moved code become `crate::`.

**Stays in the frontend** — the tab-local hover state machine, which is a cursor policy for one
browser tab and not a document concept:

| functions | destination |
|---|---|
| `HoverState`, `HOVER_CURSOR_PICKABLE`, `HOVER_CURSOR_PLAIN`, `HOVER_THROTTLE_MS`, `HOVER_RELEASE_PX`, `hover_due`, `hover_next`, `hover_cursor_css`, `hover_suppressed` | `v2/apps/editor/canvas/pointer_hover.rs` |

`HOVER_RELEASE_PX` is defined as `COMMENT_PICK_PX * 1.5`, so it now reads that const from the
engine. Keep the relationship — do not inline the number.

`render_sync.rs` is deleted once both halves have left it. Its name describes nothing that exists.

## The seam — repoint, never delete

`mission_editor.rs` re-exports this belt `pub(crate)` under the same names in three blocks (near
lines 46–84 and again at the bottom). **Those re-exports are the seam**: the page's bare call
sites, the `mission_editor::…` paths in `state/history.rs` and the panel test modules, and the
evacuated pins' `use super::…` imports all spell the names through them. Repoint every one of
them at its new home — engine names through `website_map_engine::editing::…`, hover names through
`crate::v2::apps::editor::canvas::pointer_hover` — and **keep the `cfg` gates exactly as they
are**. Do not delete a re-export to "simplify"; that renames a call site in fifty places.

The `#[cfg(test)]` block at the bottom of `mission_editor.rs` must stay after every production
item — `class_r_scrub::live_code()` blanks a file from its first `#[cfg(test)]` to EOF.

## Pins

Three source-scrubbing tests read this file: `tests/t802_hover_cursor.rs`,
`tests/t808_symbology_feed.rs`, `tests/t784_comment_glyph.rs`. Brief 3B-A anchored them, so each
is a one-line path change to the file that now holds the code it argues about — a test that pins
the comment lane points at `editing/lanes/comments.rs`, one that pins hover points at
`pointer_hover.rs`. This is expected breakage, not a regression. **Repoint them; do not weaken an
assertion to make one pass.** Check for others with a grep for `render_sync` across the repo before
you finish.

The moved code's own unit tests move with it, into the engine's sibling-file convention
(`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`, declared at the bottom).

## Documentation

Every moved item keeps its documentation, rewritten to CLAUDE.md Law 8: what the code does now and
why — the invariant, the unit, the lane it feeds. The existing block comments narrate the tree's
history by ticket number across dozens of lines; that prose does not travel. Each new engine file
opens with the engine's house header (`Role: / Position: / Signals & state: / Invariants:`).
Engine files are audited by nothing that grandfathers them, so they must be clean on landing.

## Verification — run once, at the end, from the repo root

```
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
```

Expected: map-engine >= 1414 passed (the belt's own tests land here, so the count should **rise**),
0 failed. `verify engine-layers` PASS, all 8 rules — rule 5 bans `web_sys`/`leptos`/`wasm_bindgen`
anywhere under `map-engine/src/editing/**`, so a stray import fails it outright. Frontend pass
count unchanged, 0 failed. wasm32 clean. fmt silent.

Also paste `rg -n 'render_sync' --type rust` over the repo — it must be empty.

## Report

Paste every output verbatim. State the line counts of the six new files, and name every re-export
you repointed.

Commit directly to `main`:

```
refactor(engine-split): the canvas helper belt resolves into engine lanes and a hover policy (3B)
```
