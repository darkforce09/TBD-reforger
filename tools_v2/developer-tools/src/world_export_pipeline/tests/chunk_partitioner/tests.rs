/// T-946/T-960 — the road census comes from the committed roads.json.gz, and it is 887.
///
/// Both slots used to be hardcoded (`segments` 0, `byRoadClass` {}) because roads export as
/// prefab-less `RoadEntity` rows that classification never sees, so no rule edit could ever
/// make them non-zero. Five places in the repo also claimed 888; the file ships 887.
#[test]
fn the_road_census_reads_the_committed_roads_file() {
    let objects =
        crate::browser_testing::server::repo_root().join("packages/map-assets/everon/objects");
    if !objects.join("roads.json.gz").is_file() {
        panic!("everon roads.json.gz missing at {}", objects.display());
    }
    let (segments, by_class) = road_census(&objects).expect("census");
    println!("── segments {segments} · byClass {by_class:?}");
    assert_eq!(segments, 887, "roads.json.gz ships 887 segments, not 888");
    assert_eq!(
        by_class
            .values()
            .filter_map(|v| v["instances"].as_u64())
            .sum::<u64>(),
        segments,
        "every segment lands in exactly one class"
    );
    assert_eq!(by_class.len(), 5, "five road classes: {by_class:?}");
}

/// T-090.12.1 — the P5 filter admits T-244's `vehicle` kind: without it a rebuild from the
/// staged export silently dropped the 13 wreck prefabs (176 instances) the committed
/// catalogue carries since T-594, and E6 could never have matched the committed artifacts.
#[test]
fn p5_admits_the_vehicle_lane() {
    let p5 = super::phase_kinds("P5_props").unwrap();
    assert!(p5.contains(&"vehicle"), "{p5:?}");
    for k in super::super::INSTANCE_KINDS {
        if k == "road" || k == "utility" {
            continue;
        }
        assert!(
            p5.contains(&k),
            "P5 must admit every instance kind: missing {k}"
        );
    }
    assert!(!super::phase_kinds("P4_rocks").unwrap().contains(&"vehicle"));
}

use super::*;
use std::fs;

#[test]
fn non_density_phase_must_not_clear() {
    assert!(!may_clear_density_dir(false));
    assert!(may_clear_density_dir(true));
}

#[test]
fn clear_density_skips_when_not_rebuilding() {
    let dir = std::env::temp_dir().join(format!("t378-density-skip-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let marker = dir.join("12_34.bin");
    fs::write(&marker, b"keep-me").unwrap();

    let cleared = clear_density_dir_if_rebuilding(&dir, false).unwrap();
    assert!(!cleared, "non-rebuild must not attempt clear");
    assert!(
        marker.exists(),
        "non-density phase must leave density bins intact"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn clear_density_wipes_when_rebuilding() {
    let dir = std::env::temp_dir().join(format!("t378-density-wipe-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let marker = dir.join("12_34.bin");
    fs::write(&marker, b"gone").unwrap();

    let cleared = clear_density_dir_if_rebuilding(&dir, true).unwrap();
    assert!(cleared);
    assert!(
        !dir.exists(),
        "intentional density rebuild must remove the dir"
    );
}

#[test]
fn refuse_empty_catalog_write_contract() {
    // T-537 Class-R: empty kept/prefab set must refuse before objects/ wipe.
    let err = super::super::refuse_empty_write(
        "build-world-objects catalog",
        true,
        "zero kept instances/prefabs — refusing empty objects/ overwrite",
    )
    .expect_err("must refuse");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing empty write (build-world-objects catalog)"),
        "{msg}"
    );
}
