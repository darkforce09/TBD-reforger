/// T-810 (F-23) — the searchable TYPE picker, the Revert affordance, and Eden's axis colours.
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

/// F-23 (c) — the axis labels carry THREE DISTINCT colours (X/Y/Z), plus a fourth for Rotation,
/// and no other field is tinted. This is the acceptance's "3 distinct colours" pinned by CALLING
/// the pure mapping (the reason [`super::axis_chip_class`] lives outside the wasm block, like
/// `nudge_step`) rather than scraping a `view!` string.
#[test]
fn axis_chip_class_is_three_distinct_axis_colours_plus_rotation() {
    use super::axis_chip_class;
    let x = axis_chip_class("X").expect("X is coloured");
    let y = axis_chip_class("Y").expect("Y is coloured");
    let z = axis_chip_class("Z").expect("Z is coloured");
    let rot = axis_chip_class("Rotation").expect("Rotation is coloured");
    // Three DISTINCT axis colours (the acceptance counts three).
    let mut axes = vec![x, y, z];
    axes.sort_unstable();
    axes.dedup();
    assert_eq!(
        axes.len(),
        3,
        "X/Y/Z must be three distinct colours; got {x}/{y}/{z}"
    );
    // Rotation is its own hue, not a re-used axis colour.
    assert!(
        rot != x && rot != y && rot != z,
        "Rotation must carry a fourth distinct chip"
    );
    // Every NON-axis field is untinted — the chip rides only the spatial rows.
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

/// F-23 (c) — the chip rides the LABEL, not the value, and it is decorative (`aria-hidden`). Pin
/// `field_label`'s body: it must consult `axis_chip_class(label)` and mark the swatch aria-hidden.
#[test]
fn the_axis_chip_rides_the_label_and_is_aria_hidden() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let body = only_body(&code, "fn field_label(");
    assert!(
        body.contains("axis_chip_class(label)"),
        "field_label must derive the chip from the label; body was:\n{body}"
    );
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let body_src = only_body(&src, "fn field_label(");
    assert!(
        body_src.contains("aria-hidden"),
        "the colour swatch must be aria-hidden — a screen reader must not announce a bare colour"
    );
}

/// F-23 (a) — the TYPE field is a PICKER, and the T-082 wiring the pins protect stays in
/// `identity_tab`: it calls `type_picker` (not `text_field`) for Type, still reads
/// `a.asset_id.clone()`, still takes the multi-edit gate, and still routes the ONE commit seam.
/// (`identity_tab_commits_type_and_role_description_through_their_own_argument_slots` above still
/// pins the exact `commit_slot(...)` shape; this pins that the ENTRY became the picker.)
#[test]
fn the_type_field_is_the_searchable_picker_not_a_freetext_field() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let body = only_body(&code, "fn identity_tab(");
    assert!(
        body.contains("type_picker("),
        "the Type entry must be `type_picker`, not a bare text_field; body was:\n{body}"
    );
    // The picker still receives the live catalog and the gate, and still commits via commit_slot.
    assert!(
        body.contains("registry_items") && body.contains("g(diff.asset_id, opts.asset_id)"),
        "the picker must take the live catalog and the asset_id multi-edit gate"
    );
    // The commit seam is unchanged (also pinned by the T-082 test) — a picked leaf lands in the
    // asset_id slot alone.
    assert!(
        body.contains("commit_slot(targets, None, None, None, Some(asset_id), None)"),
        "the picker must commit into the asset_id slot alone, exactly as the field did"
    );
}

/// F-23 (a) — the picker keeps the freetext escape hatch (the T-785 `text_field`) behind an
/// Advanced affordance, so an unlisted id is still typeable. `type_picker`'s body must still call
/// `text_field` (the freetext half) and offer an "advanced" control.
#[test]
fn the_picker_keeps_a_freetext_advanced_escape_hatch() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let body = only_body(&code, "fn type_picker(");
    assert!(
        body.contains("text_field("),
        "type_picker must keep the T-785 text_field for unlisted ids; body was:\n{body}"
    );
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let body_src = only_body(&src, "fn type_picker(");
    assert!(
        body_src.to_lowercase().contains("advanced"),
        "the freetext field must sit behind an 'advanced' affordance"
    );
    // Empty is a first-class option: the picker offers a clear-to-faction-default row.
    assert!(
        body_src.contains("Faction default"),
        "empty = faction default must stay a first-class, reachable choice"
    );
}

/// F-23 (a) — the empty catalog shows CAUSE + RETRY, never a dead list, MIRRORING the T-800
/// dock vocabulary (`eden_dock_right::catalog_failure_view`). The empty branch turns on
/// `catalog_leaf_count == 0` and offers a retry.
#[test]
fn the_empty_catalog_shows_cause_and_retry_not_a_dead_list() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let body = only_body(&code, "fn type_picker(");
    assert!(
        body.contains("catalog_leaf_count("),
        "the empty state must key on catalog_leaf_count, not render a bare list"
    );
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let body_src = only_body(&src, "fn type_picker(");
    // The mirrored T-800 cause + a Retry control.
    assert!(
        body_src.contains("No modpack is configured"),
        "the empty state must state the cause (mirrored T-800 vocabulary)"
    );
    assert!(
        body_src.contains("Retry") && body.contains("reload()"),
        "the empty state must offer a working Retry (a full reload re-runs /registry)"
    );
}

/// F-23 (a) — ESC LAYERING: the picker popover's own Escape closes the POPOVER first and CONSUMES
/// the event (`stop_propagation`), so the modal's window listener does not also close the modal on
/// the same press. Layer order: picker → field (the advanced text_field's own Esc) → modal.
#[test]
fn the_picker_popover_consumes_its_own_escape_first() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let body = only_body(&code, "fn type_picker(");
    // Find the popover's keydown handler and prove it stops propagation AND closes the popover.
    assert!(
        body.contains("stop_propagation()"),
        "the popover's Escape must stop_propagation so the modal listener does not fire too"
    );
    assert!(
        body.contains("open.set(false)"),
        "the popover's Escape must close the popover layer itself"
    );
    // The advanced field is the T-785 text_field, which carries its OWN Escape (field layer) —
    // pinned already by `text_field_commits_on_blur_or_enter_and_never_remounts_mid_keystroke`
    // and the modal's Esc is the third layer (`attributes_modal_none_arm_still_closes...`).
}

/// F-23 (b) — REVERT restores the on-open snapshot as REAL writes, PER SLOT (not the homogeneous
/// T-788 batch, which would flatten a differing multi-selection onto one member's value).
/// `revert_to_snapshot` must walk the snapshot and call the SINGLE-slot commit ops per entry.
#[test]
fn revert_restores_per_slot_through_the_single_slot_commit_ops() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let body = only_body(&code, "fn revert_to_snapshot(");
    assert!(
        body.contains("attrs_update_position(") && body.contains("attrs_update_slot("),
        "Revert must restore BOTH the transform half and the identity/type half; body was:\n{body}"
    );
    // Per-slot (a loop over the snapshot), NOT the `_multi` batch — the batch is homogeneous and
    // cannot express each slot's own pre-open value.
    assert!(
        body.contains("for ") && body.contains("snapshot.get_value()"),
        "Revert must iterate the snapshot per slot"
    );
    assert!(
        !body.contains("_multi("),
        "Revert must NOT use the homogeneous multi batch — it would flatten differing slots"
    );
}

/// F-23 (b) — the snapshot is READ AT OPEN (the T-082 lesson): the capture effect tracks
/// `attrs_open` and NOT `doc_tick`, so a live edit does not overwrite the pre-open values, and the
/// modal STATES the model in one line. Pin the host body.
#[test]
fn the_revert_snapshot_is_captured_on_open_and_the_model_is_stated() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let host = only_body(&code, "pub fn AttributesModal(");
    // The capture reads read_attrs into the snapshot store, driven by the attrs_open effect.
    assert!(
        host.contains("snapshot.set_value("),
        "AttributesModal must capture the snapshot into its store on open"
    );
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let modal = only_body(&src, "fn modal_view(");
    // The one-line model statement lives in the panel (per spec).
    assert!(
        modal.contains("Edits apply live. Revert restores"),
        "the panel must state the revert model in one line"
    );
    // And a Revert control exists and calls the restore.
    let modal_code = only_body(&code, "fn modal_view(");
    assert!(
        modal_code.contains("revert_to_snapshot(snapshot)"),
        "the Revert button must call revert_to_snapshot"
    );
}

/* ─────────── T-818 — vehicle Attributes gains the dock Placed editor ─────────── */

/// Host route: when `read_attrs` is None, a vehicle id opens the vehicle editor instead of
/// closing. A hollow rewrite that keeps `close_attributes` on every None path goes RED.
#[test]
fn attributes_modal_routes_vehicles_to_the_vehicle_editor() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
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

/// The moved controls call the SAME mutators the Placed strip used — digest/undo parity.
/// Heading commits through number_field (T-785), not a raw on:change input.
#[test]
fn vehicle_attrs_view_wires_heading_cargo_crew_through_existing_mutators() {
    let code = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
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
    // Operator-visible section labels survive on live_source (live_code blanks string literals).
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let live = only_body(&src, "fn vehicle_attrs_view(");
    for label in ["\"Heading\"", "\"Add cargo\"", "\"Crew\"", "\"Cargo\""] {
        assert!(
            live.contains(label),
            "T-818: vehicle editor must show {label}; body was:\n{live}"
        );
    }
    // Multi-edit machinery must stay untouched for vehicles.
    assert!(
        !body.contains("attrs_multi_ids") && !body.contains("Gate::maybe"),
        "T-818: vehicle editor must not pull in T-649 multi-edit; body was:\n{body}"
    );
}

/// Seat model parity with the strip: driver/gunner/commander then cargo1..N.
#[test]
fn vehicle_attrs_seat_model_matches_the_strip() {
    let seats = super::seat_model(super::DEFAULT_CARGO_SEATS);
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
