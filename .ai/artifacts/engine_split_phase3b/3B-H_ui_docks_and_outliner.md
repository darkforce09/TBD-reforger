# 3B-H — `ui/docks/` and `ui/outliner/`

Read `00_rules_every_agent_obeys.md` first. Every rule there applies to this brief.

`panels/` is 22 files and 43,734 lines — the largest flat dump in the frontend, and exactly what
CLAUDE.md Law 5 exists to prevent. Three briefs take it apart by surface. This one takes the
docked chrome and the outliner.

## The move

| from `v2/apps/editor/panels/` | to `v2/apps/editor/ui/docks/` |
|---|---|
| `dock_left.rs` | `dock_left.rs` |
| `dock_right.rs` | `dock_right.rs` |
| `top_strip.rs` | `top_strip.rs` |
| `toolbelt.rs` | `toolbelt.rs` |
| `context_menu.rs` | `context_menu.rs` |

| from `v2/apps/editor/panels/` | to `v2/apps/editor/ui/outliner/` |
|---|---|
| `outliner.rs` | `outliner.rs` |
| `outliner_tree.rs` | `tree.rs` |
| `outliner_drag.rs` | `drag.rs` |

`outliner_tree` and `outliner_drag` lose the stutter once they sit inside `outliner/` — Law 4.
`outliner.rs` keeps its name as the surface's entry point; `ui/outliner/mod.rs` declares the three.
`ui/mod.rs` declares `docks` and `outliner` (and grows as briefs 3B-I, 3B-J and 3B-K land their
folders); `ui/docks/mod.rs` declares the five.

**These files are not split here.** `dock_right.rs` is 6,483 lines and `top_strip.rs` is 4,765;
both carry an allowlist row and both are Phase 3C's subject. Move them whole.

## Pins — this brief has the densest concentration in the tree

`dock_right.rs`, `top_strip.rs` and `toolbelt.rs` hold 111 self-pins between them. A self-pin
(`include_str!("dock_right.rs")` inside `dock_right.rs`) needs **no change**: a file always
travels with itself. Do not touch them.

What does need repointing:

- `dock_right.rs` pins `pages/operations/orbat_manager.rs` and several map-engine files. Brief
  3B-A anchored those, so each is a one-line suffix change only if its **subject** moved — the
  orbat manager has not moved yet (brief 3B-J), so leave that one alone.
- `toolbelt.rs` pins `attributes_modal.rs`, `settings_modal.rs` and `dock_right.rs` across what is
  about to become a folder boundary; `help_modal.rs` pins `top_strip.rs` and `context_menu.rs` the
  same way. They are anchored, so they keep working as long as you update the suffix of any file
  **you** move. Grep the whole crate for the old suffixes after the move and paste the empty result.
- `help_modal.rs`'s keymap census keeps `(filename, source, expected count)` rows for files in this
  brief. Update the paths and filename strings; the counts must not change.

## What travels, what to repoint

Sibling test files with their bottom-of-file declarations — `class_r_scrub::live_code()` blanks a
file from its first `#[cfg(test)]` to EOF, and these files are read by scrubs. Allowlist rows
updated in the same commit. `//!` headers rewritten where they name `editor/panels`. Every
`crate::v2::apps::editor::panels::…` call site swept. `mission_editor.rs`'s re-exports and mounts
repointed, never deleted. `xtask/` and `tools/` grepped for any path you moved.

## Verification — run once, at the end, from the repo root

```
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
```

Expected: frontend pass count unchanged, 0 failed; wasm32 clean; fmt silent. Paste all three
verbatim plus `git status --porcelain` for your staged paths.

Commit directly to `main`:

```
refactor(engine-split): the docked chrome and the outliner take their ui homes (3B)
```
