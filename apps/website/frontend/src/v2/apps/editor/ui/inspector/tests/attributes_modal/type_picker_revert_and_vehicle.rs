//! Attributes modal type picker revert and vehicle tests.

use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

#[test]
fn axis_chip_class_is_three_distinct_axis_colours_plus_rotation() {
    use super::field_gates_and_labels::axis_chip_class;
    let x = axis_chip_class("X").expect("X is coloured");
    let y = axis_chip_class("Y").expect("Y is coloured");
    let z = axis_chip_class("Z").expect("Z is coloured");
    let rot = axis_chip_class("Rotation").expect("Rotation is coloured");
    let mut axes = vec![x, y, z];
    axes.sort_unstable();
    axes.dedup();
    assert_eq!(
        axes.len(),
        3,
        "X/Y/Z must be three distinct colours; got {x}/{y}/{z}"
    );
    assert!(
        rot != x && rot != y && rot != z,
        "Rotation must carry a fourth distinct chip"
    );
    for other in [
        "Role",
        "Tag",
        "Type",
        "Stance",
        "Role Description",
        "Squad",
        "",
    ] {
        assert!(
            axis_chip_class(other).is_none(),
            "{other} must not get an axis chip"
        );
    }
}

#[test]
fn the_axis_chip_rides_the_label_and_is_aria_hidden() {
    let code = live_code(super::ATTRIBUTES_MODAL_SOURCE);
    let body = only_body(&code, "fn field_label(");
    assert!(
        body.contains("axis_chip_class(label)"),
        "field_label must derive the chip from the label; body was:\n{body}"
    );
    let src = live_source(super::ATTRIBUTES_MODAL_SOURCE);
    let body_src = only_body(&src, "fn field_label(");
    assert!(
        body_src.contains("aria-hidden"),
        "the colour swatch must be aria-hidden — a screen reader must not announce a bare colour"
    );
}

#[test]
fn the_type_field_is_the_searchable_picker_not_a_freetext_field() {
    let code = live_code(super::ATTRIBUTES_MODAL_SOURCE);
    let body = only_body(&code, "fn identity_tab(");
    assert!(
        body.contains("type_picker("),
        "the Type entry must be `type_picker`, not a bare text_field; body was:\n{body}"
    );
    assert!(
        body.contains("registry_items") && body.contains("g(diff.asset_id, opts.asset_id)"),
        "the picker must take the live catalog and the asset_id multi-edit gate"
    );
    assert!(
        body.contains("commit_slot(targets, None, None, None, Some(asset_id), None)"),
        "the picker must commit into the asset_id slot alone, exactly as the field did"
    );
}

#[test]
fn the_picker_keeps_a_freetext_advanced_escape_hatch() {
    let code = live_code(super::ATTRIBUTES_MODAL_SOURCE);
    let body = only_body(&code, "fn type_picker(");
    assert!(
        body.contains("text_field("),
        "type_picker must keep the T-785 text_field for unlisted ids; body was:\n{body}"
    );
    let src = live_source(super::ATTRIBUTES_MODAL_SOURCE);
    let body_src = only_body(&src, "fn type_picker(");
    assert!(
        body_src.to_lowercase().contains("advanced"),
        "the freetext field must sit behind an 'advanced' affordance"
    );
    assert!(
        body_src.contains("Faction default"),
        "empty = faction default must stay a first-class, reachable choice"
    );
}

#[test]
fn the_empty_catalog_shows_cause_and_retry_not_a_dead_list() {
    let code = live_code(super::ATTRIBUTES_MODAL_SOURCE);
    let body = only_body(&code, "fn type_picker(");
    assert!(
        body.contains("catalog_leaf_count("),
        "the empty state must key on catalog_leaf_count, not render a bare list"
    );
    let src = live_source(super::ATTRIBUTES_MODAL_SOURCE);
    let body_src = only_body(&src, "fn type_picker(");
    assert!(
        body_src.contains("No modpack is configured"),
        "the empty state must state the cause (mirrored T-800 vocabulary)"
    );
    assert!(
        body_src.contains("Retry") && body.contains("reload()"),
        "the empty state must offer a working Retry (a full reload re-runs /registry)"
    );
}

#[test]
fn the_picker_popover_consumes_its_own_escape_first() {
    let code = live_code(super::ATTRIBUTES_MODAL_SOURCE);
    let body = only_body(&code, "fn type_picker(");
    assert!(
        body.contains("stop_propagation()"),
        "the popover's Escape must stop_propagation so the modal listener does not fire too"
    );
    assert!(
        body.contains("open.set(false)"),
        "the popover's Escape must close the popover layer itself"
    );
}

#[test]
fn revert_restores_per_slot_through_the_single_slot_commit_ops() {
    let code = live_code(super::ATTRIBUTES_MODAL_SOURCE);
    let body = only_body(&code, "fn revert_to_snapshot(");
    assert!(
        body.contains("attrs_update_position(") && body.contains("attrs_update_slot("),
        "Revert must restore BOTH the transform half and the identity/type half; body was:\n{body}"
    );
    assert!(
        body.contains("for ") && body.contains("snapshot.get_value()"),
        "Revert must iterate the snapshot per slot"
    );
    assert!(
        !body.contains("_multi("),
        "Revert must NOT use the homogeneous multi batch — it would flatten differing slots"
    );
}

#[test]
fn the_revert_snapshot_is_captured_on_open_and_the_model_is_stated() {
    let code = live_code(super::ATTRIBUTES_MODAL_SOURCE);
    let host = only_body(&code, "pub fn AttributesModal(");
    assert!(
        host.contains("snapshot.set_value("),
        "AttributesModal must capture the snapshot into its store on open"
    );
    let src = live_source(super::ATTRIBUTES_MODAL_SOURCE);
    let modal = only_body(&src, "fn modal_view(");
    assert!(
        modal.contains("Edits apply live. Revert restores"),
        "the panel must state the revert model in one line"
    );
    let modal_code = only_body(&code, "fn modal_view(");
    assert!(
        modal_code.contains("revert_to_snapshot(snapshot)"),
        "the Revert button must call revert_to_snapshot"
    );
}

#[test]
fn attributes_modal_routes_vehicles_to_the_vehicle_editor() {
    let code = live_code(super::ATTRIBUTES_MODAL_SOURCE);
    let host = only_body(&code, "pub fn AttributesModal(");
    assert!(
            host.contains("is_vehicle_id(&id)") && host.contains("vehicle_attrs_view("),
            "T-818: AttributesModal None arm must route vehicles to vehicle_attrs_view; body was:\n{host}"
        );
    assert!(
        host.contains("close_attributes()"),
        "true absence must still close; body was:\n{host}"
    );
}

#[test]
fn vehicle_attrs_view_wires_heading_cargo_crew_through_existing_mutators() {
    let code = live_code(super::ATTRIBUTES_MODAL_SOURCE);
    let body = only_body(&code, "fn vehicle_attrs_view(");
    for needle in [
        "set_vehicle_heading(",
        "set_vehicle_cargo(",
        "assign_crew_seat(",
        "clear_crew_seat(",
        "number_field(",
        "Gate::open()",
        "placed_slot_choices()",
        "seat_model(",
    ] {
        assert!(
            body.contains(needle),
            "T-818: vehicle_attrs_view must contain `{needle}`; body was:\n{body}"
        );
    }
    let src = live_source(super::ATTRIBUTES_MODAL_SOURCE);
    let live = only_body(&src, "fn vehicle_attrs_view(");
    for label in ["\"Heading\"", "\"Add cargo\"", "\"Crew\"", "\"Cargo\""] {
        assert!(
            live.contains(label),
            "T-818: vehicle editor must show {label}; body was:\n{live}"
        );
    }
    assert!(
        !body.contains("attrs_multi_ids") && !body.contains("Gate::maybe"),
        "T-818: vehicle editor must not pull in T-649 multi-edit; body was:\n{body}"
    );
}

#[test]
fn vehicle_attrs_seat_model_matches_the_strip() {
    let seats =
        super::vehicle_attributes::seat_model(super::vehicle_attributes::DEFAULT_CARGO_SEATS);
    let ids: Vec<&str> = seats.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "driver",
            "gunner",
            "commander",
            "cargo1",
            "cargo2",
            "cargo3",
            "cargo4"
        ]
    );
    assert_eq!(seats[0].1, "Driver");
    assert_eq!(seats[1].1, "Gunner");
    assert_eq!(seats[2].1, "Commander");
}
