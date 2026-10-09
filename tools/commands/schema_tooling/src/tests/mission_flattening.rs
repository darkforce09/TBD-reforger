use super::*;
use serde_json::json;
use std::io::Write;

fn write_temp_mission(name: &str, value: &Value) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("t383-flatten-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("tmpdir");
    let path = dir.join(name);
    let mut f = fs::File::create(&path).expect("create");
    f.write_all(serde_json::to_string_pretty(value).unwrap().as_bytes())
        .unwrap();
    f.write_all(b"\n").unwrap();
    path
}

/// Minimal mission with orbat that produces one slot id matching a prior slot.
fn mission_with_prior_loadout_uid() -> Value {
    json!({
        "schemaVersion": "1.1",
        "meta": {"title": "t383"},
        "factions": [],
        "orbat": {
            "blufor": {
                "groups": [{
                    "callsign": "Ranger",
                    "roles": [{"slot": "SL", "kit": "kit:us_sl", "count": 1}]
                }]
            }
        },
        "zones": [],
        "flow": {},
        "winConditions": {},
        "slots": [{
            "id": "blufor:Ranger:SL:0",
            "uid": "keep-me",
            "faction": "blufor",
            "groupCallsign": "Ranger",
            "role": "SL",
            "kit": "kit:us_sl",
            "x": 0.0,
            "z": 0.0,
            "headingDeg": 0.0,
            "loadout": {"gear": {"primary": "Rifle.et"}}
        }]
    })
}

#[test]
fn flatten_in_place_preserves_loadout_uid_and_schema() {
    let path = write_temp_mission("preserve.json", &mission_with_prior_loadout_uid());
    let before = fs::read_to_string(&path).unwrap();
    flatten_orbat_slots(path.to_str().unwrap(), true).expect("in-place must succeed");
    let after: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(after["schemaVersion"], "1.1");
    let slot = &after["slots"][0];
    assert_eq!(slot["uid"], "keep-me");
    assert_eq!(slot["loadout"]["gear"]["primary"], "Rifle.et");
    // Must have rewritten (coordinates change) but not dropped fields.
    assert_ne!(before, fs::read_to_string(&path).unwrap());
}

#[test]
fn flatten_in_place_preserves_schema_version_1_0() {
    let mut m = mission_with_prior_loadout_uid();
    m["schemaVersion"] = json!("1.0");
    // 1.0 fixture has no slots requirement — clear slots so preserve path is schema-only.
    m["slots"] = json!([]);
    let path = write_temp_mission("sv10.json", &m);
    flatten_orbat_slots(path.to_str().unwrap(), true).expect("ok");
    let after: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        after["schemaVersion"], "1.0",
        "in-place must not force-stamp schemaVersion 1.1 over deliberate 1.0"
    );
    assert!(!after["slots"].as_array().unwrap().is_empty());
}

#[test]
fn flatten_in_place_refuses_lossy_loadout_drop() {
    // Prior slot id does NOT match what flatten will emit → loadout/uid would be dropped.
    let mut m = mission_with_prior_loadout_uid();
    m["slots"][0]["id"] = json!("blufor:Other:SL:0");
    let path = write_temp_mission("lossy.json", &m);
    let before = fs::read_to_string(&path).unwrap();
    let err =
        flatten_orbat_slots(path.to_str().unwrap(), true).expect_err("must refuse lossy in-place");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing empty write") && (msg.contains("loadout") || msg.contains("uid")),
        "expected lossy refuse, got: {msg}"
    );
    assert_eq!(
        before,
        fs::read_to_string(&path).unwrap(),
        "lossy refuse must not overwrite the file"
    );
}

#[test]
fn flatten_in_place_refuses_empty_slots_overwrite() {
    // Orbat empty → 0 slots, but prior had slots → refuse.
    let mut m = mission_with_prior_loadout_uid();
    m["orbat"] = json!({});
    let path = write_temp_mission("empty-slots.json", &m);
    let before = fs::read_to_string(&path).unwrap();
    let err =
        flatten_orbat_slots(path.to_str().unwrap(), true).expect_err("must refuse empty overwrite");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing empty write") && msg.contains("empty slots"),
        "expected empty-slots refuse, got: {msg}"
    );
    assert_eq!(before, fs::read_to_string(&path).unwrap());
}

// ─── stdout path shares preserve/refuse (not a silent lossy preview) ───
//
// MAJOR: preserve Class-R must pin `flatten_orbat_slots(..., false)` (stdout
// entrypoint), NOT `apply_flatten_orbat_slots` alone. A post-apply stdout-only
// `mission["schemaVersion"] = "1.1"` stamp must RED these pins.

#[test]
fn flatten_stdout_preserves_loadout_uid_and_schema() {
    let path = write_temp_mission("stdout-preserve.json", &mission_with_prior_loadout_uid());
    let m = flatten_stdout_json(path.to_str().unwrap()).expect("stdout entrypoint must succeed");
    assert_eq!(m["schemaVersion"], "1.1");
    let slot = &m["slots"][0];
    assert_eq!(slot["uid"], "keep-me");
    assert_eq!(slot["loadout"]["gear"]["primary"], "Rifle.et");
    // File untouched on stdout path.
    let on_disk: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(on_disk["slots"][0]["uid"], "keep-me");
    assert_eq!(
        on_disk["slots"][0]["x"],
        mission_with_prior_loadout_uid()["slots"][0]["x"],
        "stdout must not rewrite the input file"
    );
}

#[test]
fn flatten_stdout_preserves_schema_version_1_0() {
    let mut m = mission_with_prior_loadout_uid();
    m["schemaVersion"] = json!("1.0");
    m["slots"] = json!([]);
    let path = write_temp_mission("stdout-sv10.json", &m);
    let after =
        flatten_stdout_json(path.to_str().unwrap()).expect("stdout entrypoint must succeed");
    assert_eq!(
        after["schemaVersion"], "1.0",
        "stdout entrypoint must not force-stamp schemaVersion 1.1 over deliberate 1.0"
    );
    assert!(!after["slots"].as_array().unwrap().is_empty());
}

/// Defense-in-depth: apply-level still covered, but must not be the only stdout pin.
#[test]
fn flatten_apply_preserves_schema_version_1_0_defense() {
    let mut m = mission_with_prior_loadout_uid();
    m["schemaVersion"] = json!("1.0");
    m["slots"] = json!([]);
    apply_flatten_orbat_slots(&mut m, "flatten-orbat-slots (stdout) probe").expect("ok");
    assert_eq!(m["schemaVersion"], "1.0");
}

#[test]
fn flatten_stdout_refuses_lossy_loadout_drop() {
    // Class-R: silent drop on stdout must RED (same refuse as --in-place).
    let mut m = mission_with_prior_loadout_uid();
    m["slots"][0]["id"] = json!("blufor:Other:SL:0");
    let path = write_temp_mission("stdout-lossy.json", &m);
    let before = fs::read_to_string(&path).unwrap();
    let err =
        flatten_orbat_slots(path.to_str().unwrap(), false).expect_err("must refuse lossy stdout");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing empty write")
            && msg.contains("stdout")
            && (msg.contains("loadout") || msg.contains("uid")),
        "expected lossy stdout refuse, got: {msg}"
    );
    assert_eq!(
        before,
        fs::read_to_string(&path).unwrap(),
        "lossy stdout refuse must not touch the file"
    );
}

#[test]
fn flatten_stdout_refuses_empty_slots() {
    let mut m = mission_with_prior_loadout_uid();
    m["orbat"] = json!({});
    let path = write_temp_mission("stdout-empty-slots.json", &m);
    let before = fs::read_to_string(&path).unwrap();
    let err =
        flatten_orbat_slots(path.to_str().unwrap(), false).expect_err("must refuse empty stdout");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing empty write")
            && msg.contains("stdout")
            && msg.contains("empty slots"),
        "expected empty-slots stdout refuse, got: {msg}"
    );
    assert_eq!(before, fs::read_to_string(&path).unwrap());
}
