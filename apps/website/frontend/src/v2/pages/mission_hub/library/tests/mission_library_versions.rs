//! The guards on the version comparison behind the dossier, and on the document upload it
//! feeds: the differ battery and the upload pipeline.

use super::{
    diff_summary_lines, next_semver, oversize_refusal, parse_uploaded_document,
    unwrap_export_envelope, upload_failure, UPLOAD_MAX_BYTES,
};
use serde_json::json;
// ───────────────────────────────────────────────────────────────────────────────────────
// The version differ.
//
// NON-VACUITY IS THE WHOLE POINT HERE. A differ that answers "nothing changed" to every
// question renders perfectly and passes any test that only asserts a section appears. So every
// test below names the SPECIFIC change it expects, and `differ_is_not_vacuous_on_identical_input`
// is its paired control: the same helper pair must also report *silence* when there is nothing
// to say, or "reports everything" would pass just as cheaply as "reports nothing".
// ───────────────────────────────────────────────────────────────────────────────────────
use super::{
    census_line, classify_row, diff_mission_payloads, version_census, RowChange, DIFF_SAMPLE_CAP,
};
use serde_json::Value;

/// Mission "Bridgehead at Levie" v0.1.0 — three slots in one squad, one objective, one
/// loadout. Shapes copied from `map_engine_core::mission::flatten`'s own fixtures so the
/// differ is tested against the rows this editor really writes.
fn levie_v1() -> Value {
    json!({
        "schemaVersion": 1,
        "title": "Bridgehead at Levie",
        "map": { "terrain": "everon", "bounds": [0, 0, 12800, 12800] },
        "environment": { "timeOfDay": "dawn", "weather": "clear" },
        "objectives": [{ "id": "o1", "name": "Seize the bridge" }],
        "markers": [],
        "vehicles": [],
        "entities": [],
        "loadouts": { "l1": { "primary": "L85A3" } },
        "editor": {
            "factions": [{ "id": "f1", "key": "BLUFOR", "name": "US Army" }],
            "squads": [{ "id": "sq1", "factionId": "f1", "callsign": "Alpha", "name": "Alpha 1-1" }],
            "slots": [
                { "id": "s1", "name": "Alpha SL",  "squadId": "sq1", "index": 0, "role": "SL",
                  "position": { "x": 100.0, "y": 200.0, "z": 0.0, "rotation": 0.0 } },
                { "id": "s2", "name": "Alpha TL",  "squadId": "sq1", "index": 1, "role": "TL",
                  "position": { "x": 110.0, "y": 200.0, "z": 0.0, "rotation": 0.0 } },
                { "id": "s3", "name": "Alpha RFL", "squadId": "sq1", "index": 2, "role": "RFL",
                  "position": { "x": 120.0, "y": 200.0, "z": 0.0, "rotation": 0.0 } }
            ],
            "editorLayers": [{ "id": "L1", "name": "Default" }]
        }
    })
}

/// v0.2.0 — FIVE named changes against [`levie_v1`], and nothing else:
///   1. `s1` **edited** (role SL → PL)
///   2. `s2` **moved** (position only)
///   3. `s3` **removed**
///   4. `s4` **added**
///   5. terrain everon → arland, `environment.timeOfDay` dawn → dusk, `environment.weather`
///      **deleted**
/// The slot array is also written in a different order than v1, so any test that passes here
/// has also proved the diff is order-insensitive.
fn levie_v2() -> Value {
    json!({
        "schemaVersion": 1,
        "title": "Bridgehead at Levie",
        "map": { "terrain": "arland", "bounds": [0, 0, 12800, 12800] },
        "environment": { "timeOfDay": "dusk" },
        "objectives": [{ "id": "o1", "name": "Seize the bridge" }],
        "markers": [],
        "vehicles": [],
        "entities": [],
        "loadouts": { "l1": { "primary": "L85A3" } },
        "editor": {
            "factions": [{ "id": "f1", "key": "BLUFOR", "name": "US Army" }],
            "squads": [{ "id": "sq1", "factionId": "f1", "callsign": "Alpha", "name": "Alpha 1-1" }],
            "slots": [
                { "id": "s1", "name": "Alpha SL",  "squadId": "sq1", "index": 0, "role": "PL",
                  "position": { "x": 100.0, "y": 200.0, "z": 0.0, "rotation": 0.0 } },
                { "id": "s2", "name": "Alpha TL",  "squadId": "sq1", "index": 1, "role": "TL",
                  "position": { "x": 480.5, "y": 902.5, "z": 0.0, "rotation": 90.0 } },
                { "id": "s4", "name": "Alpha MED", "squadId": "sq1", "index": 3, "role": "MED",
                  "position": { "x": 130.0, "y": 200.0, "z": 0.0, "rotation": 0.0 } }
            ],
            "editorLayers": [{ "id": "L1", "name": "Default" }]
        }
    })
}

fn slots_delta(d: &super::MissionDiff) -> &super::CollectionDelta {
    d.collections
        .iter()
        .find(|c| c.label == "Slots")
        .expect("Slots must be a diffed collection")
}

/// **The non-vacuity test.** Five known changes in, five specifically-named changes out.
/// Every number here is asserted exactly — `assert!(delta.changed() > 0)` would pass for a
/// differ that mislabelled all four slot events as the same thing.
#[test]
fn differ_names_the_specific_change_between_two_versions() {
    let d = diff_mission_payloads(&levie_v1(), &levie_v2());
    let slots = slots_delta(&d);

    assert_eq!(slots.added, 1, "s4 was added: {slots:#?}");
    assert_eq!(slots.removed, 1, "s3 was removed: {slots:#?}");
    assert_eq!(slots.moved, 1, "s2 changed only its position: {slots:#?}");
    assert_eq!(slots.edited, 1, "s1 changed its role: {slots:#?}");
    assert_eq!(slots.unchanged, 0, "no slot survived untouched: {slots:#?}");
    assert_eq!(slots.a_rows, 3);
    assert_eq!(slots.b_rows, 3);

    // The sample lines name the rows by the label an author would recognise, in document
    // order on the new side, then removals in document order on the old side.
    assert_eq!(
        slots.samples,
        vec![
            "~ Alpha SL edited".to_string(),
            "~ Alpha TL moved".to_string(),
            "+ Alpha MED".to_string(),
            "− Alpha RFL".to_string(),
        ],
        "the differ must name WHICH rows changed, not just how many"
    );

    // Untouched collections must stay silent — a differ that flags everything is as useless
    // as one that flags nothing.
    for c in &d.collections {
        if c.label != "Slots" {
            assert!(
                c.is_unchanged(),
                "{} reported a change and nothing in it changed: {c:#?}",
                c.label
            );
        }
    }

    // Scalars, including the DELETED environment key — the case a new-side-only walk misses.
    let fields: Vec<(String, String, String)> = d
        .fields
        .iter()
        .map(|f| (f.label.clone(), f.from.clone(), f.to.clone()))
        .collect();
    assert_eq!(
        fields,
        vec![
            ("Terrain".into(), "everon".into(), "arland".into()),
            (
                "Environment · timeOfDay".into(),
                "dawn".into(),
                "dusk".into()
            ),
            ("Environment · weather".into(), "clear".into(), "—".into()),
        ],
        "scalar changes must name the field and both values"
    );
}

/// The paired control. Without this, "report every field as changed" would satisfy the test
/// above; with it, the differ has to be right in both directions.
#[test]
fn differ_is_not_vacuous_on_identical_input() {
    let d = diff_mission_payloads(&levie_v1(), &levie_v1());
    assert!(
        d.is_empty(),
        "a version compared with itself has no changes: {d:#?}"
    );
    assert_eq!(d.changed_collections().count(), 0);
    assert_eq!(slots_delta(&d).unchanged, 3);
}

/// **The claim that justifies a structural diff over a textual one.** `compile_payload`
/// re-emits collections in whatever order `entityOrder` held, so a re-save can permute the
/// arrays with no authorial change. A line diff would scream; this must not.
#[test]
fn reordering_rows_is_not_a_change() {
    let v1 = levie_v1();
    let mut v2 = levie_v1();
    let slots = v2["editor"]["slots"].as_array_mut().unwrap();
    slots.reverse();
    assert_ne!(
        serde_json::to_string(&v1).unwrap(),
        serde_json::to_string(&v2).unwrap(),
        "the two payloads must differ TEXTUALLY, or this test proves nothing"
    );
    let d = diff_mission_payloads(&v1, &v2);
    assert!(
        d.is_empty(),
        "permuting the emit order is not an edit — a textual diff would have reported 3 \
             changed rows here: {d:#?}"
    );
}

/// Moving a slot is not the same event as re-roling it, and the differ must not collapse them.
#[test]
fn position_only_change_is_moved_not_edited() {
    let a = json!({ "id": "s1", "role": "SL", "position": { "x": 1.0, "y": 2.0 } });
    let b = json!({ "id": "s1", "role": "SL", "position": { "x": 9.0, "y": 2.0 } });
    assert_eq!(classify_row(&a, &b), RowChange::Moved);
    // Same position, different role → edited.
    let c = json!({ "id": "s1", "role": "PL", "position": { "x": 1.0, "y": 2.0 } });
    assert_eq!(classify_row(&a, &c), RowChange::Edited);
    // Moved AND re-roled → the stronger claim wins.
    let e = json!({ "id": "s1", "role": "PL", "position": { "x": 9.0, "y": 2.0 } });
    assert_eq!(classify_row(&a, &e), RowChange::Edited);
    // A key appearing is an edit even though every shared key is equal.
    let f = json!({ "id": "s1", "role": "SL", "position": { "x": 1.0, "y": 2.0 }, "tag": "CMD" });
    assert_eq!(classify_row(&a, &f), RowChange::Edited);
    assert_eq!(classify_row(&a, &a), RowChange::Same);
}

/// A representation flip is not an edit.
///
/// `serde_json` parses `100` as `PosInt` and `100.0` as `Float`, and `Value`'s own `PartialEq`
/// calls those unequal. Before this fix `classify_row` inherited that, so a payload whose
/// integers had been re-serialized as floats (a `yrs` BigInt↔Number coercion across an editor
/// change) reported every touched row as **edited** while nothing had changed.
#[test]
fn a_number_representation_flip_is_not_an_edit() {
    // Positive control FIRST: the two payloads really do differ under `Value`'s equality, so
    // this test is exercising the flip and not comparing a value with itself.
    let int = json!({ "id": "s1", "count": 100, "position": { "x": 1, "y": 2 } });
    let float = json!({ "id": "s1", "count": 100.0, "position": { "x": 1.0, "y": 2.0 } });
    assert_ne!(
        int, float,
        "serde_json must still consider these unequal, or this test proves nothing"
    );

    assert_eq!(
        classify_row(&int, &float),
        RowChange::Same,
        "100 and 100.0 are the same number; reporting an edit here is crying wolf"
    );
    // And a flip confined to `position` must not read as a MOVE either.
    let moved = json!({ "id": "s1", "count": 100, "position": { "x": 9.0, "y": 2.0 } });
    assert_eq!(classify_row(&int, &moved), RowChange::Moved);

    // The negative half: widening must not swallow a real difference, including two distinct
    // u64s above 2^53 that would collide if compared as f64.
    let other = json!({ "id": "s1", "count": 101.0, "position": { "x": 1, "y": 2 } });
    assert_eq!(classify_row(&int, &other), RowChange::Edited);
    let big_a = json!({ "id": "s1", "count": 9_007_199_254_740_993_u64 });
    let big_b = json!({ "id": "s1", "count": 9_007_199_254_740_992_u64 });
    assert_eq!(
        classify_row(&big_a, &big_b),
        RowChange::Edited,
        "two distinct u64s above 2^53 must not be collapsed by an f64 comparison"
    );
}

/// Every row on each side lands in exactly one bucket. This is the invariant that makes a
/// silently-dropped row impossible: if the differ ever ignores an input, these sums stop
/// matching the raw row counts.
#[test]
fn every_row_is_accounted_for_on_both_sides() {
    let d = diff_mission_payloads(&levie_v1(), &levie_v2());
    for c in &d.collections {
        assert_eq!(
            c.b_rows,
            c.added + c.moved + c.edited + c.unchanged + c.unkeyed_b,
            "{} lost a NEW-side row between the walk and the counters: {c:#?}",
            c.label
        );
        assert_eq!(
            c.a_rows,
            c.removed + c.moved + c.edited + c.unchanged + c.unkeyed_a,
            "{} lost an OLD-side row between the walk and the counters: {c:#?}",
            c.label
        );
    }
}

/// A row with no `id` cannot be matched to anything. It must be COUNTED and reported, never
/// dropped — dropping it is "a tool reports success over an input it never examined".
#[test]
fn unkeyed_rows_are_reported_not_silently_dropped() {
    let mut v2 = levie_v1();
    v2["editor"]["slots"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "name": "orphan", "role": "RFL" }));
    let d = diff_mission_payloads(&levie_v1(), &v2);
    let slots = slots_delta(&d);
    assert_eq!(slots.b_rows, 4, "the id-less row is still a row");
    assert_eq!(slots.unkeyed_b, 1);
    assert_eq!(
        slots.added, 0,
        "an unkeyable row must not be guessed as added"
    );
    assert_eq!(
        slots.unreadable(),
        1,
        "the UI needs a caveat count it can surface"
    );
}

/// `loadouts` is the one collection whose id is the OBJECT KEY, not a field in the row. A
/// differ that only understands arrays would report it as permanently unchanged.
#[test]
fn object_keyed_loadouts_are_diffed_by_their_map_key() {
    let mut v2 = levie_v1();
    v2["loadouts"]["l1"]["primary"] = json!("M4A1");
    v2["loadouts"]["l2"] = json!({ "primary": "AKM" });
    let d = diff_mission_payloads(&levie_v1(), &v2);
    let lo = d
        .collections
        .iter()
        .find(|c| c.label == "Loadouts")
        .expect("Loadouts must be a diffed collection");
    assert_eq!(lo.edited, 1, "l1 changed its primary: {lo:#?}");
    assert_eq!(lo.added, 1, "l2 is new: {lo:#?}");
    assert_eq!(lo.a_rows, 1);
    assert_eq!(lo.b_rows, 2);
}

/// **The performance contract, as an assertion.** Counts are exact at any size; the sample
/// list is what stays bounded. If this ever fails by `samples.len()` growing, a diff of the
/// 367k-slot missions this codebase really has would build a 367k-entry `Vec<String>`.
#[test]
fn counts_are_exact_at_scale_while_samples_stay_bounded() {
    const N: usize = 5_000;
    let mut big = levie_v1();
    {
        let slots = big["editor"]["slots"].as_array_mut().unwrap();
        slots.clear();
        for i in 0..N {
            slots.push(json!({
                "id": format!("bulk-{i}"),
                "name": format!("Rifleman {i}"),
                "squadId": "sq1",
                "position": { "x": i as f64, "y": 0.0, "z": 0.0, "rotation": 0.0 }
            }));
        }
    }
    let empty = json!({});
    let d = diff_mission_payloads(&empty, &big);
    let slots = slots_delta(&d);
    assert_eq!(slots.added, N, "every bulk row must be counted");
    assert_eq!(slots.b_rows, N);
    assert_eq!(
        slots.samples.len(),
        DIFF_SAMPLE_CAP,
        "the sample list is the only thing allowed to grow with the document, and it must not"
    );
}

/// The census must be the differ's own output, not a parallel row-counter that can drift.
#[test]
fn census_is_the_diff_from_the_empty_document() {
    let census = version_census(&levie_v1());
    assert_eq!(
        census,
        vec![
            ("Slots", 3),
            ("Squads", 1),
            ("Factions", 1),
            ("Editor layers", 1),
            ("Objectives", 1),
            ("Loadouts", 1),
        ],
        "census must list every non-empty collection, in payload order"
    );
    assert_eq!(
        census_line(&census),
        "3 slots · 1 squads · 1 factions · 1 editor layers · 1 objectives · 1 loadouts"
    );
    // Empty collections are omitted, not rendered as "0 markers".
    assert!(!census.iter().any(|(l, _)| *l == "Markers"));
    // The seeded golden mission's payload is literally `{}` — the dossier's empty state.
    assert!(version_census(&json!({})).is_empty());
}

/// Source ratchet. Pins the two structural decisions the comparison is built on, so a
/// later "simplification" into a whole-document string compare goes red here rather
/// than in a browser at hundreds of thousands of rows.
#[test]
fn differ_source_ratchet_is_structural_and_wired_into_the_dossier() {
    let src = crate::v2::core::test_support::pins::mission_library_source();
    assert!(
        src.contains("let mut left: HashMap<&str, &Value> = HashMap::new();"),
        "the id index must stay a BORROWED map — cloning rows doubles a hundreds-of-MB payload"
    );
    // concat! so this assertion does not match its own source text.
    let textual = concat!("serde_json::to_string(a) ", "== serde_json::to_string(b)");
    assert!(
        !src.contains(textual),
        "a whole-document string compare is O(n) memory and answers the wrong question — \
             re-emit order is not an edit (see reordering_rows_is_not_a_change)"
    );
    assert!(
        src.contains("version_history_section(&m)") && src.contains("{version_rail}"),
        "the version rail must be MOUNTED in the dossier — a differ nothing calls is dead \
             code that #![allow(dead_code)] at the top of this file would hide"
    );
    assert!(
            src.contains("fn version_census(payload: &Value) -> Vec<(&'static str, usize)> {\n    diff_mission_payloads(&Value::Null, payload)"),
            "the census must route through the differ, not count rows a second way"
        );
}

// ── Mission document upload ──────────────────────────────────────────────────────

/// **The test this upload panel lives or dies on.**
///
/// `create_version` answers a bad document with 400 + a `details` array naming every finding.
/// This drives the FULL client chain the browser drives — the raw response body through
/// [`crate::v2::core::api::client::error_body_message`] (which folds `details` into the message as extra
/// lines) and out through [`upload_failure`] — and demands every finding survive.
///
/// The body is **not invented**: it is the response measured from the running dev API on
/// 2026-07-31 for `POST /missions/{id}/versions` with `schemaVersion:"nope"`,
/// `markers:"not-an-array"` and a numeric slot `role`.
///
/// RED under perturbation: return `Vec::new()` from `upload_failure`'s generic arm, or route
/// 400 through one of the detail-less status arms, and this fails naming the lost finding.
#[test]
fn a_rejected_document_surfaces_every_finding_the_api_sent() {
    let measured = json!({
        "error": "invalid mission payload",
        "details": [
            "/schemaVersion: \"nope\" is not of type \"integer\"",
            "/markers: \"not-an-array\" is not of type \"array\"",
            "/editor: this payload does not match the shape the mission compiler reads, so it cannot be compiled — invalid type: integer `123`, expected a string at line 1 column 83"
        ]
    });
    let msg = crate::v2::core::api::client::error_body_message(&measured)
        .expect("the client must extract a message from a 400 body carrying `error`");
    let (head, findings) = upload_failure(400, Some(&msg), "0.2.0");

    assert_eq!(
        findings.len(),
        3,
        "all three findings must reach the author; got {findings:?}"
    );
    for needle in [
        "/schemaVersion",
        "is not of type \"integer\"",
        "/markers",
        "/editor",
        "expected a string at line 1 column 83",
    ] {
        assert!(
            findings.iter().any(|f| f.contains(needle)),
            "the author must be told {needle:?} — an upload UI that swallows the reason is \
                 worse than none, because the document cannot be fixed from a verdict. \
                 findings={findings:?}"
        );
    }
    assert!(
        head.contains("invalid mission payload") && head.contains("400"),
        "the headline must name the verdict and the status; got {head:?}"
    );
    assert!(
            head.contains("3 problem(s)"),
            "the headline must point at the list rather than pretend there is one problem; got {head:?}"
        );
}

/// A systematic defect yields one finding per slot; the API caps its own list at 20 and the
/// client folds at [`crate::v2::core::api::client::MAX_ERROR_DETAILS`] with a `… and N more` tail. That tail
/// is itself a finding row and must reach the author — otherwise a 20-problem document reads
/// as a 6-problem one and the author "fixes" it and re-uploads into the same wall.
#[test]
fn a_truncated_finding_list_still_tells_the_author_how_many_there_are() {
    let details: Vec<String> = (0..20)
        .map(|i| format!("/editor/slots/{i}/role: 42 is not of type \"string\""))
        .collect();
    let body = json!({ "error": "invalid mission payload", "details": details });
    let msg = crate::v2::core::api::client::error_body_message(&body).expect("message");
    let (_head, findings) = upload_failure(400, Some(&msg), "1.0.0");
    assert_eq!(
        findings.len(),
        crate::v2::core::api::client::MAX_ERROR_DETAILS + 1,
        "six shown findings plus the count tail; got {findings:?}"
    );
    assert!(
        findings.last().is_some_and(|t| t.contains("14 more")),
        "the tail must name the 14 findings not shown; got {:?}",
        findings.last()
    );
}

/// The four failures that are NOT a defect in the document get their own words, because the
/// author does something different about each — and none of them carries `details`.
#[test]
fn non_validation_failures_say_what_to_do_instead() {
    let (head, rows) = upload_failure(409, Some("version already exists"), "0.2.0");
    assert!(rows.is_empty());
    assert!(
        head.contains("0.2.0") && head.contains("already exists"),
        "409 must name the taken version; got {head:?}"
    );

    // The backend computes its own MB figure from MISSION_VERSION_MAX_BODY_BYTES. Echo it —
    // restating a number here would be a second source of truth that silently goes stale.
    let (head, _) = upload_failure(413, Some("payload too large (max 256 MB)"), "0.2.0");
    assert!(
        head.contains("256 MB"),
        "413 must carry the server's own limit verbatim; got {head:?}"
    );

    let (head, _) = upload_failure(401, None, "0.2.0");
    assert!(head.to_lowercase().contains("sign in"), "got {head:?}");

    // status 0 is the client's transport failure (`ApiErr` uses 0 for network/serde).
    let (head, _) = upload_failure(0, None, "0.2.0");
    assert!(
        head.contains("Nothing was saved"),
        "a network failure must say the mission is unchanged; got {head:?}"
    );

    // A 403 has no `details` but does have a message, and must not be swallowed.
    let (head, _) = upload_failure(403, Some("not your mission"), "0.2.0");
    assert!(
        head.contains("not your mission"),
        "the backend's own reason must survive; got {head:?}"
    );
}

/// **The round-trip.** Both exporters wrap the editor payload in an envelope
/// (`compile_export` and the API's `build_mission_doc`), and posting that envelope verbatim
/// is MEASURED to answer `400 payload must include editor content` — a message that sends the
/// author looking in entirely the wrong place. Export → Upload only works if the envelope is
/// unwrapped, so this pins both shapes.
///
/// The envelope below is the key set captured from `GET /missions/{id}/export` on 2026-07-31.
#[test]
fn both_an_exported_file_and_a_bare_payload_upload() {
    let payload = json!({
        "schemaVersion": 3,
        "editor": { "slots": [{ "id": "s1", "callsign": "Alpha 1-1", "role": "Rifleman" }] },
        "markers": []
    });
    let envelope = json!({
        "exportFormatVersion": 1,
        "missionId": "8881e97e-b348-4052-bdea-4a45c8e962a7",
        "title": "T-117 scratch probe",
        "terrain": "everon",
        "gameMode": "pve_coop",
        "weather": "clear",
        "timeOfDay": "14:00:00",
        "maxPlayers": 8,
        "version": "0.2.0",
        "armory": [],
        "payload": payload.clone(),
        "exportedAt": "2026-07-31T05:00:00Z"
    });
    assert_eq!(
        unwrap_export_envelope(envelope).expect("an exported mission file must upload"),
        payload,
        "the envelope's `payload` is the editor document the API validates"
    );
    // A bare editor payload has no `exportFormatVersion` and passes through untouched.
    assert_eq!(
        unwrap_export_envelope(payload.clone()).expect("a bare payload must upload"),
        payload
    );
}

/// The two ways an envelope can be unusable are named as envelope problems, not as schema
/// errors 200 lines deep — the author needs to know the file is the wrong *kind* of thing.
#[test]
fn a_broken_envelope_is_named_as_an_envelope_problem() {
    let no_payload = json!({ "exportFormatVersion": 1, "title": "x" });
    let err = unwrap_export_envelope(no_payload).expect_err("no payload must be refused");
    assert!(
        err.contains("exported mission file") && err.contains("payload"),
        "got {err:?}"
    );

    let bad_payload = json!({ "exportFormatVersion": 1, "payload": [1, 2, 3] });
    let err = unwrap_export_envelope(bad_payload).expect_err("array payload must be refused");
    assert!(
        err.contains("an array"),
        "the kind must be named; got {err:?}"
    );

    // A top-level array is the shape someone gets by exporting the wrong thing entirely.
    let err = unwrap_export_envelope(json!([1, 2])).expect_err("array doc must be refused");
    assert!(
        err.contains("JSON object") && err.contains("an array"),
        "got {err:?}"
    );
    let err = unwrap_export_envelope(json!("hello")).expect_err("string doc must be refused");
    assert!(err.contains("a string"), "got {err:?}");
}

/// A syntax error keeps `serde_json`'s line/column. The server's own answer for the same file
/// is a flat `payload is not valid JSON` with no position, so parsing client-side is the only
/// way an author locates the break in a 30 MB document.
#[test]
fn a_syntax_error_keeps_its_line_and_column() {
    let err = parse_uploaded_document("{\n  \"schemaVersion\": 3,\n  \"editor\": {,\n}")
        .expect_err("malformed JSON must be refused");
    assert!(err.contains("not valid JSON"), "got {err:?}");
    assert!(
        err.contains("line 3") && err.contains("column"),
        "the position is the whole point of parsing here; got {err:?}"
    );
    assert_eq!(
        parse_uploaded_document("   \n\t ").expect_err("blank file"),
        "That file is empty."
    );
    // The happy path goes through the envelope unwrap, so a pasted export parses to its payload.
    let got = parse_uploaded_document(
        r#"{"exportFormatVersion":1,"payload":{"schemaVersion":3,"markers":[]}}"#,
    )
    .expect("an exported file must parse");
    assert_eq!(got, json!({"schemaVersion": 3, "markers": []}));
}

/// The size gate refuses BEFORE the file is read, and names both numbers. `usize` at the exact
/// budget is accepted — an off-by-one here would reject a file the tab can handle.
#[test]
fn the_size_gate_names_both_numbers_and_is_inclusive_at_the_budget() {
    assert!(oversize_refusal(0).is_none());
    assert!(
        oversize_refusal(UPLOAD_MAX_BYTES).is_none(),
        "a document exactly at the budget must be accepted"
    );
    let refusal = oversize_refusal(UPLOAD_MAX_BYTES + 1).expect("over budget must be refused");
    assert!(
        refusal.contains("8.4 MB"),
        "the budget must be named; got {refusal:?}"
    );
    let huge = oversize_refusal(400 << 20).expect("400 MiB must be refused");
    // BOTH numbers, and this is the case that can actually prove it: one byte over the budget
    // rounds to the same text as the budget, so the assertion above cannot tell the two apart
    // and must not be asked to. (It used to be written as `contains(X) && contains(X)` — the
    // same needle twice — which read as a two-number check and was a one-number check.)
    assert!(
        huge.contains("419.4 MB") && huge.contains("8.4 MB"),
        "both the author's file size and the budget must be named — 'too large' without them \
             is unactionable; got {huge:?}"
    );
    assert!(
        huge.contains("Mission Creator"),
        "a refusal must say what to do instead; got {huge:?}"
    );
}

#[test]
fn duplicate_slot_id_under_callsign_is_refused() {
    let payload = json!({
        "schemaVersion": 1,
        "editor": {
            "squads": [
                {
                    "id": "sq1",
                    "callsign": "Alpha 1-1",
                    "slotIds": ["s1", "s1"]
                }
            ],
            "slots": [
                { "id": "s1", "role": "SL" }
            ]
        }
    });
    let text = serde_json::to_string(&payload).unwrap();
    let err = parse_uploaded_document(&text).expect_err("duplicate slot id must be refused");
    assert!(
        err.contains("Alpha 1-1") && err.contains("s1"),
        "refusal must name both callsign and duplicated slot id; got {err:?}"
    );
}

/// The suggested version has to be valid semantic versioning, because the route parses
/// it, and unused, because a duplicate is refused — so it is a patch bump of the tip.
/// Pre-release and build metadata are dropped rather than incremented inside.
#[test]
fn the_suggested_version_bumps_the_patch_of_the_tip() {
    assert_eq!(next_semver(Some("0.1.0")), "0.1.1");
    assert_eq!(next_semver(Some("1.2.3")), "1.2.4");
    assert_eq!(next_semver(Some("2.0.9")), "2.0.10");
    assert_eq!(next_semver(Some("1.2.3-rc1")), "1.2.4");
    assert_eq!(next_semver(Some("1.2.3+build.7")), "1.2.4");
    // No current version, or something this parser cannot read, falls back to the number
    // `create_mission` itself writes first — always valid, and only taken if the mission
    // already has one, in which case the 409 says so precisely.
    assert_eq!(next_semver(None), "0.1.0");
    for junk in ["", "1", "1.2", "1.2.3.4", "banana", "1.2.x", " 1.2.3"] {
        assert_eq!(next_semver(Some(junk)), "0.1.0", "junk semver {junk:?}");
    }
}

/// The preview runs the payload comparison with a real second document — the case it
/// was written for and could not reach until an uploaded document existed. Counts are
/// exact; rows with no usable id are reported rather than dropped.
#[test]
fn the_preview_names_what_the_document_would_change() {
    let current = json!({
        "schemaVersion": 3,
        "map": { "terrain": "everon" },
        "editor": { "slots": [
            { "id": "a", "callsign": "Alpha 1-1", "role": "Rifleman", "position": [1.0, 1.0] },
            { "id": "b", "callsign": "Alpha 1-2", "role": "Medic",    "position": [2.0, 2.0] }
        ]}
    });
    let uploaded = json!({
        "schemaVersion": 3,
        "map": { "terrain": "arland" },
        "editor": { "slots": [
            { "id": "a", "callsign": "Alpha 1-1", "role": "Rifleman", "position": [9.0, 9.0] },
            { "id": "b", "callsign": "Alpha 1-2", "role": "Grenadier","position": [2.0, 2.0] },
            { "id": "c", "callsign": "Alpha 1-3", "role": "Rifleman", "position": [3.0, 3.0] },
            { "callsign": "no id at all" }
        ]}
    });
    let lines = diff_summary_lines(&diff_mission_payloads(&current, &uploaded));
    let joined = lines.join("\n");
    assert!(
        joined.contains("Terrain: everon → arland"),
        "a changed scalar must be named; got\n{joined}"
    );
    assert!(
        joined.contains("Slots: 2 → 4"),
        "the row census must be exact on both sides; got\n{joined}"
    );
    assert!(
        joined.contains("1 added") && joined.contains("1 moved") && joined.contains("1 edited"),
        "added/moved/edited must be distinguished; got\n{joined}"
    );
    assert!(
        joined.contains("no usable id"),
        "a row the differ could not key must be reported, not silently dropped — that is the \
             'reports success over an input it never examined' failure in miniature; got\n{joined}"
    );

    // Identical payloads produce nothing to say, which is what lets the UI offer the
    // "uploading this would only add a version number" line honestly.
    assert!(diff_mission_payloads(&current, &current).is_empty());
    assert!(diff_summary_lines(&diff_mission_payloads(&current, &current)).is_empty());
    // The empty document on the left is the census path — same code, so they cannot drift.
    assert!(!diff_mission_payloads(&Value::Null, &uploaded).is_empty());
}

/// Source ratchet. The pure functions above can all pass while the panel is
/// wired to nothing, so pin the wiring itself: the control, the route, and — above all —
/// that the failure path goes through [`upload_failure`] into a rendered findings list.
///
/// RED under perturbation: delete the section, point the POST somewhere else, drop the
/// `up_findings.set(rows)` assignment, or delete the findings block from the view.
#[test]
fn the_upload_panel_is_wired_to_the_versions_route() {
    let src = crate::v2::core::test_support::pins::mission_library_source();
    let production = src
        .split("#[cfg(test)]")
        .next()
        .expect("missions.rs must have a #[cfg(test)] module");

    for (needle, why) in [
        (
            "data-testid=\"mission-upload-pick\"",
            "the dossier must render a control that opens the file picker",
        ),
        (
            "data-testid=\"mission-upload-findings\"",
            "the rejection findings must have a rendered home; a toast is not one",
        ),
        (
            "up_findings.set(rows)",
            "the rows from upload_failure must reach the rendered list — dropping this \
                 assignment is exactly the 'swallowed the reason' defect",
        ),
        (
            "upload_failure(status, msg.as_deref(), &semver)",
            "the error path must route through upload_failure, not a fixed string",
        ),
        (
            "format!(\"/missions/{}/versions\", id_sv.get_value())",
            "the upload must POST the versions route — the only mission-write route with a \
                 lifted body cap",
        ),
        (
            "parse_uploaded_document(&text)",
            "the picked file must be parsed (and envelope-unwrapped) before it is posted",
        ),
        (
            "oversize_refusal(size)",
            "the size gate must run on the picked file's size before the read",
        ),
        (
            "version_body_to_writer(",
            "the wire body must be built by map-engine-core, not hand-rolled here, so the \
                 two doors onto create_version cannot drift — `version_body_to_writer` and the \
                 editor Save's `version_body` are both wrappers over one `VersionBody` struct, \
                 and compile.rs's `both_doors_onto_create_version_serialise_identical_bytes` \
                 pins them byte-identical",
        ),
        (
            "api_post_raw(store, &path, body)",
            "the upload must hand over an already-serialised String — `api_post` takes a \
                 `Value` and clones it again per attempt, which is the amplification that sets \
                 UPLOAD_MAX_BYTES",
        ),
        (
            "up_doc.with_untracked(|slot|",
            "the document must be read BY REFERENCE to build the body — `get_untracked()` \
                 clones the whole parsed tree, and on wasm32 that clone is a whole extra copy of \
                 the mission for no reason",
        ),
    ] {
        assert!(production.contains(needle), "{why} (missing: {needle})");
    }

    // `File`/`FileList` are what `HtmlInputElement::files()` needs; without them this compiles
    // on native and dies on the wasm build.
    let cargo = crate::v2::core::test_support::fixtures::crate_cargo_toml();
    for feat in ["\"File\"", "\"FileList\"", "\"Blob\""] {
        assert!(
            cargo.contains(feat),
            "Cargo.toml must keep the web-sys {feat} feature for the document picker"
        );
    }
}
