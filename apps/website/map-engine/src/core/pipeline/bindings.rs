//! Role: bindings.
//! Position: `core/pipeline` in the graphics engine.
//! Signals & state: the frame packet's pipeline and bind-group tables, and the lane → binding
//! policy that fills them.
//! Invariants: **this is the half of the old encoder that could not cross.** Every decision
//! the renderer used to make by matching on `LaneRole` — forest density vs plain textured,
//! text pipeline vs icon pipeline, which atlas a sprite lane samples — is made here and
//! travels to `website-graphics-engine` as a `PipelineId` / `BindGroupId` on the batch.
//!
//! The third of the old encoder's three lane switches has no entry here on purpose.
//! `encoder.rs:105-107` re-routed `WorldLabels | WorldRoadLabels | WorldTownLabels` from the
//! icon pipeline onto the text pipeline; those three lanes now build a `DrawPayload::Text`
//! at upload time in `renderers/text/lanes.rs`, so the routing IS the payload and there is
//! nothing left to ask.

use crate::core::pipeline::draw_order::LaneRole;
use website_graphics_engine::frame::{BindGroupId, LaneId, PipelineId};

/// `vs_quad` — axis-aligned coloured quads.
pub(crate) const PIPE_QUAD: PipelineId = PipelineId(0);

/// `vs_textured` — one sampled rect.
pub(crate) const PIPE_TEXTURED: PipelineId = PipelineId(1);

/// The density variant of the textured pipeline.
pub(crate) const PIPE_DENSITY: PipelineId = PipelineId(2);

/// `LineList`.
pub(crate) const PIPE_LINE: PipelineId = PipelineId(3);

/// `vs_building` — oriented quads carrying their own basis.
pub(crate) const PIPE_BUILDING: PipelineId = PipelineId(4);

/// Indexed triangle fills.
pub(crate) const PIPE_POLYGON: PipelineId = PipelineId(5);

/// Atlas-sampled sprites.
pub(crate) const PIPE_ICON: PipelineId = PipelineId(6);

/// Atlas-sampled glyphs.
pub(crate) const PIPE_TEXT: PipelineId = PipelineId(7);

/// The storage-buffer icon pipeline the compute-culled indirect draws bind.
pub(crate) const PIPE_ICON_STORAGE32: PipelineId = PipelineId(8);

/// How many pipeline slots a frame packet carries.
pub(crate) const PIPELINE_SLOTS: usize = 9;

/// Group 0 for every draw.
pub(crate) const BIND_CAMERA: BindGroupId = BindGroupId(0);

/// The glyph cell atlas.
pub(crate) const BIND_GLYPH_ATLAS: BindGroupId = BindGroupId(1);

/// The text cell atlas.
pub(crate) const BIND_TEXT_ATLAS: BindGroupId = BindGroupId(2);

/// The slot atlas, at rest.
pub(crate) const BIND_SLOT_BASE: BindGroupId = BindGroupId(3);

/// The slot atlas, with the drag offset uniform applied.
pub(crate) const BIND_SLOT_DRAG: BindGroupId = BindGroupId(4);

/// First slot reserved for a textured lane's own texture.
const BIND_TEX_BASE: u16 = 5;

/// One slot per possible lane id, above the five fixed atlas slots.
///
/// [`lane_id`] is `2·lane_order` (+1 for the one tie), so the widest id the 48 lanes can
/// produce is `2·47`. Sizing the table from that keeps a textured lane's slot a pure function
/// of its lane — stable for the life of the lane, with no allocator and no free list.
pub(crate) const BIND_SLOTS: usize = BIND_TEX_BASE as usize + 95;

/// The bind-group slot holding `lane`'s own texture.
#[must_use]
pub(crate) fn tex_bind_id(lane: LaneId) -> BindGroupId {
    BindGroupId(BIND_TEX_BASE + lane.0)
}

/// Which pipeline a textured lane draws with.
///
/// This is `encoder.rs:72`'s `role == ForestFill` branch, moved to the side that knows what a
/// forest is.
#[must_use]
pub(crate) fn textured_pipeline_for(role: LaneRole) -> PipelineId {
    if role == LaneRole::ForestFill {
        PIPE_DENSITY
    } else {
        PIPE_TEXTURED
    }
}

/// Which atlas a sprite lane samples at group 2.
///
/// This is `encoder.rs:119-128` and the identical switch at `core/culling/engine.rs:139-146`,
/// which is why both now read the one table.
#[must_use]
pub(crate) fn sprite_atlas_for(role: LaneRole) -> BindGroupId {
    match role {
        LaneRole::SlotDrag => BIND_SLOT_DRAG,

        LaneRole::Slots
        | LaneRole::Clusters
        | LaneRole::SlotPlacePreview
        | LaneRole::MissionVehicles
        | LaneRole::MissionMarkers
        | LaneRole::MissionComments => BIND_SLOT_BASE,
        _ => BIND_GLYPH_ATLAS,
    }
}
