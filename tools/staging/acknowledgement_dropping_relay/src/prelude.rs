//! The names a caller imports with `use acknowledgement_dropping_relay::prelude::*;`.

pub use crate::cli::entrypoint;
pub use crate::control_socket::{ControlCommand, send_control_command};
pub use crate::drop_policy::{DropTarget, RelayStatus};
pub use crate::error::Error;
pub use crate::relay::{RelayLog, serve, start};
pub use crate::relay_settings::RelaySettings;
