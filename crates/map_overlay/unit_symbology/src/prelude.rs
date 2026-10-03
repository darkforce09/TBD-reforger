//! The names a caller of the unit symbology imports with `use unit_symbology::prelude::*;`.

pub use crate::classification::{
    UnitRoleClass, VehicleKind, side_rgba, unit_role_class, vehicle_kind_for_alias,
};
pub use crate::markers::{MarkerGlyph, marker_glyph_for_alias};
pub use crate::slot_ids::SlotId;
pub use crate::squad_links::{SquadLinkInput, build_squad_link_segments};
pub use crate::symbol_atlas::{WidenedSlotAtlas, build_slot_atlas, extend_atlas_with_unit_glyphs};
