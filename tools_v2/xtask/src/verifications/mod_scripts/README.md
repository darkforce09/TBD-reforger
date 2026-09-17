# Enfusion Mod Script Verifications (`verifications/mod_scripts`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Static analysis checks enforcing code contracts and layout syntax for the Enfusion game mod.

---

## Verifications

- **`destroy_target_diagnostics.rs`** (formerly `gate_t437.rs`): Proves destroy-objective entity diagnostics handle missing target entities cleanly.
- **`player_identity_comments.rs`** (formerly `mod_comment_gates.rs` & `gate_t296.rs`): Verifies comment contracts and `#tbd link` implementation claims in Enfusion script classes.
- **`mission_loader_rest_size.rs`** (formerly `gate_t456.rs`): Proves `TBD_MissionLoader.c` enforces HTTP body size limits before parsing JSON payloads.
- **`enfusion_ui_layouts.rs`** (formerly `gate_ui_layouts.rs` & `gate_ui_layouts_awk.rs`): Validates `.layout` syntax, brace nesting, slot types, and widget naming contracts.
- **`spawn_determinism.rs`** (formerly `gate_tbd_spawn_determinism.rs`): Proves player and object spawn placements are mathematically deterministic for a given seed.
