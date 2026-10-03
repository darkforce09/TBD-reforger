//! The executor side of the platform's fleet command ledger: claim the next command for this
//! server, report that its effect is starting, report its outcome. The API side is described in
//! `documentation/apps/api/verification_evidence/fleet_command_ledger.md`; the wire
//! contract is `contracts/definitions/fleet-command.schema.json`.
//!
//! - `ledger_messages`: the result report built from a verdict and the API's error envelope; the
//!   wire shapes are `fleet_wire_contract::executor_messages`.
//! - `ledger_api`: the HTTP calls and the classification of their failures.
//! - `retry_backoff`: jittered exponential backoff.
//! - `command_loop`: the claim loop and its reporting rules.

mod command_loop;
mod ledger_api;
mod ledger_messages;
mod retry_backoff;

pub use command_loop::{CommandLoop, LedgerTimings};
pub use ledger_api::{ClaimOutcome, LedgerApi, LedgerApiSetupError, LedgerError};
pub use retry_backoff::{BackoffPolicy, JitteredBackoff};
