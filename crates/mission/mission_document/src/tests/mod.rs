//! The tests that build a whole mission document: id array merges between peers, undo grouping
//! on an injected clock, and payload and vehicle rows round-tripped through the payload compiler.

use crate::MissionDocCore;

mod id_array_merges;
mod payload_round_trips;
mod undo_groups;
