//! Legacy admin module — home of the server RCON console until it moves to
//! `server_infrastructure`. The same-named `admin.rs` is glob re-exported so
//! `handlers::admin::send_rcon` resolves here.

// Deliberate inception: the module keeps its same-named root handler file, glob re-exported
// below so the flat `handlers::admin::…` path resolves.
#[allow(clippy::module_inception)]
mod admin;
pub use self::admin::*;
