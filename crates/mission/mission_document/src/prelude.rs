//! The names a caller of the mission document imports with `use mission_document::prelude::*;`:
//! the document, its rows and patches, its ids and its error.

pub use crate::ids::{
    ClientId, CommentId, CompositionId, ConnectionId, CrewSeatId, EntityId, FactionId, LayerId,
    SquadId, VehicleId,
};
pub use crate::{
    ConnectionFinding, ConnectionKind, ConnectionRow, EntityTransformPatch, Error, MergeOpts,
    MergeReport, MissionDocCore, Result, SquadMembership, formation_offsets,
    validate_connection_rows,
};
