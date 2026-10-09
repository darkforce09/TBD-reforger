//! The member load of the `staging_load` receipt and the `staging-load` executable that runs it.
//!
//! - **Role:** [`run`] turns a `staging_load_plan::LoadRunPlan` into a
//!   `staging_load_plan::LoadReport`: open-loop virtual clients spread over several source
//!   addresses, each rotating through its own accounts, measured over a fixed window.
//!   [`entrypoint`] is the command line of the `staging-load` executable: a plan as JSON in, the
//!   report as JSON out.
//! - **Position:** tier 2 of `tools/staging`, above `staging_load_plan`, whose types it runs and
//!   whose report it assembles. `developer_tools`' `staging-load` binary calls [`entrypoint`]; the
//!   xtask load procedure runs that binary as a child process, so tokio and the HTTP client stay
//!   out of xtask; the API behind the plan's target origin is the only peer.
//! - **Signals & state:** a run builds its own multi-threaded tokio runtime and owns the clients,
//!   the per-address guards and the account rings for one call; rotated tokens stay in memory and
//!   are dropped with them.
//! - **Invariants:** a token never reaches a log, an error, the report or standard output; every
//!   request leaves from a source address of the plan under that address's ceilings.

mod account_rotation;
mod account_switch_lane;
mod command_line;
mod error;
mod guarded_exchange;
mod http_client;
mod load_run;
mod member_request_lane;
pub mod prelude;
mod source_address_pool;
mod virtual_client;

pub use command_line::entrypoint;
pub use error::{Error, Result};
pub use load_run::run;
