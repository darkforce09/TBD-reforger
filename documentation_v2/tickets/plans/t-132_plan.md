# T-132 — Plan

## Context
The visual-git half of the backlog item (co-editing is T-295). Versions exist server-side (app.rs:751); nothing
computes or shows a difference between two of them.

## Approach
1. New `crates/map-engine-core/src/doc/diff.rs` (register in `doc/mod.rs`): `diff(a: &Value, b: &Value) -> DocDiff`
   keyed by slot/squad/vehicle/marker uid; moved = same uid, position delta > 0.5 m; changed = any other field.
   Tests on two committed goldens plus self-diff and swap symmetry.
2. New `frontend/src/pages/public/mission_diff.rs` (register in `pages/public/mod.rs`): version pickers, grouped
   change list, map overlay (from→to arrows) using the existing overlay path.
3. Perturbation: moved detection off → test red; restore, `touch`, green.

## Risks
- Uid stability across versions depends on T-674's slot uid; ids without uid fall back to id and are reported.

## Verification
- `cargo test -p map-engine-core --all-features doc::diff` · leptos gates · `cargo xtask platform wave gate --slice T-132`
