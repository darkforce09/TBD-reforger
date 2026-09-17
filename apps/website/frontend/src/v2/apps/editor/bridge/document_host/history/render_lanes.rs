//! Builds render lanes from the current mission document.

use super::*;

/// Builds comment glyph positions from document rows.
pub(super) fn comment_lane_xy(doc: &MissionDocCore) -> Vec<f32> {
    crate::v2::apps::editor::mission_editor::comment_lane_xy(&doc.comments_json())
}

/// Builds comment identifiers aligned with glyph positions.
pub(super) fn comment_lane_ids(doc: &MissionDocCore) -> Vec<String> {
    crate::v2::apps::editor::mission_editor::comment_lane_ids(&doc.comments_json())
}

/// Returns role labels aligned with materialized slots.
pub(crate) fn soa_roles(soa: &SlotSoa) -> Vec<String> {
    soa.role_idx
        .iter()
        .map(|&i| soa.roles.get(i as usize).cloned().unwrap_or_default())
        .collect()
}

/// Builds vehicle glyph positions, labels, colors, and headings.
pub(crate) fn vehicle_lane_fields() -> (Vec<f32>, Vec<String>, Vec<u8>, Vec<f32>) {
    let rows = website_map_engine::editing::hosted_commands::vehicle_rows();
    let mut xy = Vec::with_capacity(rows.len() * 2);
    let mut aliases = Vec::with_capacity(rows.len());
    let mut tints = Vec::with_capacity(rows.len() * 4);
    let mut headings = Vec::with_capacity(rows.len());
    for r in rows {
        let Some((x, y)) = r.xy else { continue };
        #[allow(clippy::cast_possible_truncation)]
        {
            xy.push(x as f32);
            xy.push(y as f32);
            headings.push(r.rotation.unwrap_or(0.0) as f32);
        }
        let side = r
            .faction_id
            .strip_prefix("faction-")
            .unwrap_or(&r.faction_id);
        tints.extend_from_slice(
            &website_map_engine::overlay::symbology::roles::classify::side_rgba(side),
        );
        aliases.push(r.resource_name);
    }
    (xy, aliases, tints, headings)
}

/// Builds marker positions, colors, and captions.
pub(super) fn marker_lane_xy_tints(
    doc: &MissionDocCore,
) -> (Vec<f32>, Vec<u8>, Vec<String>, Vec<String>) {
    crate::v2::apps::editor::mission_editor::marker_lane_fields(&doc.briefing_marker_rows_json())
}

/// Uploads squad hierarchy links to the renderer.
pub(super) fn upload_squad_links(e: &mut RenderEngine, doc: &MissionDocCore, soa: &SlotSoa) {
    let mut xy_by_slot: HashMap<String, (f32, f32)> = HashMap::with_capacity(soa.ids.len());
    for (i, id) in soa.ids.iter().enumerate() {
        let x = soa.xy[i * 2];
        let y = soa.xy[i * 2 + 1];
        xy_by_slot.insert(id.clone(), (x, y));
    }
    let inputs = website_map_engine::editing::picking::squad_link_inputs(doc);
    let verts = build_squad_link_segments(&inputs, &xy_by_slot);
    #[allow(clippy::cast_possible_truncation)]
    let segment_count = (verts.len() / 12) as u32;
    e.upload_hairline_segments(role_id::SQUAD_LINKS, &verts, segment_count, true);
}

/// Uploads tactical graphic segments to the renderer.
pub(super) fn upload_tactical_graphics(e: &mut RenderEngine, doc: &MissionDocCore) {
    use crate::v2::apps::editor::bridge::tactical_graphics as tg;
    let mut rows = tg::live_tactical_graphics(doc);
    tactical_graphics_authoring::apply_tactical_drag_preview(&mut rows);
    let verts = tg::tactical_lane_verts(
        &rows,
        tactical_graphics_authoring::selected_tactical_graphic().as_deref(),
    );
    e.upload_hairline_segments(
        role_id::MISSION_ZONES,
        &verts,
        tg::lane_segment_count(&verts),
        true,
    );
}

/// Rebinds tactical graphic vertices from the active document.
pub fn refresh_tactical_lane() {
    HISTORY_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let mut eg = ctx.engine.borrow_mut();
        let Some(e) = eg.as_mut() else {
            return;
        };
        let d = ctx.doc.borrow();
        if let Some(doc) = d.as_ref() {
            upload_tactical_graphics(e, doc);
        }
    });
}
