# T-141 — Plan

## Context
T-071 numbers slots; names remain the bare role. The backlog asks for word-pack names with a manual override that
wins. The generator is pure and testable; the only mutation change is in orbat_add_slot and rename.

## Approach
1. New `state/operations/slot_naming.rs` (register in `state/operations.rs`): two word packs, `name_for(seed, index,
   role) -> String`, wire-safe; tests for determinism, in-squad uniqueness (first 64 indices), wire-safety.
2. `state/operations/entity.rs`: orbat_add_slot (:2135) writes the generated name; the rename op sets
   `nameOverride = true`; renumbering skips overridden slots.
3. Perturbation: drop the seed from the hash → determinism test red; restore, `touch`, green.

## Risks
- Adding a slot key touches the document shape; use an editor-side key (not wire) so flatten is untouched.
- Word packs must avoid real-unit slurs/ambiguity; keep them short and reviewed in the PR body.

## Verification
- `cargo xtask mk ci-local-leptos` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-141`
