use super::{Rules, reclassify_rows, rule_for_kind_class};
use serde_json::{Value, json};

fn rules(extra: Vec<Value>) -> Rules {
    let mut rs = vec![json!({
        "kind": "building", "class": "hut",
        "match": { "resourceNameContains": ["/Hut"] },
        "ai": { "summary": "a hut", "taxonomyPath": "building/hut", "confidence": 0.9 },
        "spatial": { "model": "obb", "pivot": "center" },
        "gameplay": { "cover": { "type": "full" } },
        "render": { "iconKey": "hut" }
    })];
    rs.extend(extra);
    Rules {
        doc: json!({
            "rules": rs,
            "fallback": {
                "kind": "prop", "class": "unknown",
                "ai": { "summary": "?", "taxonomyPath": "prop/unknown" },
                "spatial": { "model": "obb", "pivot": "center", "fallback": true },
                "gameplay": {}
            }
        }),
    }
}

fn wreck_rule() -> Value {
    json!({
        "kind": "vehicle", "class": "armor",
        "match": { "resourceNameContains": ["Props/Wrecks/"] },
        "ai": { "summary": "a wreck", "taxonomyPath": "vehicle/armor", "confidence": 0.8 },
        "spatial": { "model": "obb", "pivot": "center", "wreck": true },
        "gameplay": {}
    })
}

fn committed() -> Vec<Value> {
    vec![
        json!({
            "prefabId": 0, "resourceName": "{A}Prefabs/Hut.et",
            "kind": "building", "class": "hut", "label": "Hut",
            "ai": { "needsReview": false },
            "spatial": { "model": "obb", "pivot": "center", "halfExtentsM": { "x": 4.0 } },
            "gameplay": { "cover": { "type": "full" } }
        }),
        json!({
            "prefabId": 1, "resourceName": "{B}Prefabs/Props/Wrecks/T62.et",
            "kind": "prop", "class": "unknown", "label": "T62",
            "ai": { "needsReview": true },
            "spatial": { "model": "obb", "pivot": "center", "fallback": true },
            "gameplay": {}
        }),
    ]
}

/// GREEN: rules unchanged since the catalogue was built → no drift, nothing to do.
#[test]
fn no_drift_when_rules_match_the_catalogue() {
    let (rows, rep) = reclassify_rows(&rules(vec![]), &committed()).expect("ok");
    assert!(rep.is_clean(), "unexpected drift: {:?}", rep.drift);
    assert_eq!(rows.len(), 2);
    assert_eq!(rep.unclassified_before, 1);
    assert_eq!(rep.unclassified_after, 1);
    assert!(rep.new_kinds.is_empty());
}

/// RED: this is T-244's exact shape — append a rule, and the committed artifact is stale.
#[test]
fn appending_a_rule_reports_drift_and_a_new_census_bucket() {
    let (rows, rep) = reclassify_rows(&rules(vec![wreck_rule()]), &committed()).expect("ok");
    assert_eq!(rep.drift.len(), 1, "{:?}", rep.drift);
    let d = &rep.drift[0];
    assert_eq!(
        (d.prefab_id, &*d.old_kind, &*d.new_kind),
        (1, "prop", "vehicle")
    );
    assert_eq!(rows[1]["kind"], json!("vehicle"));
    assert_eq!(rows[1]["class"], json!("armor"));
    assert_eq!(rep.new_kinds, vec!["vehicle".to_string()]);
    assert_eq!(rep.unclassified_after, 0, "the wreck is no longer unknown");
    // Identity survives a reclassification; only the classification lane moves.
    assert_eq!(rows[1]["prefabId"], json!(1));
    assert_eq!(rows[1]["label"], json!("T62"));
    assert_eq!(rows[1]["ai"]["needsReview"], json!(false));
}

/// A measured OBB is not recoverable from the repo, so it must survive a kind change.
#[test]
fn measured_spatial_is_preserved_and_template_spatial_is_reclaimed() {
    let (rows, _) = reclassify_rows(&rules(vec![wreck_rule()]), &committed()).expect("ok");
    // Row 0 carries a measured halfExtents → untouched even though the rule has no such key.
    assert_eq!(rows[0]["spatial"]["halfExtentsM"]["x"], json!(4.0));
    // Row 1's committed spatial IS the fallback template → re-templated from the new rule.
    assert_eq!(rows[1]["spatial"]["wreck"], json!(true));
    assert!(rows[1]["spatial"].get("fallback").is_none());
}

/// The house defect, refused: an empty rules file classifies nothing, which would re-stamp
/// every row `prop/unknown` and report a confident, enormous, wrong drift.
#[test]
fn empty_rules_are_refused_not_reported_as_massive_drift() {
    let empty = Rules {
        doc: json!({ "rules": [], "fallback": { "kind": "prop", "class": "unknown" } }),
    };
    let err = reclassify_rows(&empty, &committed()).expect_err("must refuse");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing empty write (reclassify catalogue)"),
        "{msg}"
    );
    assert!(msg.contains("no prefab matched any rule"), "{msg}");
}

#[test]
fn empty_catalogue_is_refused() {
    let err = reclassify_rows(&rules(vec![]), &[]).expect_err("must refuse");
    assert!(format!("{err:#}").contains("zero prefab rows"), "{err:#}");
}

#[test]
fn rule_lookup_falls_back_when_no_rule_owns_the_pair() {
    let r = rules(vec![]);
    assert_eq!(
        rule_for_kind_class(&r, "building", "hut")["class"],
        json!("hut")
    );
    assert_eq!(
        rule_for_kind_class(&r, "prop", "unknown")["spatial"]["fallback"],
        json!(true),
        "an unclaimed kind/class pair must resolve to the fallback template"
    );
}
