//! The overlay instance packers of the map: the bytes of every slot, vehicle, comment,
//! cluster and fire-mission mark.
//!
//! **Role:** packs the icon instances of slots, vehicles, comments and clusters
//! ([`symbols`]), the drag previews ([`drag`]) and the in-place row patches of the slot lane
//! ([`patches`]), and builds the fire-mission marks ([`fire_mission_marks`]).
//! **Position:** map overlay tier 3, over `unit_symbology` (tints, classes, atlas cells),
//! `map_draw_lanes` (the lanes the marks land on) and `render_primitives` (the icon and line
//! layouts). The map engine's slot, vehicle and comment bridges and the mortar page's map
//! picker upload what it packs.
//! **Signals & state:** none; pure functions into owned buffers.
//! **Invariants:** every icon instance is 20 bytes in the `render_primitives::text::pack`
//! layout; nothing here owns a GPU resource.

pub mod drag;
pub mod fire_mission_marks;
pub mod patches;
pub mod prelude;
pub mod symbols;
