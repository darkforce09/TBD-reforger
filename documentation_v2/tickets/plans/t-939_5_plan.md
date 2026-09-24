# T-939.5 — Plan

## Context
asset_catalog.rs has zero squad rows (:321, :460 drop *_base.et); Compositions palette dock_right.rs:63-92 lists from the
doc. No compositions table in the API → defaults compile into the frontend.

## Approach
1. Verify on main: test that a shipped faction's catalog yields zero squad rows; paste the red.
2. `arsenal/squad_templates.rs` (new, in arsenal/mod.rs): `SquadTemplate`, defaults per faction, `from_composition`.
3. asset_catalog.rs: Squads section merging defaults + doc Compositions.
4. dock_right.rs: place a template as one operation (squad + role-named slots); numeric suffix on name clash.
5. Perturbation: drop the suffix logic → naming test red; restore, touch, green.

## Risks
- Role names must match the slot-naming rules (T-141); reuse its helpers.
- Default tables drift from the faction library: keep them small and unit-tested.

## Verification
- `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-939.5`
