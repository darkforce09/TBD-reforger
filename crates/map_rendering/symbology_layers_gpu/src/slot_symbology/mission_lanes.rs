//! **Role:** the mission lanes beside the slots: vehicles, briefing markers with their captions,
//! comments, and the placement preview ring.
//! **Position:** `symbology_layers_gpu::slot_symbology`, methods of [`SlotSymbology`]; the
//! Mission Creator's document host, select tool, armed placement and pointer gestures and the
//! mortar map picker call them.
//! **Signals & state:** the slot bridge's comment columns and selection, and the vehicle, marker,
//! comment and preview lanes.
//! **Invariants:** a bind uploads its own lane and never touches the slot pick columns
//! (`last_ids`); every bind is a no-op until the slot atlas is armed; an empty bind removes its
//! lane; a comment draws selected only when its id is in the selection.

use super::view::SlotSymbology;
use crate::icon_uniforms::{convert_icon_world_to_anchor, sprite_atlas_for};
use gpu_frame::frame::{DrawBatch, DrawPayload, InstanceBuffer, TextRun};
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use overlay_instances::symbols::SLOT_ICON_STRIDE;
use renderer_core::packet_bindings;
use unit_symbology::classification::SIDE_BLUFOR_RGBA;

impl SlotSymbology<'_> {
    /// Vehicles bind.
    pub fn vehicles_bind(&mut self, xy: &[f32]) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let bytes = overlay_instances::symbols::pack_vehicle_instances(xy);
        let vis = !bytes.is_empty();
        if !vis {
            self.remove_lane(LaneRole::MissionVehicles);
            return;
        }
        self.upload_slot_role_lane(LaneRole::MissionVehicles, &bytes, true);
    }

    /// Falls back to `vehicles_bind`'s disc lane when the atlas carries no symbology cells. Empty `xy` clears the lane.
    pub fn vehicles_bind_symbology(
        &mut self,
        xy: &[f32],
        aliases: Vec<String>,
        side_tints_rgba: &[u8],
        headings_deg: &[f32],
    ) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        if xy.len() < 2 {
            self.remove_lane(LaneRole::MissionVehicles);
            return;
        }
        let Some(base) = self.slot_bridge.symbology_base else {
            self.vehicles_bind(xy);
            return;
        };
        let n = xy.len() / 2;
        let tints: Vec<[u8; 4]> = (0..n)
            .map(|i| {
                side_tints_rgba
                    .get(i * 4..i * 4 + 4)
                    .map_or(SIDE_BLUFOR_RGBA, |s| [s[0], s[1], s[2], s[3]])
            })
            .collect();
        let bytes = overlay_instances::symbols::pack_vehicle_symbology(
            xy,
            &aliases,
            &tints,
            headings_deg,
            self.slot_m_per_px(),
            base,
        );
        self.upload_slot_role_lane(LaneRole::MissionVehicles, &bytes, true);
    }

    /// Markers bind.
    pub fn markers_bind(
        &mut self,
        xy: &[f32],
        side_tints_rgba: &[u8],
        icons: Vec<String>,
        captions: Vec<String>,
    ) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let n = xy.len() / 2;
        if n == 0 {
            self.remove_lane(LaneRole::MissionMarkers);
            return;
        }

        let mut icon_bytes = Vec::with_capacity(n * SLOT_ICON_STRIDE);
        for i in 0..n {
            let x = xy[i * 2];
            let y = xy[i * 2 + 1];
            let rgba = if side_tints_rgba.len() >= (i + 1) * 4 {
                [
                    side_tints_rgba[i * 4],
                    side_tints_rgba[i * 4 + 1],
                    side_tints_rgba[i * 4 + 2],
                    side_tints_rgba[i * 4 + 3],
                ]
            } else {
                unit_symbology::classification::SIDE_BLUFOR_RGBA
            };
            let glyph = icons
                .get(i)
                .map_or(unit_symbology::markers::MarkerGlyph::Disc, |a| {
                    unit_symbology::markers::marker_glyph_for_alias(a)
                }) as u16;
            render_primitives::text::pack::pack_icon_instance(
                &mut icon_bytes,
                x,
                y,
                overlay_instances::symbols::SLOT_RING_PX,
                glyph,
                render_primitives::text::pack::pack_rgba_u32(rgba),
            );
        }

        let mut caption_bytes =
            unit_symbology::markers::pack_marker_caption_bytes(xy, &captions, self.zoom());
        if !caption_bytes.is_empty() {
            let _ = self.text_atlas.ensure_text_atlas();
        }
        self.upload_marker_composite(&mut icon_bytes, &mut caption_bytes);
    }

    /// Upload marker composite.
    pub(super) fn upload_marker_composite(
        &mut self,
        icon_bytes: &mut [u8],
        caption_bytes: &mut [u8],
    ) {
        const STRIDE: usize = SLOT_ICON_STRIDE;
        #[allow(clippy::cast_possible_truncation)]
        const STRIDE_U32: u32 = SLOT_ICON_STRIDE as u32;
        if icon_bytes.is_empty() || !icon_bytes.len().is_multiple_of(STRIDE) {
            self.remove_lane(LaneRole::MissionMarkers);
            return;
        }
        use wgpu::util::DeviceExt;
        convert_icon_world_to_anchor(icon_bytes);
        let device = self.lanes.layer_context().device();
        let icons = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("marker-icons"),
            contents: icon_bytes,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        #[allow(clippy::cast_possible_truncation)]
        let icon_count = (icon_bytes.len() / STRIDE) as u32;
        let captions = if caption_bytes.is_empty() || !caption_bytes.len().is_multiple_of(STRIDE) {
            None
        } else {
            convert_icon_world_to_anchor(caption_bytes);
            let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("marker-captions"),
                contents: caption_bytes,
                usage: wgpu::BufferUsages::VERTEX,
            });
            #[allow(clippy::cast_possible_truncation)]
            let count = (caption_bytes.len() / STRIDE) as u32;
            Some((buf, count))
        };
        let lane = lane_id(LaneRole::MissionMarkers);
        self.upsert_lane(
            LaneRole::MissionMarkers,
            DrawBatch {
                lane,
                visible: true,
                pipeline: packet_bindings::PIPE_ICON,
                payload: DrawPayload::SpritesWithText {
                    sprites: InstanceBuffer::whole(icons, STRIDE_U32, icon_count),
                    atlas: sprite_atlas_for(LaneRole::MissionMarkers),
                    text: captions.map(|(buf, count)| TextRun {
                        lane,
                        glyphs: InstanceBuffer::whole(buf, STRIDE_U32, count),
                        atlas: packet_bindings::BIND_TEXT_ATLAS,
                        pipeline: packet_bindings::PIPE_TEXT,
                    }),
                },
            },
        );
    }

    /// Bind the comment lane at `xy` (two `f32` per comment): each comment draws the neutral
    /// speech bubble (`COMMENT_NOTE_RGBA`), and the selection colour only when its id is selected.
    pub fn comments_bind(&mut self, xy: &[f32]) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        self.slot_bridge.comment_xy = xy.to_vec();
        let n = xy.len() / 2;
        if n == 0 {
            self.slot_bridge.comment_ids.clear();
            self.remove_lane(LaneRole::MissionComments);
            return;
        }

        let selected: Vec<bool> = (0..n)
            .map(|i| {
                self.slot_bridge
                    .comment_ids
                    .get(i)
                    .is_some_and(|id| self.slot_bridge.selected_ids.contains(id))
            })
            .collect();
        let bytes = match self.slot_bridge.symbology_base {
            Some(base) => overlay_instances::symbols::pack_comment_instances(
                xy,
                &selected,
                self.slot_m_per_px(),
                base,
            ),

            None => {
                let mut b = Vec::with_capacity(n * SLOT_ICON_STRIDE);
                for (i, sel) in selected.iter().enumerate() {
                    let tint = render_primitives::text::pack::pack_rgba_u32(if *sel {
                        overlay_instances::symbols::SLOT_SELECTED_RGBA
                    } else {
                        overlay_instances::symbols::COMMENT_NOTE_RGBA
                    });
                    render_primitives::text::pack::pack_icon_instance(
                        &mut b,
                        xy[i * 2],
                        xy[i * 2 + 1],
                        overlay_instances::symbols::SLOT_RING_PX,
                        overlay_instances::symbols::SLOT_GLYPH_RING,
                        tint,
                    );
                }
                b
            }
        };
        self.upload_slot_role_lane(LaneRole::MissionComments, &bytes, true);
    }

    /// `ids[i]` names comment `i` in the same id space [`Self::set_selection`] receives. Without them (what [`Self::comments_bind`] passes) the bubble and its neutral colour still land, but no comment can ever be shown as selected — the engine has no way to know which one is. Ids shorter than `xy` mark only the rows they cover.
    pub fn comments_bind_ids(&mut self, xy: &[f32], ids: Vec<String>) {
        self.slot_bridge.comment_ids = ids;
        self.comments_bind(xy);
    }

    /// Refresh comment lane.
    pub(super) fn refresh_comment_lane(&mut self) {
        if self.slot_bridge.comment_xy.is_empty() {
            return;
        }
        let xy = std::mem::take(&mut self.slot_bridge.comment_xy);
        self.comments_bind(&xy);
    }

    /// Set place preview.
    pub fn set_place_preview(&mut self, world_x: f32, world_y: f32) {
        if !self.slot_bridge.atlas_ready {
            return;
        }
        let tint = render_primitives::text::pack::pack_rgba_u32([173, 198, 255, 140]);
        let mut b = Vec::with_capacity(SLOT_ICON_STRIDE);
        render_primitives::text::pack::pack_icon_instance(
            &mut b,
            world_x,
            world_y,
            overlay_instances::symbols::SLOT_RING_PX,
            overlay_instances::symbols::SLOT_GLYPH_RING,
            tint,
        );
        self.upload_slot_role_lane(LaneRole::SlotPlacePreview, &b, true);
        self.lanes.mark_damage();
    }

    /// Clear place preview.
    pub fn clear_place_preview(&mut self) {
        self.remove_lane(LaneRole::SlotPlacePreview);
        self.lanes.mark_damage();
    }
}
