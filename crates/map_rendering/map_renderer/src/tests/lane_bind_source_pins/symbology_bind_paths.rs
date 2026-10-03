//! Source-text pin: the slot atlas, the slot and vehicle symbology binds and the comment lane
//! refresh keep their upload paths.
//!
//! **Role:** reads the bind function bodies from the source text and asserts what they call.
//! **Position:** mounted from `lane_bind_source_pins/mod.rs` under the crate root.
//! **Signals & state:** none.
//! **Invariants:** the `include_str!` sources resolve from this folder and are exactly the files
//! that hold the pinned bodies; a missing signature fails the case rather than passing it.

const ENGINE: &str = concat!(
    include_str!(
        "../../../../../../crates/map_rendering/symbology_layers_gpu/src/slot_symbology/atlas.rs"
    ),
    "\n",
    include_str!(
        "../../../../../../crates/map_rendering/symbology_layers_gpu/src/slot_symbology/slot_lane.rs"
    ),
    "\n",
    include_str!(
        "../../../../../../crates/map_rendering/symbology_layers_gpu/src/slot_symbology/mission_lanes.rs"
    )
);

fn body(sig: &str) -> String {
    let start = ENGINE
        .find(sig)
        .unwrap_or_else(|| panic!("T-808: engine.rs has no `{sig}`"));
    assert!(
        !ENGINE[start + sig.len()..].contains(sig),
        "T-808: `{sig}` is not unique in engine.rs — the extractor would pin the wrong body"
    );
    let after = &ENGINE[start..];
    let brace = after.find('{').expect("body");
    let mut depth = 0usize;
    let mut end = brace;
    for (i, ch) in after[brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = brace + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    after[..end].to_string()
}

#[test]
fn ensure_slot_atlas_widens_and_records_the_symbology_base() {
    let b = body("pub fn ensure_slot_atlas");
    assert!(
        b.contains("extend_atlas_with_unit_glyphs"),
        "T-808: ensure_slot_atlas must widen the strip via \
             slots_gpu::extend_atlas_with_unit_glyphs; body:\n{b}"
    );
    assert!(
        b.contains("symbology_base = Some(wide.base_cells)"),
        "T-808: ensure_slot_atlas must record the runtime symbology base; body:\n{b}"
    );
    assert!(
        b.contains("symbology_base = None"),
        "T-808: an unparseable strip must CLEAR symbology_base, not keep a stale one; \
             body:\n{b}"
    );
    assert!(
        b.contains("atlas_ready = true"),
        "T-808: ensure_slot_atlas must still arm the slot bridge; body:\n{b}"
    );
}

#[test]
fn slots_bind_symbology_keeps_the_role_and_heading_columns() {
    let b = body("pub fn slots_bind_symbology");
    assert!(
        b.contains("last_roles = roles"),
        "T-808: slots_bind_symbology must store the ROLE column; body:\n{b}"
    );
    assert!(
        b.contains("last_headings = headings_deg"),
        "T-808: slots_bind_symbology must store the HEADING column; body:\n{b}"
    );
    assert!(
        b.contains("rematerialize_slot_lane"),
        "T-808: slots_bind_symbology must re-pack the slot lane from the new columns; \
             body:\n{b}"
    );

    assert!(
        !ENGINE.contains("fn slots_bind_soa"),
        "the slot bridge binds through slots_bind_symbology alone; no SoA bind may bypass it"
    );
}

#[test]
fn vehicles_bind_symbology_uploads_its_lane_and_falls_back_without_symbology() {
    let b = body("pub fn vehicles_bind_symbology");
    assert!(
        b.contains("MissionVehicles"),
        "T-808: vehicles_bind_symbology must upload LaneRole::MissionVehicles; body:\n{b}"
    );
    assert!(
        b.contains("pack_vehicle_symbology"),
        "T-808: vehicles_bind_symbology must pack via slots_gpu::pack_vehicle_symbology; \
             body:\n{b}"
    );
    assert!(
        b.contains("symbology_base") && b.contains("self.vehicles_bind(xy)"),
        "T-808: vehicles_bind_symbology must fall back to the disc lane when the atlas \
             carries no symbology cells; body:\n{b}"
    );
    assert!(
        !b.contains("last_ids") && !b.contains("slots_bind_soa"),
        "T-808: vehicles_bind_symbology must not enter the slot pick / SoA bridge; body:\n{b}"
    );
}

#[test]
fn refresh_comment_lane_repacks_from_cache_and_is_actually_called() {
    let b = body("fn refresh_comment_lane(&mut self)");
    assert!(
        b.contains("comment_xy"),
        "T-808: refresh_comment_lane must re-pack from the cached comment_xy; body:\n{b}"
    );
    assert!(
        b.contains("self.comments_bind(&xy)"),
        "T-808: refresh_comment_lane must re-pack through comments_bind; body:\n{b}"
    );
    for feed in ["pub fn set_selection", "fn sync_slot_zoom_uniform"] {
        let f = body(feed);
        assert!(
            f.contains("refresh_comment_lane"),
            "T-808: `{feed}` must refresh the comment lane; body:\n{f}"
        );
    }
}
