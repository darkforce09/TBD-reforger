//! The names a paper doll renderer or host imports with `use paper_doll_scene::prelude::*;`.

pub use crate::part_meshes::{mesh_cube, mesh_cylinder};
pub use crate::region_picking::{anchor_px, anchor_world, pick};
pub use crate::soldier_parts::{
    CLEAR_COLOR, DECOR, DollInstance, MeshKind, REGION_COUNT, REGION_KEYS, STATE_ACTIVE,
    STATE_EMPTY, STATE_EQUIPPED, decor_color, instances, state_color,
};
