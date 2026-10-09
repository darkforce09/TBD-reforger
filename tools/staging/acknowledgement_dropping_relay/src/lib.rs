//! The loopback relay that loses one fleet executor acknowledgement on purpose, for the lost
//! acknowledgement cases of the staging fleet receipt.
//!
//! - **Role:** runs the `acknowledgement-dropping-relay` executable. [`serve`] passes every
//!   exchange between one host agent and the API through unchanged; once armed over its control
//!   socket, it withholds the next `200` answer to a claim or to a result report past the agent's
//!   request timeout and then closes that connection without writing a byte of the answer.
//!   [`send_control_command`] arms, disarms and reads it.
//! - **Position:** tier 1 of `tools/staging`. `developer_tools`' `acknowledgement-dropping-relay`
//!   binary calls [`entrypoint`]. On the staging host the unit `acknowledgement-dropping-relay@N`
//!   runs `serve` between the relay instance's host agent and the loopback API origin, and the
//!   xtask staging fleet procedure runs `control` there over ssh to arm it before a restart and to
//!   read which command lost its answer.
//! - **Signals & state:** the arming, the counts and the last withheld answer live in one
//!   mutex-guarded policy for the life of the process; the only file is the control socket.
//! - **Invariants:**
//!   - The relay listens on a loopback address and forwards only to http on a loopback host.
//!   - One arming withholds exactly one `200` answer; a `204` claim answer and every other status
//!     pass through and keep it.
//!   - The control socket is mode 600.
//!   - The `Authorization` header is forwarded and never stored, logged or reported.

mod cli;
mod connection_abort;
mod control_socket;
mod drop_policy;
mod error;
mod http_client;
pub mod prelude;
mod relay;
mod relay_settings;

pub use cli::entrypoint;
pub use control_socket::{ControlCommand, MAXIMUM_COMMAND_BYTES, send_control_command};
pub use drop_policy::{
    Arming, DropRecord, DropTarget, ExecutorResponse, FleetCommandId, RelayStatus,
};
pub use error::{Error, Result, error_chain};
pub use relay::{RelayLog, RunningRelay, serve, start};
pub use relay_settings::{
    AGENT_REQUEST_TIMEOUT, DEFAULT_WITHHOLD, RelaySettings, UpstreamOrigin, loopback_listen_address,
};
