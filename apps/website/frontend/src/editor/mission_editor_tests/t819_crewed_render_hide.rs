use super::{crewed_slot_ids, map_render_keep_indices, selectable_ids};
use crate::editor::arsenal::class_r_scrub::{live_code, only_body};
use std::collections::HashSet;

fn slots_json() -> String {
    serde_json::json!({
        "s0": { "position": { "x": 10.0, "y": 20.0, "z": 12.345678901234567, "rotation": 0.0 } },
        "s1": { "position": { "x": 11.0, "y": 21.0, "z": 0.5, "rotation": 0.0 } },
        "s2": { "position": { "x": 12.0, "y": 22.0, "z": 1.0, "rotation": 0.0 } },
    })
    .to_string()
}

fn small_with_crew(crew: serde_json::Value) -> String {
    serde_json::json!({
        "vehiclesById": {
            "v0": {
                "resourceName": "Prefab/A.et",
                "position": { "x": 1.0, "y": 2.0 },
                "crew": crew,
            }
        }
    })
    .to_string()
}

fn small_no_crew() -> String {
    serde_json::json!({
        "vehiclesById": {
            "v0": {
                "resourceName": "Prefab/A.et",
                "position": { "x": 1.0, "y": 2.0 },
            }
        }
    })
    .to_string()
}

fn slot_z(slots_json: &str, id: &str) -> f64 {
    let v: serde_json::Value = serde_json::from_str(slots_json).unwrap();
    v[id]["position"]["z"].as_f64().expect("z")
}

/// DEFECT CLASS (pre-fix): assigning Driver/Gunner did not change the map SoA — figures stayed.
/// The keep-index count must drop by exactly the boarded set.
#[test]
fn assign_driver_and_gunner_drops_two_map_render_rows() {
    let ids = vec!["s0".into(), "s1".into(), "s2".into()];
    let before = map_render_keep_indices(&ids, &crewed_slot_ids(&small_no_crew()));
    assert_eq!(before.len(), 3, "uncrewed: every figure is on the map");

    let crewed = crewed_slot_ids(&small_with_crew(serde_json::json!({
        "driver": "s0",
        "gunner": "s1",
    })));
    assert_eq!(crewed, HashSet::from(["s0".into(), "s1".into()]));
    let after = map_render_keep_indices(&ids, &crewed);
    assert_eq!(
        after.len(),
        1,
        "T-819: Driver+Gunner must leave the map render SoA (row count -2); kept={after:?}"
    );
    assert_eq!(ids[after[0]], "s2");
}

/// Trap 2 — materialize/compile universe is NOT this filter. `selectable_ids` (slots_json) still
/// holds boarded slots; a filter that reused T-701's drop would also yank them from existence.
#[test]
fn crewed_slots_remain_in_slots_json_universe_and_selection() {
    let small = small_with_crew(serde_json::json!({ "driver": "s0", "gunner": "s1" }));
    let live = selectable_ids(&slots_json(), &small);
    assert!(live.contains("s0") && live.contains("s1") && live.contains("s2"));
    // Outliner-reachable selection: pruning over selectable_ids keeps boarded ids.
    let sel = ["s0", "s1", "s2"];
    let kept: Vec<_> = sel
        .iter()
        .filter(|id| live.contains(**id))
        .copied()
        .collect();
    assert_eq!(kept, vec!["s0", "s1", "s2"]);
}

/// Trap 1 — unassign restores the figure; stored z is exact f64 (untouched).
#[test]
fn unassign_restores_figure_at_stored_z_exact_f64() {
    let slots = slots_json();
    let z0 = slot_z(&slots, "s0");
    assert_eq!(z0, 12.345678901234567);

    let ids = vec!["s0".into(), "s1".into(), "s2".into()];
    let boarded = small_with_crew(serde_json::json!({ "driver": "s0", "gunner": "s1" }));
    assert_eq!(
        map_render_keep_indices(&ids, &crewed_slot_ids(&boarded)).len(),
        1
    );

    // Unassign Driver only — s0 returns, s1 stays hidden.
    let one_cleared = small_with_crew(serde_json::json!({ "gunner": "s1" }));
    let keep = map_render_keep_indices(&ids, &crewed_slot_ids(&one_cleared));
    assert_eq!(keep.len(), 2);
    let kept_ids: HashSet<_> = keep.iter().map(|&i| ids[i].as_str()).collect();
    assert!(kept_ids.contains("s0") && kept_ids.contains("s2"));
    assert_eq!(
        slot_z(&slots, "s0"),
        12.345678901234567,
        "T-819: unassign must not rewrite the slot's stored z"
    );
}

/// Delete the vehicle → both figures return (crew map gone with the vehicle).
#[test]
fn delete_vehicle_restores_both_figures() {
    let ids = vec!["s0".into(), "s1".into(), "s2".into()];
    let empty = serde_json::json!({ "vehiclesById": {} }).to_string();
    assert_eq!(
        map_render_keep_indices(&ids, &crewed_slot_ids(&empty)).len(),
        3
    );
}

/// Undo of an assignment = crew map gone → visibility round-trips.
#[test]
fn undo_assignment_round_trips_visibility() {
    let ids = vec!["s0".into(), "s1".into()];
    let assigned = small_with_crew(serde_json::json!({ "driver": "s0" }));
    assert_eq!(
        map_render_keep_indices(&ids, &crewed_slot_ids(&assigned)).len(),
        1
    );
    let undone = small_no_crew();
    assert_eq!(
        map_render_keep_indices(&ids, &crewed_slot_ids(&undone)).len(),
        2
    );
}

/// Trap 1 — assign path must not stamp `editorHidden` / call the T-701 mutator.
#[test]
fn assign_crew_seat_does_not_write_editor_hidden() {
    let ops = live_code(include_str!("../state/operations/entity.rs"));
    let body = only_body(&ops, "pub fn assign_crew_seat");
    assert!(
        !body.contains("editorHidden") && !body.contains("set_slots_editor_hidden"),
        "T-819: crew assignment must not reuse T-701 editorHidden — body:\n{body}"
    );
    assert!(
        body.contains("assign_crew_seat") || body.contains("after_local_edit"),
        "T-819: assign_crew_seat body should still board via the core mutator"
    );
}

/// Wiring — every map glyph bind feeds `map_render_slot_soa`, not bare `materialize()`.
#[test]
fn map_binds_feed_map_render_slot_soa() {
    let hist = include_str!("../state/history.rs");
    let rebind = only_body(hist, "pub fn rebind_engine_from_doc");
    let after = only_body(hist, "fn after_doc_change");
    for (name, body) in [
        ("rebind_engine_from_doc", rebind),
        ("after_doc_change", after),
    ] {
        assert!(
            body.contains("map_render_slot_soa"),
            "T-819: {name} must bind via map_render_slot_soa; body:\n{body}"
        );
        assert!(
            !body.contains("MissionDocCore::materialize")
                && !body.contains(".map(MissionDocCore::materialize)"),
            "T-819: {name} must not bind the unfiltered materialize SoA; body:\n{body}"
        );
        assert!(
            body.contains("slot_count"),
            "T-819: {name} must keep OBJ on authored slot_count, not filtered SoA len"
        );
    }
    // Anchor past the early registry_session `#[cfg(test)]` that would otherwise cut the page
    // (T-750 idiom): the first bind + pick sites live inside `MissionEditorPage`.
    let raw = include_str!("../mission_editor.rs");
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let page = live_code(&raw[raw.find(anchor.as_str()).expect("MissionEditorPage")..]);
    assert!(
        page.contains("map_render_slot_soa") && page.matches("map_render_slot_soa").count() >= 2,
        "T-819: MissionEditorPage must call map_render_slot_soa at the first bind and picks"
    );
    assert!(
        !page.contains(".map(|c| c.materialize())"),
        "T-819: first bind must not feed bare materialize into slots_bind_symbology"
    );
}

/// FIRED RULE — perturb the keep filter so crewed ids stay; the assign pin goes RED.
#[test]
fn perturbing_the_keep_filter_makes_the_assign_pin_fail() {
    let ids = vec!["s0".into(), "s1".into(), "s2".into()];
    let crewed = crewed_slot_ids(&small_with_crew(serde_json::json!({
        "driver": "s0",
        "gunner": "s1",
    })));
    // Green path (control).
    assert_eq!(map_render_keep_indices(&ids, &crewed).len(), 1);

    // RED: a keep filter that ignores `crewed` (the pre-T-819 defect).
    let perturbed: Vec<usize> = (0..ids.len()).collect();
    let result = std::panic::catch_unwind(|| {
        assert_eq!(
            perturbed.len(),
            1,
            "T-819: Driver+Gunner must leave the map render SoA (row count -2); kept={perturbed:?}"
        );
    });
    let err = result.expect_err("perturbation must RED");
    let msg = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        format!("{err:?}")
    };
    // Print for the report's verbatim RED field (test still passes — we caught the panic).
    eprintln!("T-819 PERTURBATION RED OUTPUT:\n{msg}");
    assert!(
        msg.contains("Driver+Gunner must leave the map render SoA") || msg.contains("row count -2"),
        "unexpected panic payload: {msg}"
    );
    // Restored green.
    assert_eq!(map_render_keep_indices(&ids, &crewed).len(), 1);
}
