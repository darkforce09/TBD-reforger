# T-926 — Plan

## Context
Wave-210 eye-pass: vehicle Attributes show Heading/Cargo/Crew (T-818) but no Position X/Y/Z,
unlike slot Attributes' Transform tab. Operator asked for parity.

## Approach
1. In `panels/attributes_modal.rs`, add a Transform section to the vehicle body: Position X/Y/Z + Heading using the existing `number_field` seam.
2. Commit edits through the vehicle pose operation the Heading field already uses; z via `keep_z_rows()`/`slot_z()` pattern, never the SoA.
3. Class-R pin: the vehicle Attributes DOM contains the four fields; perturb by removing one field.

## Risks
`attributes_modal.rs` is shared with T-939.2/T-142/T-927 — packs in a later wave. Column alignment: build ids/positions from one sorted source.

## Verification
`cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`; `cargo xtask platform wave gate --slice T-926`.
