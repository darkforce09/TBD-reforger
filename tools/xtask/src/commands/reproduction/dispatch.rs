use super::cli::ReproCmd;
use anyhow::Result;

pub(crate) fn run(cmd: ReproCmd) -> Result<u8> {
    match cmd {
        ReproCmd::MissionId => {
            crate::commands::reproduction::fixtures::cmd_mission_id()?;
            Ok(0)
        }
        ReproCmd::MissionVersionBody { out, mb, semver } => {
            crate::commands::reproduction::fixtures::cmd_mission_version_body(&out, mb, &semver)?;
            Ok(0)
        }
        ReproCmd::MissionUpload => crate::commands::reproduction::mission_version_upload::run(),
    }
}
