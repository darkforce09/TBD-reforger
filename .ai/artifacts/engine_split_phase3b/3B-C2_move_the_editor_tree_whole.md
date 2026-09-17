# 3B-C2 — the editor tree moves into `v2/apps/editor/`, unchanged inside

Read `.ai/artifacts/engine_split_phase3b/00_rules_every_agent_obeys.md` first. Every rule there
applies to this brief.

Briefs 3B-A, 3B-B and 3B-C1 have landed: pins address their subject from the crate root,
`v2::apps` is a compiled module with a dated allowlist behind its documentation audit, and the
editor tree owes no header or `pub`-item documentation.

## What this brief does — and what it deliberately does not

This is a **relocation, not a reshape**. `apps/website/frontend/src/editor/` moves to
`apps/website/frontend/src/v2/apps/editor/` with its internal shape byte-identical: `panels/` is
still `panels/`, `canvas/` is still `canvas/`, `state/` is still `state/`. The reshape into
`ui/ input/ bridge/ shell/ arsenal/` is briefs 3B-E through 3B-K, one destination folder at a time.

Doing it in this order is the whole point: every intermediate state compiles, every later brief is
a small diff, and no brief ever needs a `#[path = "..."]` escape hatch to reach a module that has
half-moved.

## The move

1. **Clear the landing site.** `v2/apps/editor/` currently holds 28 stale `README.md` files
   describing a `features/` + `ui/{top,left,right,bottom,canvas,modals}` + `state/` shape that
   contradicts CLAUDE.md's atlas and references a deleted `src/v2/map_engine`. By operator
   decision CLAUDE.md wins and they are rewritten — brief 3B-M writes the new set against the
   landed tree. Delete them here so the directories they occupy do not collide with the move.
   Leave `v2/apps/planner/`, `v2/apps/aar/` and `v2/apps/debug/` alone.
2. **Move the contents**, not the directory: `src/editor/<entry>` → `src/v2/apps/editor/<entry>`
   for every entry, with `git mv` so history follows.
3. **`mission_editor_tests/` becomes `tests/`.** The audit skips directories literally named
   `tests`, which is what exempts these 36 files (8,019 LOC) from it. Update the 36
   `#[path = "mission_editor_tests/..."]` declarations in `mission_editor.rs` to `tests/...`.
   Those declarations sit at the bottom of the file, after every production item — **keep them
   there**, `class_r_scrub::live_code()` blanks a file from its first `#[cfg(test)]` to EOF.
   `state/tests/` keeps its name and travels with `state/`.
4. **Declare it.** `mod editor;` leaves `main.rs`; `pub mod editor;` joins `v2/apps/mod.rs`.
5. **Sweep the paths.** 631 `crate::editor::` references across 72 files become
   `crate::v2::apps::editor::`. Rustdoc links in comments (`[`crate::editor::...`]`) count — sweep
   them too, then grep for a surviving `crate::editor` and paste the (empty) result.
6. **Repoint the anchored pins.** Brief 3B-A anchored every cross-file pin as
   `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/editor/..."))`. Those suffixes become
   `"/src/v2/apps/editor/..."`. `v2/core/test_support/editor_operations.rs` holds a block of them
   and is easy to miss — it names `src/editor/state/armed_placement/*` and
   `src/editor/state/entity_selection.rs`. Self-pins (a file pinning its own bare filename) need
   no change.
7. **Rewrite `editor/mod.rs`'s prose.** Its header and every inter-declaration comment narrate the
   tree's history by ticket number — "grows through the T-934 program", "before their renames",
   "split out of". That is a Law 8 violation travelling under its own power, and audit rule 5
   fails on it. Replace with a `//!` header in the house shape (Role / Position / Signals & state /
   Invariants) and comments that say what each module is **now**. Do not grandfather this file.
8. **Fill the allowlist.** Add a dated row for every moved production file that still breaks audit
   rule 2, 4 or 5 — 38 files over 500 lines, plus the files holding inline `#[cfg(test)] mod`
   blocks, plus the files whose comments name a ticket or a wave. Derive the exact set by running
   the audit and reading its failures; do not hand-guess it. For scale, the audit currently reports
   over `editor/`: 2,985 ticket/wave comment lines, 92 inline test modules, 42 oversized files, and
   34 missing `//!` headers — **every one of those 34 is in `mission_editor_tests/`, and the rename
   to `tests/` is what clears them.** If any header failure survives the rename, you renamed the
   wrong thing. Rules 1 and 3 are otherwise already at zero: brief 3B-C1 paid that debt. Every row carries a real `reason`
   naming what Phase 3C will do to that file, and `expires: 2026-12-31`.
9. **Repoint the gates outside the frontend.** `xtask/src/gate_t180.rs` holds 15
   `apps/website/frontend/src/editor/...` rows plus `EDITOR_OPS`, `ORBAT_MGR` and `EDEN_CHROME`
   consts, and its tests `fs::copy(...).unwrap()` every row — **one stale path panics every test
   in that gate**. `xtask/src/ai.rs:438` asserts on a literal
   `"cat apps/website/frontend/src/editor/mission_editor.rs"`. `xtask/src/migrate_v2.rs:736` names
   `apps/website/frontend/src/editor_ops.rs`; read what that table means before touching it and
   only repoint rows that name a live path. `ORBAT_MGR` still points at
   `src/pages/operations/orbat_manager.rs` after this brief — brief 3B-J moves that file.
   Finish by grepping all of `xtask/` and `tools/` for `src/editor` and pasting the result.

## What must not change

Module *names* and item *names*. No file is split, renamed (except `mission_editor_tests/` →
`tests/`), merged or deleted. No `pub` visibility changes. Zero behaviour change.

## Verification — run once, at the end, from the repo root

```
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
CARGO_TARGET_DIR=target-container cargo test -p xtask
```

Expected: `website-frontend` pass count unchanged from brief 3B-C1's, 0 failed — a drop means a
`#[path]` declaration or a `mod` line stopped compiling a test file. wasm32 clean. fmt silent.
`cargo test -p xtask` fails **only** the eight known names listed in the rules file; judge by name
set, never by count, and a `gate_t180` failure is yours.

Also paste:
```
git -C . status --porcelain apps/website/frontend/src | head -40
rg -n 'crate::editor' apps/website/frontend/src | wc -l        # must be 0
rg -n 'src/editor' xtask tools --type rust                     # must be empty
```

## Report

Paste every output verbatim. State the number of files moved, the number of `crate::editor`
call sites rewritten, and the number of allowlist rows added with their reasons summarised.

Commit directly to `main`:

```
refactor(engine-split): the editor app lands under the v2 apps domain (3B)
```
