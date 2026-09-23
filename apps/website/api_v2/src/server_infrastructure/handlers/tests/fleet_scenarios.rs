//! The registry's key and resource shapes, which the database constraints repeat.

use super::{valid_scenario_id, valid_terrain_key};

#[test]
fn terrain_keys_are_the_compiler_slug_shape() {
    for key in ["everon", "arland", "f_1985", "kunar_province"] {
        assert!(valid_terrain_key(key), "{key}");
    }
    for key in [
        "",
        "Everon",
        "1everon",
        "ever-on",
        "ever on",
        &"a".repeat(65),
    ] {
        assert!(!valid_terrain_key(key), "{key}");
    }
}

#[test]
fn scenario_ids_are_header_resources() {
    assert!(valid_scenario_id(
        "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf"
    ));
    for id in [
        "Missions/TBD_Dev_POC.conf",
        "{69a85365fc09e2ca}Missions/TBD_Dev_POC.conf",
        "{69A85365FC09E2}Missions/TBD_Dev_POC.conf",
        "{69A85365FC09E2CA}Missions/TBD Dev.conf",
        "{69A85365FC09E2CA}Missions/TBD_Dev_POC.ent",
        "{69A85365FC09E2CA}.conf",
    ] {
        assert!(!valid_scenario_id(id), "{id}");
    }
}
