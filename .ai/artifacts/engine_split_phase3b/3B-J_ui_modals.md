# 3B-J — `ui/modals/`, and the two editor dialogs that were never pages

Read `00_rules_every_agent_obeys.md` first. Every rule there applies to this brief.

## The move

| from | to `v2/apps/editor/ui/modals/` |
|---|---|
| `v2/apps/editor/panels/help_modal.rs` | `help_modal.rs` |
| `v2/apps/editor/panels/settings_modal.rs` | `settings_modal.rs` |
| `src/pages/operations/orbat_manager.rs` | `orbat_manager.rs` |
| `src/pages/operations/faction_manager.rs` | `faction_manager.rs` |

`panels/` is empty afterwards. Delete the directory and its `mod.rs`.

**Why the last two belong here.** Neither is a page. Neither is routed — `router.rs` has no entry
for either. They are mounted only from inside the editor: `mission_editor.rs` mounts
`FactionManagerDialog` directly, and `OrbatManagerDialog` through a bare `pub use` in
`shell/eden_chrome.rs` that exists to keep the mount path stable. A non-routed editor dialog living
under `pages/` is precisely the naming failure CLAUDE.md Law 4 targets.

**They are not duplicates of `v2/pages/operations/orbat_selection/`.** One authors an ORBAT inside
the editor; the other is the routed slotting view where people sign up. They stay distinct — the
shared noun is not a merge candidate. Do not touch `v2/pages/operations/`.

`src/pages/operations/mod.rs` drops its two `pub mod` lines. If that leaves the module empty, say
so in your report and leave the rest of `pages/operations/` alone — the other briefs own it.

## The five pins that hold these two files

Re-derive every line number; they move the moment you edit.

1. `v2/core/ui/tests/ui.rs` scrubs `orbat_manager.rs` and asserts `OrbatManagerDialog` takes its
   overlay z from `modal_stack::z_class`. It is **outside** the editor tree — easy to miss.
2. `ui/docks/dock_right.rs` scrubs `orbat_manager.rs`.
3. `help_modal.rs`'s keymap census pins **both** files by source with
   `(filename, expected count)` rows, because each installs a window-level keydown of its own and
   both are live whenever the Mission Creator is up. The census exists to catch exactly that, so
   its rows must follow the files and its counts must not change.
4. `mission_editor.rs` mounts `FactionManagerDialog` by full path.
5. `shell/eden_chrome.rs` re-exports `OrbatManagerDialog`. **Repoint the re-export; do not delete
   it** — a symbol search for the dialog lands on it, and the editor's mount path depends on it.

`xtask/src/gate_t180.rs`'s `ORBAT_MGR` const names `apps/website/frontend/src/pages/operations/
orbat_manager.rs` and its tests `fs::copy(...).unwrap()` it, so a stale path panics every test in
that gate. Repoint it, then grep the rest of `xtask/` and `tools/`.

## Documentation debt

`orbat_manager.rs` and `faction_manager.rs` have never been audited — they are landing under
`src/v2` for the first time. The audit's rules 1 and 3 are **not** exemptible: give them a `//!`
header in the house shape if they lack one, and document every `pub` item they leave undocumented.
Derive the exact list by running the audit. Rules 2, 4 and 5 (size, inline test module, ticket and
wave names) get dated allowlist rows like every other moved file — `orbat_manager.rs` is 2,128
lines and is split in Phase 3C, not here.

## What travels, what to repoint

Sibling test files with their bottom-of-file declarations. Allowlist rows added or updated in the
same commit. Anchored pins repointed. Every `crate::pages::operations::{orbat,faction}_manager` and
`crate::v2::apps::editor::panels::{help,settings}_modal` call site swept.

## Verification — run once, at the end, from the repo root

```
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
CARGO_TARGET_DIR=target-container cargo test -p xtask
```

Expected: frontend pass count unchanged, 0 failed; wasm32 clean; fmt silent; `cargo test -p xtask`
failing only the eight known names — a `gate_t180` failure is yours. Paste all four verbatim plus
`git status --porcelain` for your staged paths, and prove `panels/` is gone.

Commit directly to `main`:

```
refactor(engine-split): the editor's modal dialogs stop pretending to be pages (3B)
```
