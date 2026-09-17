# 3B-G — `shell/` and the host state behind the bridge

Read `00_rules_every_agent_obeys.md` first. Every rule there applies to this brief.

`state/` is the last of the old top-level folders. It is not one thing: half of it is the browser
session — what a tab owns, what it saves, what it restores — and half is the host-side signal
state the engine's hosted commands read. CLAUDE.md's atlas has a home for each: `shell/` is "tab
locks, autosave persistence, session preferences", `bridge/` is the engine seam.

## The move

**To `v2/apps/editor/shell/`** — the browser session:

| from | to |
|---|---|
| `state/persist.rs` | `persist.rs` |
| `state/hydrate.rs` | `hydrate.rs` |
| `state/tab_lock.rs` | `tab_lock.rs` |
| `state/save_status.rs` | `save_status.rs` |
| `state/title_prefer.rs` | `title_prefer.rs` |
| `state/session.rs` | `session.rs` |
| `state/commands_hotkeys.rs` | `document_commands.rs` |
| `layout.rs` | `layout.rs` |
| `mission_size.rs` | `mission_size.rs` |
| `world_layer_prefs.rs` | `world_layer_prefs.rs` |
| `eden_chrome.rs` | `eden_chrome.rs` |

`state/commands_hotkeys.rs` binds no hotkeys — 3A moved the command half into
`website_map_engine::editing::commands`, and what remains is the browser transport: the authed
POST, the file download, the clipboard write, the toast, the merge report and the
`window.__editorCommands` smoke bridge. Its own header says exactly that. The rename is Law 4, and
its `state/tests/exporter_grid_reference.rs` sibling travels with it as `shell/tests/…`.

**To `v2/apps/editor/bridge/host_state/`** — what the engine's hosted commands read:

| from | to |
|---|---|
| `state/editor_context/` (5 files) | `editor_context/` |
| `state/armed_placement/` (4 files) | `armed_placement/` |
| `state/entity_selection.rs` | `entity_selection.rs` |
| `state/undo_grouped_gestures.rs` | `undo_grouped_gestures.rs` |

**To `v2/apps/editor/bridge/document_host/`** — the hosted document and its undo drive:

| from | to |
|---|---|
| `state/doc_host.rs` | `doc_host.rs` |
| `state/history.rs` (what brief 3B-F left) | `history.rs` |

`state/` and its `mod.rs` are gone at the end of this brief.

## The gate that will bite you

`xtask/src/gate_t180.rs` names **fifteen** of these files by path — `EDITOR_OPS` points at
`state/editor_context/mod.rs`, `EDEN_CHROME` at `eden_chrome.rs`, and a table lists the
`armed_placement/` and `editor_context/` files plus `entity_selection.rs` and
`undo_grouped_gestures.rs`. Its tests `fs::copy(...).unwrap()` every row, so **one stale path
panics every test in that gate and `verify_t180` fails outright**. Repoint all of them, and grep
the rest of `xtask/` and `tools/` before you finish.

## What travels, what to repoint

Sibling test files with their bottom-of-file declarations; allowlist rows updated in the same
commit; `//!` headers rewritten where they name `editor/state`; anchored pins repointed (several
files under `state/` hold them — `tab_lock.rs`, `title_prefer.rs`, `save_status.rs`); every
`crate::v2::apps::editor::state::…` call site swept; `mission_editor.rs`'s
`use crate::…::state::{editor_context, history as mission_history, hydrate as mission_hydrate,
persist as yrs_persist}` aliases repointed with their `cfg` gates intact — the aliases keep their
spelling, only the path behind them changes.

`eden_chrome.rs` re-exports `OrbatManagerDialog` from `crate::pages::operations::orbat_manager`.
That re-export keeps the editor's mount path stable and **stays**; brief 3B-J moves the file it
points at and repoints it then.

## Verification — run once, at the end, from the repo root

```
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
CARGO_TARGET_DIR=target-container cargo test -p xtask
```

Expected: frontend pass count unchanged, 0 failed; wasm32 clean; fmt silent; `cargo test -p xtask`
failing **only** the eight known names — a `gate_t180` failure is yours. Paste all four verbatim,
plus `git status --porcelain` for your staged paths, and prove `state/` is gone.

Commit directly to `main`:

```
refactor(engine-split): the editor's session shell and host state take their atlas homes (3B)
```
