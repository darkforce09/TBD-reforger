//! **Role:** module boundary of the satellite loads: the preview, the full mip chain and the
//! cartographic map tiles, and the imports the parts share.
//! **Position:** `terrain::satellite_quadtree` in `map_asset_loading`; the map host calls
//! `load_satellite` at boot and swaps the basemap view.
//! **Signals & state:** none here; each load writes the basemap texture layer through the asset
//! sink.
//! **Invariants:** the container is read only through `satellite_imagery`.

use crate::asset_statistics::BridgeHandle;
use crate::asset_statistics::publish;
use crate::browser_asset_sink::BrowserAssetSink;
use crate::browser_asset_sink::BrowserAssetSinkHandle;
use browser_platform::fetch::RangeBody;
use browser_platform::fetch::RangeOutcome;
use browser_platform::fetch::fetch_bytes;
use browser_platform::fetch::fetch_range_outcome;
use map_streaming_model::asset_sink::TextureLayerSpec;
use map_streaming_model::asset_sink::TextureRegion;
use map_streaming_model::boot_progress::BootEvent;
use map_streaming_model::boot_progress::BootSeg;
use map_streaming_model::boot_progress::Ordered;
use map_streaming_model::boot_progress::SAT_CHUNK_BYTES;
use map_streaming_model::boot_progress::SAT_FETCH_CONCURRENCY;
use map_streaming_model::boot_progress::split_range;
use satellite_imagery::TbdSatIndex;
use satellite_imagery::TbdSatMip;
use satellite_imagery::TbdSatTile;
use satellite_imagery::parse_tbd_sat_index_only;
use satellite_imagery::parse_tbd_sat_index_strict;
use satellite_imagery::pick_base_level_for_limit;
use satellite_imagery::pick_preview_level;

use wasm_bindgen_futures::JsFuture;
mod selection;
use selection::MODE_SINGLE;
use selection::MODE_UNIFIED;
use selection::ROLE_BASEMAP;
use selection::fetch_index_head;
use selection::report_chosen_level;
use selection::texture_limit;
mod preview;

/// Re-export `preview::sat_preview_only`.
pub use preview::sat_preview_only;
use preview::{PREVIEW_MAX_EDGE, try_preview};
mod decode;
use decode::{Decoded, decode_webp, upload_decoded};
mod retry;
use retry::RANGE_ATTEMPTS;
use retry::fetch_range_resilient;
mod downloads;
use downloads::{fetch_mip_blocks, fetch_tiles};
mod upload;
use upload::commit_mip;
mod bootstrap;
use bootstrap::load_unified_full;
mod basemap;

/// Re-export `basemap::{load_map_basemap,load_satellite,show_satellite_basemap}`.
pub use basemap::{load_map_basemap, load_satellite, show_satellite_basemap};
