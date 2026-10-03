//! Cases of the slot, vehicle, comment and cluster instance packers, the drag previews and the
//! row patches, measured against the unit symbology's tints, classes and atlas cells.

use crate::drag::*;
use crate::patches::*;
use crate::symbols::*;
use map_draw_lanes::zoom_gates::px_to_m_at_zoom;
use unit_symbology::classification::*;
use unit_symbology::symbol_atlas::*;

use render_primitives::text::pack::{pack_rgba_u32, screen_yaw_for_heading_deg, yaw_to_snorm16};
use std::collections::HashSet;

mod cases_1;
mod cases_2;
