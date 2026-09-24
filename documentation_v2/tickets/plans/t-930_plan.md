# T-930 — Plan

## Context

W210 eye-pass: a freshly placed vehicle paints as a yellow disc until it is moved. `pack_vehicle_instances` (`slots_gpu.rs:230`) packs vehicle discs; the place path is in `operations/entity.rs`; the first upload after place apparently lacks the real glyph state or the rebind is missed.

## Approach

1. Verify on main: render/e2e test placing a vehicle asserts the glyph on first paint (red).
2. Trace place → pack → upload; fix the missed invalidate/rebind (`mission_editor.rs`) or the pack input (`slots_gpu.rs`).
3. Perturbation proof; T-819 crewed-slot hide unchanged.

## Risks

- The disc may be a deliberate placeholder for prefab-less rows; confirm before changing the pack.

## Verification

- `cargo test -p map-engine-core --all-features` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-930`
