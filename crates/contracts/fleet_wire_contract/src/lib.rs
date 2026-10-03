//! The fleet-command wire contract shared by the platform API and the fleet host agent.
//!
//! **Role:** owns every shape of `contracts/definitions/fleet-command.schema.json` that crosses
//! the API's fleet routes — the action and its rules, the operator request, receipt and list,
//! the executor claim, start and result reports, and the console command's arguments and
//! outcome — with both `Serialize` and `Deserialize`, plus the executor kind, the RFC 3339
//! timestamp spelling those shapes use, the machine credential format and the secret-file
//! limits both sides of a credential file apply.
//! **Position:** contracts tier, depending on no workspace crate. Consumed by `api` (the ledger
//! services, the fleet handlers and the staging fixtures tool, which keep their own database
//! rows and file I/O), by `fleet_host_agent` (the ledger client and the secret-file reader) and
//! by `xtask` (the credential prefix).
//! **Signals & state:** none; plain data, constants and pure functions.
//! **Invariants:** one type per shape serves both directions, so the bytes the API writes are the
//! bytes the agent reads and the reverse; every instant is written in UTC with a `Z` suffix and
//! fractional seconds trimmed of trailing zeros; a well-formed machine credential reads
//! `tbdm_<32 lowercase hex digits>_<64 lowercase hex digits>`.

pub mod console_command;
pub mod error;
pub mod executor_kind;
pub mod executor_messages;
pub mod fleet_action;
pub mod machine_credential_format;
pub mod operator_messages;
pub mod prelude;
pub mod rfc3339_timestamps;
pub mod secret_file_limits;

pub use error::{Error, Result};
pub use executor_kind::ExecutorKind;
pub use fleet_action::FleetAction;
