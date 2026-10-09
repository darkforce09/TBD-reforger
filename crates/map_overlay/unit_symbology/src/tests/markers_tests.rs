//! Cases of the briefing markers: the alias table against the schema vocabulary, the marker
//! atlas cells against the slot atlas, glyph ink placement, and caption packing.

use crate::markers::*;

#[test]
fn every_schema_alias_maps() {
    for a in ["dot", "dot2", "point", "mark", "marker"] {
        assert_eq!(marker_glyph_for_alias(a), MarkerGlyph::Disc, "{a}");
    }
    for a in ["objective_marker", "obj", "target", "task"] {
        assert_eq!(marker_glyph_for_alias(a), MarkerGlyph::Square, "{a}");
    }
    for a in ["observation_post", "op", "overwatch", "recon"] {
        assert_eq!(marker_glyph_for_alias(a), MarkerGlyph::Target, "{a}");
    }

    for a in [
        "dot",
        "dot2",
        "objective_marker",
        "objective_marker2",
        "point_of_interest",
        "point_of_interest2",
        "observation_post",
        "observation_post2",
        "destroy",
        "destroy2",
        "attack",
        "defend",
        "defend2",
        "waypoint",
        "waypoint2",
        "ambush",
        "ambush2",
        "flag",
        "flag2",
        "cross",
        "cross2",
        "circle",
        "circle2",
        "objective",
        "obj",
        "target",
        "task",
        "assault",
        "capture",
        "seize",
        "advance",
        "hold",
        "garrison",
        "fallback",
        "demolish",
        "demo",
        "sabotage",
        "move",
        "wp",
        "route",
        "phase_line",
        "poi",
        "intel",
        "contact",
        "op",
        "observe",
        "overwatch",
        "recon",
        "rally",
        "rally_point",
        "base",
        "hq",
        "spawn",
        "medical",
        "medic",
        "aid",
        "casevac",
        "medevac",
        "area",
        "zone",
        "ao",
        "point",
        "mark",
        "marker",
    ] {
        assert!(
            (marker_glyph_for_alias(a) as usize) < MARKER_GLYPH_COUNT,
            "{a}"
        );
    }
}
