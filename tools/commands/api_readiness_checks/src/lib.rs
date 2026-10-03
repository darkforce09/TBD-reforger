//! The API readiness judge: API completion is current evidence for every registered requirement.
//!
//! **Role:** [`verify`] judges every receipt the acceptance register names (running the local
//! checks first when asked) and fails each requirement whose checks did not all hold;
//! [`operational_recording`] records the receipt of one staging procedure run;
//! [`PropertyTestConfiguration`] fixes the property-test seed and refuses a case-count override.
//! **Position:** a command crate of `tools/commands`, over `verification_core` (the report),
//! `process_runner` (the checks and `git`), `content_digest` (the digests), `deploy_settings` and
//! `repository_layout`. The `verify api-readiness`, `staging` and `db test-it` commands of `xtask`
//! call it.
//! **Signals & state:** none held; reads the register, the Git tree, the configuration files and
//! the process environment, and writes receipts and logs into the evidence folder.
//! **Invariants:** a receipt counts only against both current fingerprints; a run whose tree or
//! configuration changed while it ran is refused; `--execute` never runs an `operational` check or
//! a check without a command; only a passing staging recording carries the success marker.

mod case_count;
mod error;
mod evidence;
mod evidence_storage;
mod fingerprint;
mod operational;
mod operational_log;
pub mod operational_recording;
pub mod prelude;
mod property_evidence;
mod property_test_configuration;
mod readiness_verification;
mod register;
mod tool_identity;

pub use error::{Error, Result};
pub use property_test_configuration::PropertyTestConfiguration;
pub use readiness_verification::verify;
