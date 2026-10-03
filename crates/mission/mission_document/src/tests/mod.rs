//! The tests that build a whole mission document: id array merges between peers, undo grouping
//! on an injected clock, payload and vehicle rows round-tripped through the payload compiler, and
//! the prelude surface.

use crate::MissionDocCore;

mod id_array_merges;
mod payload_round_trips;
mod prelude_surface;
mod undo_groups;
mod vehicle_row_round_trips;
