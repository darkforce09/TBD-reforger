# data/scenario

The authored mission. Public mission interfaces: existing compiler import paths remain available
while implementations live in the AST, compiler, extensions, and validation modules. `slot_line`
formats plain-text ORBAT summaries.

The aliases this module's `mod.rs` publishes (`orbat`, `compile`, `flatten`, `kit`, `validate`,
`wire_safety`, `audio`, `weather`, `spawn_modules`, `tasks`, `win_conditions`, `radio_plan`,
`tactical_graphics`) are the domain boundary the API and the SPA import. They are not conveniences
— they are the surface. Moving one is a consumer change.

Gated on `scenario`, this crate's default feature and the only tier `website-api` enables. Folded
in from `website-mission-core/src/mission` at T-0xx Phase 2A.

## Boundaries

This module does not access browser APIs, graphics devices, or Leptos state. Callers provide domain
data explicitly. Wire keys, numeric conversions, authored order, diagnostics, and resource
substitutions are preserved. Production Rust files remain under 500 lines; test files remain under
1,000 lines.
