//! Role: classify tests.
//! Position: `environment/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::environment::classify::*;
use serde_json::json;

#[test]
fn render_class_truth_table() {
    assert_eq!(
        render_class_for_prefab("building", "residential"),
        Some("building")
    );
    assert_eq!(render_class_for_prefab("water", "pier"), Some("building"));
    assert_eq!(render_class_for_prefab("water", "dock"), Some("building"));
    assert_eq!(render_class_for_prefab("water", "buoy"), None);
    assert_eq!(render_class_for_prefab("tree", "conifer"), Some("tree"));
    assert_eq!(
        render_class_for_prefab("vegetation", "bush"),
        Some("vegetation")
    );
    assert_eq!(
        render_class_for_prefab("rock", "boulder"),
        Some("rockLarge")
    );
    assert_eq!(render_class_for_prefab("prop", "barrel"), Some("prop"));
    assert_eq!(render_class_for_prefab("utility", "pole"), Some("prop"));
    assert_eq!(render_class_for_prefab("mystery", "x"), None);
}

#[test]
fn vehicle_kind_draws_as_prop() {
    for cls in ["car", "truck", "armor", "boat", "unknown"] {
        assert_eq!(
            render_class_for_prefab("vehicle", cls),
            Some("prop"),
            "vehicle/{cls} must render; None means NO_CLASS = invisible + unpickable"
        );
        assert_ne!(
            class_code(render_class_for_prefab("vehicle", cls).unwrap()),
            NO_CLASS,
            "vehicle/{cls} resolved to the unclassified sentinel"
        );
    }

    assert_eq!(RENDER_CLASS_CODES.len(), 5);
    assert_eq!(class_code("prop"), 3);
}

#[test]
fn class_codes_match_wire_order() {
    assert_eq!(class_code("building"), 0);
    assert_eq!(class_code("tree"), 1);
    assert_eq!(class_code("vegetation"), 2);
    assert_eq!(class_code("prop"), 3);
    assert_eq!(class_code("rockLarge"), 4);
    assert_eq!(class_code("nope"), NO_CLASS);
}

#[test]
fn narrow_instance_row_accepts_and_defaults() {
    assert_eq!(
        narrow_instance_row(&json!([9, 512.0, 700.25, 41.3, 90])),
        Some((9.0, 512.0, 700.25, 41.3, 90.0))
    );

    assert_eq!(
        narrow_instance_row(&json!([3, 1.0, 2.0])),
        Some((3.0, 1.0, 2.0, 0.0, 0.0))
    );

    assert_eq!(
        narrow_instance_row(&json!([3, 1.0, 2.0, "x", "y"])),
        Some((3.0, 1.0, 2.0, 0.0, 0.0))
    );
}

#[test]
fn narrow_instance_row_v2_reads_trailers_and_defaults() {
    assert_eq!(
        narrow_instance_row_v2(&json!([9, 512.0, 700.25, 41.3, 90])),
        Some(InstanceRowV2 {
            pid: 9.0,
            x: 512.0,
            y: 700.25,
            z: 41.3,
            rot: 90.0,
            pitch: 0.0,
            roll: 0.0,
            scale: 1.0
        })
    );

    assert_eq!(
        narrow_instance_row_v2(&json!([14, 900.0, 950.0, 52.0, 47.25, -3.5, 1.25, 1.15])),
        Some(InstanceRowV2 {
            pid: 14.0,
            x: 900.0,
            y: 950.0,
            z: 52.0,
            rot: 47.25,
            pitch: -3.5,
            roll: 1.25,
            scale: 1.15
        })
    );

    let r = narrow_instance_row_v2(&json!([14, 1.0, 2.0, 3.0, 4.0, "x", null, 0])).unwrap();
    assert_eq!((r.pitch, r.roll, r.scale), (0.0, 0.0, 1.0));

    assert_eq!(narrow_instance_row_v2(&json!([9, 1.0])), None);
    assert_eq!(
        narrow_instance_row_v2(&json!([3, 1.0, 2.0])).map(|r| (r.z, r.rot, r.scale)),
        Some((0.0, 0.0, 1.0))
    );
}

#[test]
fn narrow_instance_row_rejects() {
    assert_eq!(narrow_instance_row(&json!("garbage")), None);
    assert_eq!(narrow_instance_row(&json!([9, 1.0])), None);
    assert_eq!(narrow_instance_row(&json!(["x", 1.0, 2.0])), None);
    assert_eq!(narrow_instance_row(&json!([9, "x", 2.0])), None);
    assert_eq!(narrow_instance_row(&json!([9, 1.0, "y"])), None);
}
