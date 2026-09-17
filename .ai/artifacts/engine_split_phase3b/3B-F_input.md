# 3B-F — `input/`: DOM pointer and keyboard events become engine commands

Read `00_rules_every_agent_obeys.md` first. Every rule there applies to this brief.

CLAUDE.md's atlas defines `input/` as "DOM Pointer/Keyboard events -> Map Engine commands". Three
surfaces answer to that description and one of them has to be carved out of a larger file.

## The move

| from `v2/apps/editor/` | to `v2/apps/editor/input/` |
|---|---|
| `canvas/gestures.rs` | `pointer_gestures.rs` |
| `canvas/commands.rs` | `window_keydown.rs` |
| `tools/los_tool.rs`, `tools/ruler_tool.rs`, `tools/select_tool.rs`, `tools/viewshed_scheduler.rs`, `tools/los_world_wasm.rs`, `tools/mod.rs` | `tools/` |
| `state/history.rs`'s `register_key_handler` — the window `keydown` that binds Ctrl/Cmd+Z and Ctrl/Cmd+Shift+Z / Ctrl+Y | folded into `window_keydown.rs` |

`canvas/commands.rs` is the window-level keydown dispatch — its own header says so and explicitly
distinguishes itself from `state/commands_hotkeys.rs`, which is a different surface and goes to
`shell/` in brief 3B-G. The program document's table puts this file in `bridge/`; **the approved
plan corrects that and this brief follows the correction.** The rename to `window_keydown.rs` is
CLAUDE.md Law 4: `commands.rs` under `input/` next to an unrelated `commands_hotkeys` is exactly
the overloaded name the law bans.

`canvas/` is empty once these two leave. Delete the directory and its `mod.rs`.

## The `history.rs` carve-out

`state/history.rs` holds the undo/redo driver **and** a window keydown installer. The keydown half
is input; the driver is the document host and stays for brief 3B-G. Move only the installer and
whatever private helper exists solely for it. The undo/redo entry points it calls (`undo`, `redo`)
stay where they are and are called across the module boundary — `history.rs` is the only path to
them, and that invariant must survive the split. Do not duplicate a line of it.

## What travels, what to repoint

As in brief 3B-E: sibling test files and their bottom-of-file declarations, allowlist rows updated
in the same commit, `//!` headers rewritten where they name an old home, anchored source pins
repointed to the new suffixes, `crate::v2::apps::editor::{canvas,tools}::…` call sites swept, and
`xtask/` + `tools/` grepped for any path you moved. `mission_editor.rs`'s re-export blocks are
repointed, never deleted.

`help_modal.rs`'s keymap census reads `commands.rs` and `gestures.rs` **by pinned source text** and
keeps a table of `(filename, source, expected count)` rows. Repoint those rows at the new paths and
update the filename strings to match; the counts do not change, and if one does, you changed
behaviour. `t726_window_esc_stack.rs` and `t662_input_traps.rs` pin the same surfaces.

## `website-map-engine` pins `gestures.rs` — this one is not optional

`apps/website/map-engine/src/data/store/rows/tests/cases_1.rs` holds an `include_str!` of
`canvas/gestures.rs`. You are moving that file, so **repoint that pin and run the engine's test
build**. A sweep scoped to the frontend left this crate's tests uncompilable once already in this
phase, and nobody noticed until the next brief tripped over it.

## Verification — run once, at the end, from the repo root

```
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
```

Expected: frontend pass count unchanged, 0 failed; map-engine >= 1414 passed, 0 failed; wasm32
clean; fmt silent. Paste all four verbatim plus `git status --porcelain` for your staged paths,
and prove `canvas/` is gone.

Commit directly to `main`:

```
refactor(engine-split): pointer gestures, window keys and tool drivers become the editor's input layer (3B)
```
