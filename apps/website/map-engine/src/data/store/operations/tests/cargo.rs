//! Role: the installed cargo defaults, the loadout buffer, and the Apply seed.
//! Position: `doc/operations/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

fn rifleman() -> Vec<CargoRow> {
    vec![CargoRow {
        container: "backpack".to_string(),
        item: "bandage".to_string(),
        qty: 2,
    }]
}

fn doc_with_slot(asset_id: Option<String>) -> MissionDocCore {
    let core = MissionDocCore::new();
    core.add_squad("squad-a", "faction-BLUFOR", "Alpha", None);
    core.add_slot(
        "slot-1", "squad-a", "layer-a", 0, "RFL", None, asset_id, 0.0, 0.0, 0.0, 0.0,
    );
    core
}

fn installed(asset_id: &str) -> HashMap<String, Vec<CargoRow>> {
    let mut map = HashMap::new();
    map.insert(asset_id.to_string(), rifleman());
    map
}

#[test]
fn the_installed_defaults_answer_by_asset_and_are_silent_about_the_rest() {
    set_cargo_defaults(installed("char.us.rifleman"));
    assert_eq!(cargo_defaults_for("char.us.rifleman"), Some(rifleman()));
    assert_eq!(cargo_defaults_for("char.ru.rifleman"), None);
}

#[test]
fn seeding_an_asset_with_no_installed_default_leaves_the_slot_alone() {
    let core = doc_with_slot(Some("char.us.rifleman".to_string()));
    assert!(!seed_cargo_for_asset(
        &core,
        "slot-1",
        "char.us.rifleman",
        None
    ));
    assert_eq!(read_loadout(&core, "slot-1"), None);
}

#[test]
fn seeding_an_asset_with_an_installed_default_writes_the_rows_once() {
    set_cargo_defaults(installed("char.us.rifleman"));
    let core = doc_with_slot(Some("char.us.rifleman".to_string()));
    assert!(seed_cargo_for_asset(
        &core,
        "slot-1",
        "char.us.rifleman",
        None
    ));
    let seeded = read_loadout(&core, "slot-1").expect("the seed reached the slot");
    assert!(seeded.contains("bandage"));

    assert!(
        !seed_cargo_for_asset(&core, "slot-1", "char.us.rifleman", Some(&seeded)),
        "a loadout that already carries a cargo key is the author's, not the default's"
    );
}

#[test]
fn seeding_a_slot_reads_the_asset_the_slot_itself_names() {
    set_cargo_defaults(installed("char.us.rifleman"));
    let core = doc_with_slot(Some("char.us.rifleman".to_string()));
    let seeded = seed_slot_cargo_from_defaults(&core, "slot-1").expect("the slot names an asset");
    assert!(seeded.contains("bandage"));
    assert!(
        read_loadout(&core, "slot-1").is_some_and(|live| live.contains("bandage")),
        "the returned JSON is what reached the document"
    );
}

#[test]
fn a_slot_that_names_no_asset_cannot_be_seeded() {
    set_cargo_defaults(installed("char.us.rifleman"));
    let core = doc_with_slot(None);
    assert_eq!(seed_slot_cargo_from_defaults(&core, "slot-1"), None);
}

#[test]
fn a_copy_that_finds_nothing_leaves_the_previous_buffer_standing() {
    let core = doc_with_slot(Some("char.us.rifleman".to_string()));
    assert_eq!(
        buffer_loadouts_from_selection(&core, vec!["slot-1".to_string()]),
        1
    );
    assert_eq!(loadout_buffer_len(), 1);
    assert_eq!(loadout_buffer()[0].source_id, "slot-1");

    assert_eq!(buffer_loadouts_from_selection(&core, Vec::new()), 0);
    assert_eq!(
        loadout_buffer_len(),
        1,
        "an empty Copy must not destroy what was copied before it"
    );
}

#[test]
fn a_bare_slot_buffers_as_a_bare_loadout_rather_than_as_nothing() {
    let core = doc_with_slot(Some("char.us.rifleman".to_string()));
    assert_eq!(
        buffer_loadouts_from_selection(&core, vec!["slot-1".to_string()]),
        1
    );
    assert_eq!(loadout_buffer()[0].loadout_json, None);
}

#[test]
fn every_apply_draws_a_seed_the_previous_apply_did_not() {
    let first = next_apply_seed();
    let second = next_apply_seed();
    assert_ne!(first, second);
    assert_eq!(second, first.wrapping_add(APPLY_SEED_GAMMA));
}
