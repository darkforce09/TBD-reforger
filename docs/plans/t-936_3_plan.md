# T-936.3 — Plan

## Context
flatten.rs:1927-1960 derive_radio_plan synthesizes every net (T-203); :1876 says the editor has no radio UI
and no radioPlan in the payload. $defs/radioPlan and $defs/net (:481, freqMHz min 30) already exist;
Radio/TBD_RadioPlan.c parses them. No schema change is needed.

## Approach
1. `mission/radio_plan.rs` (new, in mission/mod.rs): model, range + duplicate checks, net cap (:1392).
2. Register `radioPlan` in extensions.rs AUTHORED_BLOCKS so compile.rs carries it.
3. flatten.rs: derive only when no authored plan is present; pass authored nets through unchanged.
4. `panels/radio_panel.rs` (new, in panels/mod.rs): nets, assignments, Reset-to-derived; undoable.
5. Perturbation: drop the duplicate-frequency check → red; restore, touch, green.

## Risks
- Default output must stay byte-identical (Class-R fixtures); gate the derive on absence only.
- flatten.rs shared with T-936.1 and T-674.1/T-675.1/T-290/T-291 → separate waves.

## Verification
- `cargo test -p map-engine-core --all-features`; `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`; `cargo xtask platform wave gate --slice T-936.3`
