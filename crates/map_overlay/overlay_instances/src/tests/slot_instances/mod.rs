//! Cases of the slot, vehicle, comment and cluster instance packers, the drag previews and the
//! row patches, measured against the unit symbology's tints, classes and atlas cells.

use crate::drag::*;
use crate::patches::*;
use crate::symbols::*;
use unit_symbology::classification::*;
use unit_symbology::symbol_atlas::*;

use render_primitives::text::pack::{pack_rgba_u32, yaw_to_snorm16};
use std::collections::HashSet;

mod cases_1;
mod cases_2;
