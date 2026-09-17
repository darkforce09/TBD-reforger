use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

fn attrs_src() -> String {
    live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )))
}

/// **wave-127 F-2** — an Attributes x/y edit must not silently discard an authored Z.
///
/// `update_slot_position` terrain-follows on any x/y write (`pz = 0.0` when `z` is `None`). That
/// matches the JS oracle, whose caller then re-samples the DEM — but NOTHING re-samples after an
/// Attributes commit in this frontend, so the `0.0` was final: an operator who authored a rooftop
/// Z lost it the moment they nudged X by a metre, inside the same undo step as the X edit.
///
/// The fix is pinned at the FRONTEND CALLERS deliberately. `map-engine-core`'s mutator keeps its
/// documented byte-parity with `ydoc.updateSlotPosition`; these two functions read the current Z
/// and pass it back in, which makes the follow a no-op for this path alone.
///
/// A source pin because `editor_ops` is wasm32-only and `cargo test` cannot build it.
#[test]
fn an_attributes_x_or_y_commit_carries_the_slots_current_z_back_in() {
    let ops = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/attrs.rs"
    )));
    // Single-slot still goes through update_slot_position.
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
    // T-732 — multi stamps via update_entity_transforms; sticky-z still before the batch.
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
    // The read is conditional on the commit being able to zero a z at all — an explicit z write
    // or a rotation-only edit must not pay for an O(document) JSON read.
    let rows = only_body(&ops, "fn keep_z_rows(");
    assert!(
        rows.contains("(z.is_none() && (x.is_some() || y.is_some())).then(|| raw_slot_rows(core))"),
        "keep_z_rows must read the rows exactly when an x/y edit would otherwise zero the z; \
             body was:\n{rows}"
    );
    // And the read is off the EXACT raw row, not the materialized SoA: the SoA's `zs` is f32 (a
    // round-trip would rewrite the authored value) and it OMITS slots on hidden layers (T-665),
    // where a failed read is a zeroed z.
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

/// **wave-127 F-5** — the PLACEMENT commands must not flatten an authored Z either.
///
/// Same defect as F-2 above, one path over: `editor_ops::commit_positions` — the shared commit
/// behind the top strip's Align, Distribute and placement-pattern menus — used to write every
/// slot as `update_slot_position(.., Some(x), Some(y), None, ..)`, the exact shape the core
/// mutator terrain-follows to `pz = 0.0`. T-732 now batches through
/// `MissionDocCore::update_entity_transforms`, but the sticky-z resolution must still happen
/// BEFORE the batch is built.
///
/// Pinned here, beside its sibling, for the same reason: `editor_ops` is `wasm32`-only, so no
/// test inside it is built by the native harness.
#[test]
fn a_placement_commit_carries_each_slots_current_z_back_in() {
    let ops = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/data/store/operations/transform.rs"
    )));
    let body = only_body(&ops, "fn commit_positions(");
    // The old zeroing write, verbatim: x and y set, z hard-coded absent.
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
    // The rows are an O(document) JSON parse and this commits k entities, so the batch read must
    // be HOISTED above the per-entity loop and happen exactly once.
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
    // Vehicle branch still carries its own z into the patch.
    assert!(
        body.contains("z: Some(e.z)") || body.contains("z: Some(e.z,"),
        "the vehicle branch must still pass the vehicle's own z through; body was:\n{body}"
    );
    assert!(
        body.contains("is_slot: false"),
        "vehicle patches must be marked is_slot: false; body was:\n{body}"
    );
}

/// **T-777** — and neither may PASTE. A copy lands at the elevation it was copied from.
///
/// Third path in the same family as F-2 and F-5 above. `paste_at_cursor` pushed a
/// hard-coded ground value into `paste_slots`' `zs` column for every clipboard row, justified
/// in a comment as byte-parity with the flat-map JS oracle. The **operator set that parity
/// aside on 2026-08-08** — it was a migration safety net, never a contract — and the zero was
/// FINAL either way: the oracle's caller re-sampled the DEM and wrote the real elevation back,
/// nothing in this frontend does, so copying a rooftop entity dropped the copy to the ground
/// inside the paste's own undo step.
///
/// A SOURCE pin for the same reason as its two siblings: `editor_ops` is
/// `#![cfg(target_arch = "wasm32")]`, so the native harness builds nothing inside it and no
/// test here can call the function. The other half — that a non-zero elevation carried across
/// the seam actually survives into the document, and that a multi-slot paste does not hand one
/// entity another's z — is a live native test at
/// `crates/map-engine-core/tests/paste_keeps_authored_z.rs`. Neither half is sufficient alone:
/// this one cannot see the document, that one cannot see which value the frontend chooses.
#[test]
fn a_paste_carries_each_copied_slots_authored_z_into_the_copy() {
    // T-934.7 — the ops module was split; both the scrubbed and the RAW haystacks concatenate
    // every submodule so these file-wide absence pins keep their whole-module meaning.
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
    // FILE-WIDE, not scoped to the paste body: the failure mode is the literal coming back, and
    // it does not have to come back in the function it was removed from.
    let flattening_push = ["zs.push(", "0.0)"].concat();
    assert!(
        !ops.contains(&flattening_push),
        "no path may push a hard-coded ground elevation into a zs column — nothing re-samples \
             terrain afterwards, so that value is what the operator is left with"
    );
    // The overruled rationale must not survive as a live comment either; it would send the next
    // reader to restore the behaviour the operator just removed. Checked on RAW source because
    // `live_code` strips exactly the thing under test.
    assert!(
        !ops_raw.contains("DEM not ready"),
        "the paste's parity rationale was overruled on 2026-08-08 and must not be left standing"
    );

    let domain = live_code(crate::v2::core::test_support::editor_operations::DOMAIN_ENTITY);
    let body = only_body(&domain, "pub fn paste_at_cursor(");
    // Resolved through the SHARED reader. A second z-resolution vocabulary is its own defect
    // class here — F-2, F-5, F-6 and this path must all read a z the same way.
    assert!(
        body.contains("slot_z("),
        "paste_at_cursor must read each copied slot's authored z, not invent one; body was:\n\
             {body}"
    );
    // Resolved ONCE for the whole paste and HOISTED above the per-slot walk: building the
    // lookup inside the loop would be quadratic in the paste size.
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

    // ORDER CORRESPONDENCE — the whole reason this fix can be worse than the bug if it is
    // wrong. `zs[i]` must be the elevation of the row that minted `ids[i]`, and a mismatched
    // zip hands one entity another's elevation while looking perfectly green.
    //
    // Proved structurally, not by convention: BOTH pushes live inside the ONE walk over
    // `clip`, and each occurs EXACTLY ONCE in the whole function. One iteration therefore
    // appends exactly one element to each vector, in lockstep — a total, order-preserving map
    // from clipboard row to (id, z) pair. No zip, no id lookup, no second source to drift.
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
    // And nothing re-orders either vector between the walk that builds them and the single
    // hand-off. Scoped to that REGION on purpose: this asserts a property OF the region, not a
    // ban on a token (those are file-wide, see the top of this test).
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

/* ─────────── T-741 — multi-edit header honesty (wave-112 NIT-4) ─────────── */

/// Behaviour pin: mixed slot+vehicle selection must name the SLOT write set and disclose
/// that vehicles are excluded. RED under the original overclaim ("N entities selected") and
/// under a hollow that counts the FULL selection as the header number.
#[test]
fn attrs_multi_subtitle_counts_slots_and_names_excluded_vehicles() {
    // Original defect: 2 slots + 3 vehicles under Ctrl+A.
    assert_eq!(
        super::attrs_multi_subtitle(2, 5),
        "2 slots selected · multi-edit · vehicles excluded"
    );
    // Slot-only multi-edit stays terse.
    assert_eq!(
        super::attrs_multi_subtitle(3, 3),
        "3 slots selected · multi-edit"
    );
    // Hollow shape 1 — original overclaim wording (filtered count, but "entities").
    assert_ne!(
        super::attrs_multi_subtitle(2, 5),
        "2 entities selected · multi-edit"
    );
    // Hollow shape 2 — counting the FULL selection as the write-set size.
    assert_ne!(
        super::attrs_multi_subtitle(2, 5),
        "5 entities selected · multi-edit"
    );
    assert_ne!(
        super::attrs_multi_subtitle(2, 5),
        "5 slots selected · multi-edit"
    );
}

/// Wiring pin (`live_code` / `only_body`): the modal must CALL the honesty helper with both
/// the filtered slot count and the live selection length — an inlined old `format!` cannot
/// satisfy this (literals blanked; the call shape is what remains).
///
/// Wave-137 F1: also pin value *flow* through `AttributesModal` — a dead
/// `let _ = selection_len()` (hollow B) or binding `selection_n` then passing
/// `multi.len()` / `multi_n` into `modal_view` (hollow B2) must go RED.
#[test]
fn modal_view_routes_multi_subtitle_through_the_honesty_helper() {
    let src = attrs_src();
    let body = only_body(&src, "fn modal_view(");
    assert!(
            body.contains("attrs_multi_subtitle(multi_n, selection_n)"),
            "modal_view must format the multi header via attrs_multi_subtitle(multi_n, selection_n); body was:\n{body}"
        );
    let host = only_body(&src, "pub fn AttributesModal(");
    // (1) Assignment, not a dead call — hollow B (`let _ = selection_len(); let selection_n = multi.len()`) RED.
    assert!(
            host.contains("let selection_n = website_map_engine::editing::host::selection_len()"),
            "AttributesModal must bind `let selection_n = website_map_engine::editing::host::selection_len()` (not a discarded call); body was:\n{host}"
        );
    // (2) That binding must be the modal_view selection-length argument — hollow B2
    // (`modal_view(..., multi.len(), ...)` while keeping the binding) RED.
    let compact = host.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
            compact.contains("modal_view( attrs, multi, selection_n,")
                || compact.contains("modal_view(attrs, multi, selection_n,"),
            "AttributesModal must pass `selection_n` into modal_view (not multi.len()/multi_n); body was:\n{host}"
        );
}

/// Copy pin (`live_source`): banner says "every selected slot", never the overclaiming
/// "every selected entity". Second hollow: the old header format string must be gone from
/// `modal_view`.
#[test]
fn multi_edit_copy_names_slots_not_every_selected_entity() {
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
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

/// `attrs_multi_ids` still filters to SoA slot ids — the subset the header is honest about.
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
