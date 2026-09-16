//! Role: layout.
//! Position: `apps/website/graphics-engine/src` — the shared binary contract, in one list.
//! Signals & state: none. Re-exports only; not one line of logic lives here.
//! Invariants: everything named below is POD or bit-packing — a struct with a fixed byte
//! layout, a constant that sizes one, or a function that writes one. Nothing here owns a GPU
//! resource, and nothing here decides anything.
//!
//! T-0xx Phase 2B. `website-map-engine`'s upload belts sit next to their data —
//! `environment/{buildings,vegetation}/buffers.rs`, `overlay/symbology/instances/*`,
//! `world/scene.rs`, `world/terrain/satellite/textures.rs` — and they pack bytes in the exact
//! shapes this crate's vertex layouts declare. Routing that through `map-engine`'s `frame/`
//! would put vegetation and town-label knowledge inside `frame/` and make it a god-module, so
//! belts import this surface directly and gate rule 3b leaves it unrestricted.
//!
//! The point of collecting it here is that the cross-crate ABI then reads as a list in one
//! file. Anything that needs to be added is a deliberate widening of the contract, visible in
//! one diff.

/// Per-instance vertex layouts.
pub use crate::draw::instances::{
    ATLAS_GLYPH_COUNT, BuildingInstance, CHUNK_CAPACITY, IconInstance, QuadInstance, UNIT_QUAD,
};

/// One line vertex, and the corner/offset arithmetic that places a rect.
pub use crate::draw::geometry::{LineVertex, corner_uv, pack_offset};

/// CPU-side mesh and hairline composition results, and the colour arithmetic feeding them.
pub use crate::draw::compose::{
    HairlineGpu, PolyMeshGpu, mesh_from_tri, retint_fill_alpha, u8_rgba_to_f32,
};

/// Cell size in world meters, and the character → cell map.
pub use crate::text::metrics::*;

/// Laying a placed string out into glyph instances.
pub use crate::text::layout::*;

/// The glyph-size anchor.
pub use crate::text::scale::REF_ZOOM;

/// Bit-packing for sprite instances, and the `TextUniforms` block.
pub use crate::text::pack;
