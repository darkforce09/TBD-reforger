//! Role: Module boundary for symbology/instances/slots/tests.
//! Position: `overlay/symbology/instances/slots/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

use render_primitives::text::pack::{pack_rgba_u32, screen_yaw_for_heading_deg, yaw_to_snorm16};
use std::collections::HashSet;

mod cases_1;
mod cases_2;
