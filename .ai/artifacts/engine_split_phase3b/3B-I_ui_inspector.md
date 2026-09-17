# 3B-I — `ui/inspector/`: the panels that edit what is selected

Read `00_rules_every_agent_obeys.md` first. Every rule there applies to this brief.

## The move

Eleven files from `v2/apps/editor/panels/` to `v2/apps/editor/ui/inspector/`, keeping their names:

```
attributes_modal.rs   env.rs              weather_timeline.rs   zones_panel.rs
vehicles_panel.rs     radio_panel.rs      tasks_panel.rs        spawn_modules.rs
audio_emitters.rs     win_conditions_card.rs                    validation_panel.rs
```

`ui/inspector/mod.rs` declares the eleven with a `//!` header in the house shape, each `pub mod`
line carrying the `cfg` gate of the code it declares. `panels/mod.rs` keeps declaring only what is
still under `panels/` — after this brief that is `help_modal.rs` and `settings_modal.rs`, which
brief 3B-J takes to `ui/modals/`.

Nothing is split. `attributes_modal.rs` (4,076), `validation_panel.rs` (2,563) and
`zones_panel.rs` (2,153) carry allowlist rows and are Phase 3C's subject.

## Pins

`attributes_modal.rs` holds 35 self-pins and `validation_panel.rs` 9 — self-pins travel with their
file and need no change. What needs attention: `attributes_modal.rs` and `zones_panel.rs` pin
map-engine sources and `mission_editor.rs`; `orbat_manager.rs` pins `attributes_modal.rs`;
`toolbelt.rs` pins `attributes_modal.rs`; `help_modal.rs`'s keymap census carries an
`attributes.rs` row pointing at it. All are anchored, so each is a one-line suffix change. After
the move, grep the crate for the old suffixes and paste the empty result.

## What travels, what to repoint

Sibling test files with their bottom-of-file declarations. Allowlist rows updated in the same
commit. `//!` headers rewritten where they name `editor/panels`. Every
`crate::v2::apps::editor::panels::…` call site for these eleven swept. `mission_editor.rs`'s mounts
and re-exports repointed, never deleted. `xtask/` and `tools/` grepped for any path you moved.

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
refactor(engine-split): the inspector panels take their ui home (3B)
```
