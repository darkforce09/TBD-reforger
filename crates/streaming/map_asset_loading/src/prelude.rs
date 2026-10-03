//! The names a map host or a frontend imports with `use map_asset_loading::prelude::*;`.

#[cfg(target_arch = "wasm32")]
pub use crate::asset_statistics::{BridgeHandle, MapAssetsBridge, new_bridge};
#[cfg(target_arch = "wasm32")]
pub use crate::browser_asset_sink::{BrowserAssetSink, BrowserAssetSinkHandle};
#[cfg(target_arch = "wasm32")]
pub use crate::environment::forest_mass_loader::ForestMassHost;
#[cfg(target_arch = "wasm32")]
pub use crate::environment::location_labels::loader::LabelHost;
pub use crate::live_memory_budget::hud_suffix;
pub use crate::mesh_composition::{LandcoverInput, compose_landcover_mesh};
#[cfg(target_arch = "wasm32")]
pub use crate::occluder_loader::OccluderHost;
#[cfg(target_arch = "wasm32")]
pub use crate::terrain::relief::dem_vectors::DemVectors;
#[cfg(target_arch = "wasm32")]
pub use crate::terrain::water::loader::WaterHost;
#[cfg(target_arch = "wasm32")]
pub use crate::world_loader::WorldHost;
