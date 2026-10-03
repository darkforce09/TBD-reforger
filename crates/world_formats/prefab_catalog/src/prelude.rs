//! The names a reader of the prefab catalogue imports with `use prefab_catalog::prelude::*;`.

pub use crate::error::{Error, InvalidPrefabId, Result};
pub use crate::footprint_lookups::{
    BuildingPrefabInfo, FencePrefabInfo, building_prefab_lookup, fence_prefab_lookup, obb_corners,
};
pub use crate::instance_kinds::INSTANCE_KINDS;
pub use crate::numeric_prefab_ids::prefab_id_from_f64;
pub use crate::prefab_rows::{
    PrefabCatalog, PrefabEntry, PrefabRow, build_prefab_maps, catalog_from_bytes,
    narrow_prefab_rows,
};
pub use crate::prefab_tables::{PrefabTables, tables_from_bytes, tables_from_json};
pub use crate::render_classes::{NO_CLASS, class_code, render_class_for_prefab};
pub use crate::world_payload::{WorldError, bytes_to_json};
