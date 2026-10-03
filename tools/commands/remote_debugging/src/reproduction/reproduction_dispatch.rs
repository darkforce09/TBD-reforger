//! The routing of `cargo xtask repro`.
//!
//! **Role:** [`run`] sends each [`ReproCmd`] to its function.
//! **Position:** called by `xtask`'s `cli` dispatch; calls
//! [`crate::reproduction::mission_request_bodies`] and
//! [`crate::reproduction::mission_version_upload`].
//! **Signals & state:** none held.
//! **Invariants:** the two helpers exit 0 when they succeed; the orchestrator returns its own code.

use super::reproduction_command::ReproCmd;
use crate::error::Result;

/// Runs one `cargo xtask repro` command and returns its exit code.
pub fn run(cmd: ReproCmd) -> Result<u8> {
    match cmd {
        ReproCmd::MissionId => {
            crate::reproduction::mission_request_bodies::cmd_mission_id()?;
            Ok(0)
        }
        ReproCmd::MissionVersionBody { out, mb, semver } => {
            crate::reproduction::mission_request_bodies::cmd_mission_version_body(
                &out, mb, &semver,
            )?;
            Ok(0)
        }
        ReproCmd::MissionUpload => crate::reproduction::mission_version_upload::run(),
    }
}
