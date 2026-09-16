//! Role: Domain regression cases.
//! Position: `doc/store/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn hidden_slot_raw_membership_moves_and_restores_without_losing_authored_data() {
    for layer_hidden in [false, true] {
        let mut doc = keep_source_fixture();
        doc.add_slot(
            "hidden",
            "sq-mid",
            "lyr",
            0,
            "Medic",
            Some("Doc".into()),
            Some("Prefab/Medic.et".into()),
            4321.125,
            5678.375,
            123.625,
            77.5,
        );
        doc.set_leader("sq-mid", "hidden");
        doc.update_slot_loadout("hidden", Some(r#"{"authored":"medical-supplies"}"#.into()));
        doc.add_vehicle("transport", "Prefab/Truck.et", None, None, None, None);
        doc.attach_vehicle("sq-mid", "transport");
        if layer_hidden {
            doc.set_editor_layer_hidden("lyr", true);
        } else {
            doc.set_slot_editor_hidden("hidden", true);
        }
        assert!(
            doc.materialize().ids.is_empty(),
            "fixture must really be hidden"
        );
        assert!(doc.slot_exists("hidden"));
        let original_squad = doc
            .slot_squad_id("hidden")
            .expect("hidden Attributes must resolve the authored squad, not report no move");
        assert_eq!(original_squad, "sq-mid");
        assert_eq!(doc.slot_squad_id("missing"), None);
        let before_slots = slots_map(&doc);
        let before_maps = small_maps(&doc);

        doc.begin_group();
        doc.move_slot_to_squad_keep_source("hidden", "sq-opf");
        doc.end_group();
        assert_eq!(doc.slot_squad_id("hidden").as_deref(), Some("sq-opf"));
        let moved = slots_map(&doc);
        assert_eq!(
            moved["hidden"]["position"],
            before_slots["hidden"]["position"]
        );
        assert_eq!(
            moved["hidden"]["loadout"],
            before_slots["hidden"]["loadout"]
        );
        assert_eq!(
            small_maps(&doc)["vehiclesById"],
            before_maps["vehiclesById"]
        );
        assert!(
            doc.materialize().ids.is_empty(),
            "reassignment must not unhide it"
        );
        doc.begin_group();
        doc.move_slot_to_squad_keep_source("hidden", &original_squad);
        doc.end_group();
        assert_eq!(slots_map(&doc), before_slots);
        assert_eq!(small_maps(&doc), before_maps);
        assert!(doc.undo());
        assert_eq!(doc.slot_squad_id("hidden").as_deref(), Some("sq-opf"));
        assert!(doc.redo());
        assert_eq!(slots_map(&doc), before_slots);
        assert_eq!(small_maps(&doc), before_maps);
    }
}
