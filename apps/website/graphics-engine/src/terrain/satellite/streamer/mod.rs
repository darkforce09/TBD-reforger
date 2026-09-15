//! Role: Module boundary for terrain/satellite/streamer.
//! Position: `terrain/satellite/streamer` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::formats::archives::codec::access_checked;
use crate::formats::archives::satellite::ArchivedTbdSatIndexV2;
use crate::formats::archives::satellite::TbdSatIndexV2;

use crate::formats::containers::tbds::TbdsHeader;
use serde::Deserialize;
mod model;
use model::{FORMAT_WEBP, MAGIC, V1, V2};

/// Re-export `model::{TbdSatError,TbdSatIndex,TbdSatMip,TbdSatTile}`.
pub use model::{TbdSatError, TbdSatIndex, TbdSatMip, TbdSatTile};
mod header;

/// Re-export `header::{index_range_end,parse_header}`.
pub use header::{index_range_end, parse_header};

mod archive;
use archive::AlignedIndex;
use archive::mips_from_archive;
mod validation;

/// Re-export `validation::{parse_tbd_sat_index_only,parse_tbd_sat_index_strict}`.
pub use validation::{parse_tbd_sat_index_only, parse_tbd_sat_index_strict};

mod selection;

/// Re-export `selection::{pick_base_level,pick_base_level_for_limit,pick_preview_level}`.
pub use selection::{pick_base_level, pick_base_level_for_limit, pick_preview_level};
#[cfg(test)]
mod t935_10;
