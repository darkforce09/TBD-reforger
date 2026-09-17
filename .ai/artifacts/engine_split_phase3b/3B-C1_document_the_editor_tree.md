# 3B-C1 — pay the editor tree's documentation debt, before it moves

Read `.ai/artifacts/engine_split_phase3b/00_rules_every_agent_obeys.md` first. Every rule there
applies to this brief.

## Why now

Brief 3B-C2 moves `apps/website/frontend/src/editor/` under `src/v2`, where
`v2/tests/doc_audit/` audits every production file. Three of its five rules — size, inline test
modules, ticket/wave comments — are carried on the dated allowlist brief 3B-B built, because
clearing them is Phase 3C's subject. **Two are not exempted and never will be:** every file opens
with a `//!` header, and every `pub` item carries a doc comment.

The editor tree owes exactly:

```
1  file with no `//!` header      src/editor/canvas/gizmo_z.rs
75 undocumented `pub` items       across 21 files
```

Paying that debt while the files still sit at their old paths keeps it out of the move commit,
where it would be indistinguishable from the move itself.

## The files that owe items

```
 14  src/editor/panels/audio_emitters.rs      8  src/editor/panels/outliner_drag.rs
  7  src/editor/arsenal/arsenal_rules.rs      6  src/editor/state/tab_lock.rs
  6  src/editor/canvas/gizmo_z.rs             5  src/editor/state/save_status.rs
  4  src/editor/mission_editor.rs             4  src/editor/canvas/viewport.rs
  3  src/editor/panels/top_strip.rs           3  src/editor/panels/dock_left.rs
  2  src/editor/state/title_prefer.rs         2  src/editor/panels/dock_right.rs
  2  src/editor/canvas/overlays.rs            2  src/editor/arsenal/arsenal_doll.rs
  1  each: world_layer_prefs.rs · state/editor_context/mod.rs · panels/settings_modal.rs ·
        panels/outliner.rs · panels/context_menu.rs · layout.rs · canvas/boot.rs
```

Derive the precise item list yourself — the audit's own rule functions are the oracle. An item
needs a doc when it is a `pub`/`pub(crate)`/`pub(super)` `fn`, `struct`, `enum`, `type`, `const`,
`static` or `trait`, or a `#[component]`, and the line above it (skipping attribute lines) is not
`///` or `#[doc`.

## How to write them

These are load-bearing docs, not placeholders. Read what the item actually does and say it.

- **Say what it does now and why it exists** — the invariant it holds, the unit its number is in,
  the surface it drives. CLAUDE.md Law 8: no history, no "was previously", no "moved from".
- **Never name a ticket or a wave.** `T-774`, `wave129`, `w145` and friends fail audit rule 5,
  which reads `///` lines too. If an existing comment near the item explains a behaviour by
  ticket number, restate the behaviour and drop the number.
- Match the surrounding house style: the tree's existing docs are full sentences, and the file
  headers use the `**Role:** / **Position:** / **Signals & state:** / **Invariants:**` shape.
  `gizmo_z.rs`'s new `//!` header follows that shape.
- A one-line `/// Whatever.` is right for a small helper; a component or a state struct deserves
  its signals and its invariant.

## Scope

The 21 files above, plus `gizmo_z.rs`'s header. Documentation only — **no code changes at all**,
no reordering, no renaming, no test edits. The diff must be additive comment lines and nothing
else, so `git diff -w --stat` shows only additions.

Files under `src/pages/debug/` and `src/pages/operations/` owe items too; they are **not yours** —
briefs 3B-J and 3B-L pay those with their moves.

## Verification — run once, at the end, from the repo root

```
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
CARGO_TARGET_DIR=target-container cargo check --target wasm32-unknown-unknown -p website-frontend
CARGO_TARGET_DIR=target-container cargo fmt --all -- --check
```

Expected: no change in the pass count beyond what brief 3B-B added, 0 failed; wasm32 clean; fmt
silent. Several of these files are `cfg(target_arch = "wasm32")` in whole or in part, so the wasm32
check is mandatory here, not an extra.

Then prove the debt is paid by running the audit's two rules over the 73 editor production files
and pasting the result: zero missing headers, zero undocumented items.

## Report

Paste all outputs verbatim, plus `git diff --stat` for the files you staged.

Commit directly to `main`:

```
docs(engine-split): the editor tree documents its public surface (3B)
```
