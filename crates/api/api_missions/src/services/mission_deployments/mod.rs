//! Mission deployments: an approved artifact selected for a server, validated before it is
//! persisted, performed by one fleet command, and confirmed only by a runtime session of that
//! server reporting the artifact it loaded. The slot bindings recorded with a deployment are what
//! the event roster and deployment authorization read, so game loading, roster derivation and
//! seat authorization all name the same artifact.

pub mod deployment_reads;
pub mod deployment_requests;
pub mod deployment_selection;
pub mod deployment_settlement;
pub mod slot_bindings;
