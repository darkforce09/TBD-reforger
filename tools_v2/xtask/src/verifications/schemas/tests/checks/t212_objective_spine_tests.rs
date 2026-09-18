use super::{count_mod_readers, definition_path, read_json, repo_root};
use std::path::PathBuf;

/// `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives` — the lane that owns objectives.
fn objectives_lane() -> PathBuf {
    repo_root()
        .expect("repo root")
        .join("apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives")
}

/// Property names of one `#/$defs/<name>` object, in schema order.
fn def_properties(name: &str) -> Vec<String> {
    let root = repo_root().expect("repo root");
    let schema = read_json(&definition_path(&root, "mission.schema.json")).expect("schema");
    let props = schema
        .pointer(&format!("/$defs/{name}/properties"))
        .and_then(|v| v.as_object())
        .unwrap_or_else(|| panic!("$defs/{name}/properties must exist"));
    props.keys().cloned().collect()
}

/// Every property of the typed-objective spine has at least one identifier in the lane.
#[test]
fn objective_spine_is_read_in_the_objectives_lane() {
    let lane = objectives_lane();
    assert!(lane.is_dir(), "objectives lane missing: {}", lane.display());

    let mut names = def_properties("objective");
    names.extend(def_properties("objectiveFraming"));
    assert!(
        names.len() >= 9,
        "the spine collapsed to {} properties — check $defs/objective",
        names.len()
    );

    let mut unread = Vec::new();
    for name in &names {
        if count_mod_readers(&lane, name).expect("scan objectives lane") == 0 {
            unread.push(name.clone());
        }
    }

    assert!(
        unread.is_empty(),
        "$defs/objective properties with NO identifier under {}: {unread:?}\n\
         JsonLoadContext binds by member name, so an unspelled property is unreadable. \
         Either the reader lost a field, or the schema grew one T-212's reader has not \
         taken up yet.",
        lane.display()
    );
}

/// Non-vacuity: the scan must be capable of returning zero on this very corpus, or the test
/// above would pass no matter what the lane contains.
#[test]
fn the_lane_scan_can_still_report_zero() {
    let lane = objectives_lane();
    assert_eq!(
        count_mod_readers(&lane, "zzNotAnObjectivePropertyZZ").expect("scan"),
        0,
        "a word that appears nowhere must count 0"
    );
    assert!(
        count_mod_readers(&lane, "TBD_Objective").expect("scan") > 0,
        "the scan must actually be reading the lane's .c files"
    );
}
