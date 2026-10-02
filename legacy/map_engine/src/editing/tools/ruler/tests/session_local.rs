//! Role: the guard that a ruler measurement stays a MEASUREMENT.
//! Position: `editing/tools/ruler/tests` in the map engine.
//! Signals & state: this module's own source text.
//! Invariants: proven on SCRUBBED code — comments and strings blanked — so a mention of a document
//! mutator in prose cannot satisfy the guard.

use crate::source_scrub::strip_rust_lexical_noise;

/// A ruler reading is a measurement, not mission content. Nothing here may reach the authored
/// document, so a reload leaves the map clean and the compiled payload byte-identical.
#[test]
fn the_ruler_never_writes_the_document() {
    let code = strip_rust_lexical_noise(concat!(
        include_str!("../chain.rs"),
        include_str!("../host_registry.rs"),
        include_str!("../leg.rs"),
        include_str!("../mod.rs"),
        include_str!("../projection.rs"),
        include_str!("../tool_mode.rs"),
    ));
    for banned in [
        "MissionDocCore",
        "move_entities",
        "add_slot",
        "data::store",
        "hydrate",
        "after_local_edit",
        "editor_ops",
    ] {
        assert!(
            !code.contains(banned),
            "a ruler reading is a measurement, not mission content — found document-write token \
             `{banned}` in the ruler"
        );
    }
}
