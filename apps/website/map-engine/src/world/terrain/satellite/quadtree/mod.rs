//! Role: Module boundary for terrain/satellite/quadtree.
//! Position: `world/terrain/satellite/quadtree` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::handles::EngineHandle;
use crate::streaming::bridge::progress::BootEvent;
use crate::streaming::bridge::progress::BootSeg;
use crate::streaming::bridge::progress::Ordered;
use crate::streaming::bridge::progress::SAT_CHUNK_BYTES;
use crate::streaming::bridge::progress::SAT_FETCH_CONCURRENCY;
use crate::streaming::bridge::progress::split_range;
use crate::streaming::bridge::statistics::BridgeHandle;
use crate::streaming::bridge::statistics::publish;
use crate::streaming::loaders::fetch::RangeBody;
use crate::streaming::loaders::fetch::RangeOutcome;
use crate::streaming::loaders::fetch::fetch_bytes;
use crate::streaming::loaders::fetch::fetch_range_outcome;
use crate::world::terrain::satellite::streamer::TbdSatIndex;
use crate::world::terrain::satellite::streamer::TbdSatMip;
use crate::world::terrain::satellite::streamer::TbdSatTile;
use crate::world::terrain::satellite::streamer::parse_tbd_sat_index_only;
use crate::world::terrain::satellite::streamer::parse_tbd_sat_index_strict;
use crate::world::terrain::satellite::streamer::pick_base_level_for_limit;
use crate::world::terrain::satellite::streamer::pick_preview_level;

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
