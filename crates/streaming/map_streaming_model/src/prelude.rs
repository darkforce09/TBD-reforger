//! The names a map host, loader or renderer imports with
//! `use map_streaming_model::prelude::*;`.

pub use crate::asset_sink::{
    ForestDensityRaster, GlyphAtlasImage, MapAssetSink, MapAssetSinkSlot, MapViewport,
    SharedMapAssetSink, TextureLayerSpec, TextureRegion, WorldGlyphLane,
};
pub use crate::boot_progress::{BootEvent, BootSeg, ProgressFn};
pub use crate::error::{Error, Result};
pub use crate::host_preferences::{BootstrapScope, HostPreferences, RenderPreferences};
pub use crate::memory_budget::{Asset, Decision, Ledger, LevelBytes};
pub use crate::world_layer_preferences::WorldLayerPrefs;
