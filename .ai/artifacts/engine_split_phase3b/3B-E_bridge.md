# 3B-E — `bridge/`: the canvas mount and everything that drives a frame

Read `00_rules_every_agent_obeys.md` first. Every rule there applies to this brief.

The editor now lives at `apps/website/frontend/src/v2/apps/editor/` in its old internal shape, and
`render_sync.rs` is gone. This brief is the first of the reshape briefs, one destination folder
each. CLAUDE.md's atlas defines `bridge/` as the canvas mount, DPR scaling and rAF heartbeat
connector — the frontend's side of the engine seam.

## The move

| from `v2/apps/editor/` | to `v2/apps/editor/bridge/` |
|---|---|
| `canvas/boot.rs` | `boot.rs` |
| `canvas/viewport.rs` | `viewport.rs` |
| `canvas/overlays.rs` | `overlays.rs` |
| `canvas/gizmo_z.rs` | `gizmo_z.rs` |
| `canvas/tactical_graphics.rs` | `tactical_graphics.rs` |
| `canvas/tactical_graphics_authoring.rs` | `tactical_graphics_authoring.rs` |
| `canvas/pointer_hover.rs` | `pointer_hover.rs` |
| `world_assets/mod.rs` | `world_assets.rs` |

`canvas/gestures.rs` and `canvas/commands.rs` stay where they are — brief 3B-F takes them to
`input/`, and `canvas/` disappears there. Do not anticipate that move.

`bridge/mod.rs` declares the eight modules with a `//!` header in the house shape; each `pub mod`
line carries the same `cfg` gate as the code it declares (`world_assets` is
`#[cfg(target_arch = "wasm32")]` today, and that gate travels with it).
`canvas/mod.rs` keeps declaring only what is still under `canvas/`.

## What travels with each file

- its sibling test files and their `#[cfg(test)] #[path = "tests/…"] mod …;` declarations, which
  stay at the **bottom** of the file — `class_r_scrub::live_code()` blanks a file from its first
  `#[cfg(test)]` to EOF;
- its allowlist rows: update the path of every row naming a file you moved, in the same commit. A
  row naming a path that does not exist is a hard failure, by design;
- its `//!` header, rewritten if it names its old location.

## What to repoint

- `crate::v2::apps::editor::canvas::…` call sites across the crate → `…::bridge::…`.
- `mission_editor.rs`'s `pub(crate) use` re-export blocks for the overlay components
  (`AssetPickerOverlay`, `CommentEditorOverlay`, `ConflictDialog`, `ConnectionsPanelOverlay`,
  `SnapReadout`, `TransformWidgetOverlay`, `WidgetModeHint`, `read_widget_pivot`,
  `register_widget_pivot`, `AssetPickerState`, `ConflictInfo`). **Repoint, never delete** — they
  are the seam the page's bare mounts spell through, and their `cfg` gates are load-bearing.
- anchored source pins naming `/src/v2/apps/editor/canvas/<moved file>` → the new suffix. These are
  one-line changes because brief 3B-A anchored them; find them with a grep for the old suffix.
- `xtask/` and `tools/`: grep both for any path naming a file you moved before you finish.

## Verification — run once, at the end, from the repo root

```
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
```

Expected: frontend pass count unchanged from the previous brief, 0 failed. Most of this tree is
`cfg(target_arch = "wasm32")`, so the wasm32 check is mandatory here, not an extra. fmt silent.

Paste all three verbatim, plus `git status --porcelain` for the paths you staged and the (empty)
result of grepping the repo for the old module path.

Commit directly to `main`:

```
refactor(engine-split): the canvas mount and its overlays become the editor's engine bridge (3B)
```
