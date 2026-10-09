use super::{crewed_slot_ids, map_render_keep_indices, selectable_ids};
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
