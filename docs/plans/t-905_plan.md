# T-905 — Plan
## Context
T-899 allowlisted the frontend SIZE-3 files (>1000 lines) in `.coding-standards-allowlist.yaml` with reason + expiry so the gate could go live; the rows are debt. Since then `editor/state/operations.rs` became a 45-line root whose mass is `operations/entity.rs` (4779 lines); `gestures.rs`, `loadout.rs` and `pages/debug/building_viewer.rs` are on the list too — all owned here. This ticket collides with every live editor slice, so it packs alone.

## Approach
1. Verify on main: `cargo xtask verify file-length` is green only because of the rows; list each owned file's line count.
2. Per file: keep `<stem>.rs` as the public surface and move responsibilities into `<stem>/<responsibility>.rs` submodules declared from `<stem>.rs` (e.g. `panels/attributes_modal/{loadout,transform}.rs`); no `_part2` names; parent `mod.rs` untouched; `pub use` keeps every path stable.
3. Drop each file's row from `.coding-standards-allowlist.yaml` as it crosses under 1000 lines (`operations.rs` row drops immediately); never raise the threshold.
4. `cargo xtask mk leptos-gates` after every few files; commit per file group.

## Risks
- `mission_editor.rs` and `los_tool.rs` carry cfg/test guards keyed on paths (CARGO_MANIFEST_DIR includes) — grep before moving.
- The slice is oversized; if a wave budget cuts it, the remaining rows stay on the allowlist with unchanged expiry and the report lists them.

## Verification
- `cargo xtask verify file-length` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-905`
