# T-142 — Plan

## Context
Layout-only polish from the north-star backlog: toolbelt home, grouped Attributes modal, honest stub tools. It
packs after the T-939.x fixes to the same two files and before T-158 (dock/top-bar consolidation).

## Approach
1. `panels/attributes_modal.rs`: wrap existing fields in four groups (Identity / Position / Gear / Crew); keep every
   field id and mutator; wasm test asserts the rendered field-id digest is unchanged.
2. `panels/top_strip.rs`: render the toolbelt row directly under the strip; stub tools get `disabled` + tooltip
   "not yet — see backlog"; wasm test: stub click does not change any signal.
3. Perturbation: hide a stub tool → visibility test red; restore, `touch`, green.

## Risks
- T-939.x may have moved the RowMirror; rebase and re-read before editing.
- Grouping must not alter tab order; test focus sequence on the modal.

## Verification
- `cargo xtask mk ci-local-leptos` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-142`
