//! Role: the guard that a sight check stays a MEASUREMENT.
//! Position: `editing/tools/line_of_sight/tests` in the map engine.
//! Signals & state: this module's own source text.
//! Invariants: proven on SCRUBBED code — comments and strings blanked — so a mention of a document
//! mutator in prose cannot satisfy the guard.

use crate::source_scrub::strip_rust_lexical_noise;

/// A line-of-sight result is session-local overlay state: the operator asked a question about the
/// world, not about the mission. Nothing in this module may reach the authored document, so a
/// reload leaves the map clean and the compiled payload byte-identical.
#[test]
fn the_line_of_sight_tool_never_writes_the_document() {
    let code = strip_rust_lexical_noise(concat!(
        include_str!("../capture.rs"),
        include_str!("../host_registry.rs"),
        include_str!("../object_verdict.rs"),
        include_str!("../object_wash.rs"),
        include_str!("../projection.rs"),
        include_str!("../terrain_survey.rs"),
        include_str!("../terrain_verdict.rs"),
        include_str!("../viewshed_texture.rs"),
        include_str!("../wash_palette.rs"),
    ));
    for banned in [
        "MissionDocCore",
        "move_entities",
        "add_slot",
        "data::store",
        "hydrate",
        "after_local_edit",
    ] {
        assert!(
            !code.contains(banned),
            "a sight check is a measurement, not mission content — found document-write token \
             `{banned}` in the line-of-sight tool"
        );
    }
}
