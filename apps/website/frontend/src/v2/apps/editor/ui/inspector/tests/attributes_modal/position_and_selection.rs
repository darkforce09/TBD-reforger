//! Attributes modal position and selection tests.

use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

fn attrs_src() -> String {
    live_code(super::ATTRIBUTES_MODAL_SOURCE)
}

#[test]
fn an_attributes_x_or_y_commit_carries_the_slots_current_z_back_in() {
    let ops = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/attrs.rs"
    )));
    {
        let f = "pub fn attrs_update_position(";
        let body = only_body(&ops, f);
        let resolve = body.find("z.or_else(").unwrap_or_else(|| {
            panic!("{f} must resolve a missing z before writing; body was:\n{body}")
        });
        let write = body
            .find("core.update_slot_position(")
            .unwrap_or_else(|| panic!("{f} must still write through the core mutator"));
        assert!(
            resolve < write,
            "{f} must resolve the sticky z BEFORE the write; resolve at {resolve}, write at \
                 {write}"
        );
        assert!(
            body.contains("slot_z("),
            "{f} must read the slot's CURRENT z, not invent one; body was:\n{body}"
        );
    }
    {
        let f = "pub fn attrs_update_position_multi(";
        let body = only_body(&ops, f);
        let resolve = body.find("z.or_else(").unwrap_or_else(|| {
            panic!("{f} must resolve a missing z before writing; body was:\n{body}")
        });
        let write = body.find("update_entity_transforms(").unwrap_or_else(|| {
            panic!("{f} must commit via update_entity_transforms (T-732 atomic batch)")
        });
        assert!(
            resolve < write,
            "{f} must resolve the sticky z BEFORE the batch write; resolve at {resolve}, write \
                 at {write}"
        );
        assert!(
            body.contains("slot_z("),
            "{f} must read the slot's CURRENT z, not invent one; body was:\n{body}"
        );
        let per_id = ["core.update_slot", "_position(id,"].concat();
        assert!(
            !body.contains(&per_id),
            "T-732: {f} must not call per-id update_slot_position (N undo steps)"
        );
    }
    let rows = only_body(&ops, "fn keep_z_rows(");
    assert!(
        rows.contains("(z.is_none() && (x.is_some() || y.is_some())).then(|| raw_slot_rows(core))"),
        "keep_z_rows must read the rows exactly when an x/y edit would otherwise zero the z; \
             body was:\n{rows}"
    );
    let live_ops = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/attrs.rs"
    )));
    let read = only_body(&live_ops, "fn slot_z(");
    assert!(
        read.contains("\"position\"") && read.contains("\"z\""),
        "slot_z must read `position.z` off the raw slot row; body was:\n{read}"
    );
    assert!(
        !read.contains("materialize"),
        "slot_z must not go through the f32 SoA — it drops hidden-layer slots and rounds the \
             value it exists to preserve; body was:\n{read}"
    );
}

#[test]
fn a_placement_commit_carries_each_slots_current_z_back_in() {
    let ops = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/transform.rs"
    )));
    let body = only_body(&ops, "fn commit_positions(");
    assert!(
        !body.contains("Some(t.x), Some(t.y), None, None"),
        "commit_positions must not write `z = None` on an x/y move — that stores pz = 0.0 and \
             the authored z is gone; body was:\n{body}"
    );
    let read = body.find("slot_z(").unwrap_or_else(|| {
        panic!(
            "commit_positions must read each slot's CURRENT z, not invent one; body was:\n{body}"
        )
    });
    let write = body.find("update_entity_transforms(").unwrap_or_else(|| {
        panic!("commit_positions must commit via update_entity_transforms (T-732 atomic batch)")
    });
    assert!(
        read < write,
        "commit_positions must resolve the sticky z BEFORE the batch write; read at {read}, \
             write at {write}"
    );
    let rows = body.find("keep_z_rows(").unwrap_or_else(|| {
        panic!(
            "commit_positions must resolve its rows through keep_z_rows — \
                 a second z-resolution path is its own defect; body was:\n{body}"
        )
    });
    let loop_at = body
        .find("for (e, t) in")
        .unwrap_or_else(|| panic!("commit_positions must still walk entities/targets pairwise"));
    assert!(
        rows < loop_at,
        "the keep_z_rows read must be hoisted ABOVE the per-entity loop (read at {rows}, loop \
             at {loop_at}) — one O(document) parse per BATCH, not per entity"
    );
    assert_eq!(
        body.matches("keep_z_rows(").count(),
        1,
        "exactly one keep_z_rows call — more than one means the document is re-parsed per \
             entity; body was:\n{body}"
    );
    assert!(
        body.contains("z: Some(e.z)") || body.contains("z: Some(e.z,"),
        "the vehicle branch must still pass the vehicle's own z through; body was:\n{body}"
    );
    assert!(
        body.contains("is_slot: false"),
        "vehicle patches must be marked is_slot: false; body was:\n{body}"
    );
}

#[test]
fn a_paste_carries_each_copied_slots_authored_z_into_the_copy() {
    let ops_raw = [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/hosted_commands/slot_attributes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/loadout_commands.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/hosted_commands/slot_loadouts.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/hosted_commands/composition_library.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/data/store/operations/compositions.rs"
        )),
        crate::v2::core::test_support::editor_operations::CONTEXT,
        crate::v2::core::test_support::editor_operations::ENTITY,
        crate::v2::core::test_support::editor_operations::DOMAIN_ENTITY,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/editing/hosted_commands/selection_transform.rs"
        )),
    ]
    .concat();
    let ops = live_code(&ops_raw);
    let flattening_push = ["zs.push(", "0.0)"].concat();
    assert!(
        !ops.contains(&flattening_push),
        "no path may push a hard-coded ground elevation into a zs column — nothing re-samples \
             terrain afterwards, so that value is what the operator is left with"
    );
    assert!(
        !ops_raw.contains("DEM not ready"),
        "the paste's parity rationale was overruled on 2026-08-08 and must not be left standing"
    );

    let domain = live_code(crate::v2::core::test_support::editor_operations::DOMAIN_ENTITY);
    let body = only_body(&domain, "pub fn paste_at_cursor(");
    assert!(
        body.contains("slot_z("),
        "paste_at_cursor must read each copied slot's authored z, not invent one; body was:\n\
             {body}"
    );
    let rows = body.find("let z_rows").unwrap_or_else(|| {
        panic!("paste_at_cursor must resolve its z lookup up front; body was:\n{body}")
    });
    let loop_at = body
        .find("for slot in &clip")
        .unwrap_or_else(|| panic!("paste_at_cursor must still walk the clipboard rows"));
    assert!(
        rows < loop_at,
        "the z lookup must be built ABOVE the per-slot loop (built at {rows}, loop at \
             {loop_at}) — once per PASTE, not once per slot"
    );
    assert_eq!(
        body.matches("let z_rows").count(),
        1,
        "exactly one z lookup; a second one means a second z-resolution path"
    );

    let per_slot = only_body(&body, "for slot in &clip");
    assert!(
        per_slot.contains("ids.push(mint_id("),
        "the id mint must stay inside the clipboard walk; loop body was:\n{per_slot}"
    );
    assert!(
        per_slot.contains("zs.push("),
        "the z push must sit in the SAME iteration as the id mint, or the two vectors are only \
             conventionally aligned; loop body was:\n{per_slot}"
    );
    for (needle, what) in [("ids.push(mint_id(", "id mint"), ("zs.push(", "z push")] {
        assert_eq!(
            body.matches(needle).count(),
            1,
            "exactly one {what} in paste_at_cursor — a second one appends off-cadence and \
                 shifts every later index by one"
        );
    }
    let hand_off = body.find("core.paste_slots(").unwrap_or_else(|| {
        panic!("paste_at_cursor must still commit through the bulk paste mutator")
    });
    assert!(
        loop_at < hand_off,
        "the parallel arrays must be built before they are handed off"
    );
    for reorder in [
        ".sort",
        ".reverse(",
        ".dedup",
        ".retain(",
        ".swap(",
        ".rotate_",
    ] {
        assert!(
            !body[loop_at..hand_off].contains(reorder),
            "nothing may `{reorder}` between building the paste arrays and handing them to \
                 paste_slots — the index correspondence is the contract"
        );
    }
}

#[test]
fn attrs_multi_subtitle_counts_slots_and_names_excluded_vehicles() {
    assert_eq!(
        super::attrs_multi_subtitle(2, 5),
        "2 slots selected · multi-edit · vehicles excluded"
    );
    assert_eq!(
        super::attrs_multi_subtitle(3, 3),
        "3 slots selected · multi-edit"
    );
    assert_ne!(
        super::attrs_multi_subtitle(2, 5),
        "2 entities selected · multi-edit"
    );
    assert_ne!(
        super::attrs_multi_subtitle(2, 5),
        "5 entities selected · multi-edit"
    );
    assert_ne!(
        super::attrs_multi_subtitle(2, 5),
        "5 slots selected · multi-edit"
    );
}

#[test]
fn modal_view_routes_multi_subtitle_through_the_honesty_helper() {
    let src = attrs_src();
    let body = only_body(&src, "fn modal_view(");
    assert!(
            body.contains("attrs_multi_subtitle(multi_n, selection_n)"),
            "modal_view must format the multi header via attrs_multi_subtitle(multi_n, selection_n); body was:\n{body}"
        );
    let host = only_body(&src, "pub fn AttributesModal(");
    assert!(
            host.contains("let selection_n = website_map_engine::editing::host::selection_len()"),
            "AttributesModal must bind `let selection_n = website_map_engine::editing::host::selection_len()` (not a discarded call); body was:\n{host}"
        );
    let compact = host.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
            compact.contains("modal_view( attrs, multi, selection_n,")
                || compact.contains("modal_view(attrs, multi, selection_n,"),
            "AttributesModal must pass `selection_n` into modal_view (not multi.len()/multi_n); body was:\n{host}"
        );
}

#[test]
fn multi_edit_copy_names_slots_not_every_selected_entity() {
    let src = live_source(super::ATTRIBUTES_MODAL_SOURCE);
    let body = only_body(&src, "fn modal_view(");
    assert!(
        body.contains("every selected slot"),
        "differing-fields banner must say every selected slot; body was:\n{body}"
    );
    assert!(
        !body.contains("every selected entity"),
        "banner must not overclaim vehicles as editable entities; body was:\n{body}"
    );
    assert!(
        !body.contains("entities selected · multi-edit"),
        "modal_view must not keep the old entities-selected header format; body was:\n{body}"
    );
}

#[test]
fn attrs_multi_ids_still_filters_selection_to_slot_soa() {
    let ops = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/editing/hosted_commands/slot_attributes.rs"
    )));
    let body = only_body(&ops, "pub fn attrs_multi_ids(open_id: &str) -> Vec<String>");
    assert!(
        body.contains("soa.ids.iter().any(|r| r == s)"),
        "attrs_multi_ids must keep filtering to slot SoA ids; body was:\n{body}"
    );
    let host = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/editing/host.rs"
    )));
    let sel = only_body(&host, "pub fn selection_len() -> usize");
    assert!(
        sel.contains("selection.borrow().len()"),
        "selection_len must read the live selection length; body was:\n{sel}"
    );
}
