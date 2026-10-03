//! The names a caller of the operations imports with `use mission_operations::prelude::*;`: the
//! side-level commands and their error, the plain rows and their projections, and the authoring
//! session state a host keeps between gestures.

pub use crate::apply_faction::{
    ApplyFactionResult, FactionLibraryInput, FactionLibraryRole, FactionLibraryVehicle,
    apply_faction_library,
};
pub use crate::cargo::{
    BufferedLoadout, cargo_defaults_for, loadout_buffer, loadout_buffer_len, next_apply_seed,
    set_cargo_defaults,
};
pub use crate::cargo_rules::CargoRow;
pub use crate::entity::{
    ArmedPlacement, ArmedPlacementKind, LayerDrag, PlacementCommit, ZoneDrawStep, begin_zone_draft,
    cancel_refile, close_zone_polygon_draft, ensure_layer, placement_is_armable, take_rename_armed,
    vehicle_places_its_crew,
};
pub use crate::place_orbat::place_character_under_side;
pub use crate::projections::{faction_rows, layer_rows, slot_rows, squad_rows};
pub use crate::rows::{CommentRow, FactionRow, LayerRow, SlotRow, SquadRow};
pub use crate::tactical_graphics::{
    begin_tactical_draw, cancel_tactical_draw, clear_tactical_selection, tactical_draw_armed,
};
pub use crate::zones::{DrawTarget, ZoneShape};
pub use crate::{Error, Result};
