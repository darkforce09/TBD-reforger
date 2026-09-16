//! Role: Module boundary for symbology/instances/slots.
//! Position: `overlay/symbology/instances/slots` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::overlay::symbology::instances::symbols::CLUSTER_DISC_RGBA`.
pub use crate::overlay::symbology::instances::symbols::CLUSTER_DISC_RGBA;

/// Re-export `crate::overlay::symbology::instances::symbols::CLUSTER_SLOT_THRESHOLD`.
pub use crate::overlay::symbology::instances::symbols::CLUSTER_SLOT_THRESHOLD;

/// Re-export `crate::overlay::symbology::instances::symbols::COMMENT_NOTE_PX`.
pub use crate::overlay::symbology::instances::symbols::COMMENT_NOTE_PX;

/// Re-export `crate::overlay::symbology::instances::symbols::COMMENT_NOTE_RGBA`.
pub use crate::overlay::symbology::instances::symbols::COMMENT_NOTE_RGBA;

/// Re-export `crate::overlay::symbology::instances::symbols::SLOT_GLYPH_DISC`.
pub use crate::overlay::symbology::instances::symbols::SLOT_GLYPH_DISC;

/// Re-export `crate::overlay::symbology::instances::symbols::SLOT_GLYPH_RING`.
pub use crate::overlay::symbology::instances::symbols::SLOT_GLYPH_RING;

/// Re-export `crate::overlay::symbology::instances::symbols::SLOT_ICON_STRIDE`.
pub use crate::overlay::symbology::instances::symbols::SLOT_ICON_STRIDE;

/// Re-export `crate::overlay::symbology::instances::symbols::SLOT_PRIMARY_RGBA`.
pub use crate::overlay::symbology::instances::symbols::SLOT_PRIMARY_RGBA;

/// Re-export `crate::overlay::symbology::instances::symbols::SLOT_RING_PX`.
pub use crate::overlay::symbology::instances::symbols::SLOT_RING_PX;

/// Re-export `crate::overlay::symbology::instances::symbols::SLOT_SELECTED_PX`.
pub use crate::overlay::symbology::instances::symbols::SLOT_SELECTED_PX;

/// Re-export `crate::overlay::symbology::instances::symbols::SLOT_SELECTED_RGBA`.
pub use crate::overlay::symbology::instances::symbols::SLOT_SELECTED_RGBA;

/// Re-export `crate::overlay::symbology::instances::symbols::SLOT_UNIT_PX`.
pub use crate::overlay::symbology::instances::symbols::SLOT_UNIT_PX;

/// Re-export `crate::overlay::symbology::instances::symbols::SYMBOLOGY_MAX_M_PER_PX`.
pub use crate::overlay::symbology::instances::symbols::SYMBOLOGY_MAX_M_PER_PX;

/// Re-export `crate::overlay::symbology::instances::symbols::VEHICLE_SYMBOL_PX`.
pub use crate::overlay::symbology::instances::symbols::VEHICLE_SYMBOL_PX;

/// Re-export `crate::overlay::symbology::instances::symbols::ZOOM_CLUSTER_MAX`.
pub use crate::overlay::symbology::instances::symbols::ZOOM_CLUSTER_MAX;

/// Re-export `crate::overlay::symbology::instances::symbols::cluster_disc_size_px`.
pub use crate::overlay::symbology::instances::symbols::cluster_disc_size_px;

/// Re-export `crate::overlay::symbology::instances::symbols::cluster_mode`.
pub use crate::overlay::symbology::instances::symbols::cluster_mode;

/// Re-export `crate::overlay::symbology::instances::symbols::pack_cluster_instances`.
pub use crate::overlay::symbology::instances::symbols::pack_cluster_instances;

/// Re-export `crate::overlay::symbology::instances::symbols::pack_comment_instances`.
pub use crate::overlay::symbology::instances::symbols::pack_comment_instances;

/// Re-export `crate::overlay::symbology::instances::symbols::pack_one_slot`.
pub use crate::overlay::symbology::instances::symbols::pack_one_slot;

/// Re-export `crate::overlay::symbology::instances::symbols::pack_rings`.
pub use crate::overlay::symbology::instances::symbols::pack_rings;

/// Re-export `crate::overlay::symbology::instances::symbols::pack_slot_instances`.
pub use crate::overlay::symbology::instances::symbols::pack_slot_instances;

/// Re-export `crate::overlay::symbology::instances::symbols::pack_slot_symbology`.
pub use crate::overlay::symbology::instances::symbols::pack_slot_symbology;

/// Re-export `crate::overlay::symbology::instances::symbols::pack_vehicle_instances`.
pub use crate::overlay::symbology::instances::symbols::pack_vehicle_instances;

/// Re-export `crate::overlay::symbology::instances::symbols::pack_vehicle_symbology`.
pub use crate::overlay::symbology::instances::symbols::pack_vehicle_symbology;

/// Re-export `crate::overlay::symbology::instances::symbols::px_to_m_at_zoom`.
pub use crate::overlay::symbology::instances::symbols::px_to_m_at_zoom;

/// Re-export `crate::overlay::symbology::instances::symbols::symbology_visible`.
pub use crate::overlay::symbology::instances::symbols::symbology_visible;

/// Re-export `crate::overlay::symbology::roles::classify::SIDE_BLUFOR_RGBA`.
pub use crate::overlay::symbology::roles::classify::SIDE_BLUFOR_RGBA;

/// Re-export `crate::overlay::symbology::roles::classify::SIDE_INDFOR_RGBA`.
pub use crate::overlay::symbology::roles::classify::SIDE_INDFOR_RGBA;

/// Re-export `crate::overlay::symbology::roles::classify::SIDE_OPFOR_RGBA`.
pub use crate::overlay::symbology::roles::classify::SIDE_OPFOR_RGBA;

/// Re-export `crate::overlay::symbology::roles::classify::UNIT_ROLE_CLASS_COUNT`.
pub use crate::overlay::symbology::roles::classify::UNIT_ROLE_CLASS_COUNT;

/// Re-export `crate::overlay::symbology::roles::classify::UnitRoleClass`.
pub use crate::overlay::symbology::roles::classify::UnitRoleClass;

/// Re-export `crate::overlay::symbology::roles::classify::VEHICLE_KIND_COUNT`.
pub use crate::overlay::symbology::roles::classify::VEHICLE_KIND_COUNT;

/// Re-export `crate::overlay::symbology::roles::classify::VehicleKind`.
pub use crate::overlay::symbology::roles::classify::VehicleKind;

/// Re-export `crate::overlay::symbology::roles::classify::side_rgba`.
pub use crate::overlay::symbology::roles::classify::side_rgba;

/// Re-export `crate::overlay::symbology::roles::classify::side_tints_rgba_bytes`.
pub use crate::overlay::symbology::roles::classify::side_tints_rgba_bytes;

/// Re-export `crate::overlay::symbology::roles::classify::unit_role_class`.
pub use crate::overlay::symbology::roles::classify::unit_role_class;

/// Re-export `crate::overlay::symbology::roles::classify::vehicle_kind_for_alias`.
pub use crate::overlay::symbology::roles::classify::vehicle_kind_for_alias;

/// Re-export `crate::overlay::symbology::instances::packing::pack_icon_instance`.
pub use crate::overlay::symbology::instances::packing::pack_icon_instance;

/// Re-export `crate::overlay::symbology::instances::packing::pack_icon_instance_yaw`.
pub use crate::overlay::symbology::instances::packing::pack_icon_instance_yaw;

/// Re-export `crate::overlay::symbology::instances::packing::pack_rgba_u32`.
pub use crate::overlay::symbology::instances::packing::pack_rgba_u32;

/// Re-export `crate::overlay::symbology::instances::packing::screen_yaw_for_heading_deg`.
pub use crate::overlay::symbology::instances::packing::screen_yaw_for_heading_deg;

/// Re-export `crate::overlay::symbology::instances::packing::yaw_to_snorm16`.
pub use crate::overlay::symbology::instances::packing::yaw_to_snorm16;

/// Re-export `crate::overlay::symbology::instances::drag::DragGpuPhase`.
pub use crate::overlay::symbology::instances::drag::DragGpuPhase;

/// Re-export `crate::overlay::symbology::instances::drag::classify_drag_transition`.
pub use crate::overlay::symbology::instances::drag::classify_drag_transition;

/// Re-export `crate::overlay::symbology::instances::drag::drag_projected`.
pub use crate::overlay::symbology::instances::drag::drag_projected;

/// Re-export `crate::overlay::symbology::instances::drag::pack_drag_overlay`.
pub use crate::overlay::symbology::instances::drag::pack_drag_overlay;

/// Re-export `crate::overlay::symbology::instances::drag::pack_drag_overlay_symbology`.
pub use crate::overlay::symbology::instances::drag::pack_drag_overlay_symbology;

/// Re-export `crate::overlay::symbology::instances::drag::pack_vehicle_drag_preview`.
pub use crate::overlay::symbology::instances::drag::pack_vehicle_drag_preview;

/// Re-export `crate::overlay::symbology::instances::patches::hide_slot_row_patch`.
pub use crate::overlay::symbology::instances::patches::hide_slot_row_patch;

/// Re-export `crate::overlay::symbology::instances::patches::pack_selection_only`.
pub use crate::overlay::symbology::instances::patches::pack_selection_only;

/// Re-export `crate::overlay::symbology::instances::patches::selected_mask`.
pub use crate::overlay::symbology::instances::patches::selected_mask;

/// Re-export `crate::overlay::symbology::instances::patches::selected_row_patch`.
pub use crate::overlay::symbology::instances::patches::selected_row_patch;

/// Re-export `crate::overlay::symbology::instances::patches::symbology_row_patch`.
pub use crate::overlay::symbology::instances::patches::symbology_row_patch;

/// Re-export `crate::overlay::symbology::instances::patches::unselected_row_patch`.
pub use crate::overlay::symbology::instances::patches::unselected_row_patch;

/// Re-export `crate::overlay::symbology::instances::patches::unselected_row_patch_for`.
pub use crate::overlay::symbology::instances::patches::unselected_row_patch_for;

/// Re-export `crate::overlay::symbology::atlas::raster::ATLAS_CELL_PX`.
pub use crate::overlay::symbology::atlas::raster::ATLAS_CELL_PX;

/// Re-export `crate::overlay::symbology::atlas::raster::COMMENT_CELL`.
pub use crate::overlay::symbology::atlas::raster::COMMENT_CELL;

/// Re-export `crate::overlay::symbology::atlas::raster::COMMENT_SELECTED_CELL`.
pub use crate::overlay::symbology::atlas::raster::COMMENT_SELECTED_CELL;

/// Re-export `crate::overlay::symbology::atlas::raster::SLOT_ATLAS_H`.
pub use crate::overlay::symbology::atlas::raster::SLOT_ATLAS_H;

/// Re-export `crate::overlay::symbology::atlas::raster::SLOT_ATLAS_UV`.
pub use crate::overlay::symbology::atlas::raster::SLOT_ATLAS_UV;

/// Re-export `crate::overlay::symbology::atlas::raster::SLOT_ATLAS_W`.
pub use crate::overlay::symbology::atlas::raster::SLOT_ATLAS_W;

/// Re-export `crate::overlay::symbology::atlas::raster::SYMBOLOGY_CELL_COUNT`.
pub use crate::overlay::symbology::atlas::raster::SYMBOLOGY_CELL_COUNT;

/// Re-export `crate::overlay::symbology::atlas::raster::SlotAtlas`.
pub use crate::overlay::symbology::atlas::raster::SlotAtlas;

/// Re-export `crate::overlay::symbology::atlas::raster::UNIT_CELL_BASE`.
pub use crate::overlay::symbology::atlas::raster::UNIT_CELL_BASE;

/// Re-export `crate::overlay::symbology::atlas::raster::UNIT_SELECTED_CELL_BASE`.
pub use crate::overlay::symbology::atlas::raster::UNIT_SELECTED_CELL_BASE;

/// Re-export `crate::overlay::symbology::atlas::raster::VEHICLE_CELL_BASE`.
pub use crate::overlay::symbology::atlas::raster::VEHICLE_CELL_BASE;

/// Re-export `crate::overlay::symbology::atlas::raster::WidenedSlotAtlas`.
pub use crate::overlay::symbology::atlas::raster::WidenedSlotAtlas;

/// Re-export `crate::overlay::symbology::atlas::raster::build_slot_atlas`.
pub use crate::overlay::symbology::atlas::raster::build_slot_atlas;

/// Re-export `crate::overlay::symbology::atlas::raster::extend_atlas_with_unit_glyphs`.
pub use crate::overlay::symbology::atlas::raster::extend_atlas_with_unit_glyphs;

#[cfg(test)]
mod tests;
