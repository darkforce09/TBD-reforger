use super::cli::ModCmd;
use crate::core::repository_root::find_repo_root;
use anyhow::Result;

pub(crate) fn run(cmd: ModCmd) -> Result<u8> {
    match cmd {
        ModCmd::RemoteLogs { file, selftest } => {
            crate::commands::debug::remote_logs::run(file, selftest)
        }
        ModCmd::SpawnDeterminism {
            preflight,
            selftest,
            runs,
            world,
        } => crate::verifications::mod_scripts::spawn_determinism::run(
            &find_repo_root()?,
            preflight,
            selftest,
            runs.unwrap_or(5),
            world.as_deref().unwrap_or("worlds/TBD_Dev_POC.ent"),
        ),
        ModCmd::SpawnVerify { selftest, pattern } => {
            crate::verifications::mod_scripts::spawn_verification::run(selftest, pattern)
        }
        ModCmd::ManualTest => crate::commands::mod_ops::manual_test::run(&find_repo_root()?),
        ModCmd::DevBootstrap { args } => {
            crate::commands::mod_ops::development_bootstrap::run(&args)
        }
        ModCmd::DevServer { args } => crate::commands::mod_ops::development_server::run(&args),
        ModCmd::TestMission { target } => {
            crate::commands::mod_ops::mission_test::run(target.as_deref())
        }
        ModCmd::BootstrapStaging => crate::commands::setup::staging_server::run(),
        ModCmd::SeedAnnouncement => crate::commands::db::milestone_announcement::run(),
        ModCmd::TestPhase1Api => {
            crate::commands::mod_ops::backend_api_test::run(&find_repo_root()?)
        }
        ModCmd::Playtest { args } => crate::commands::mod_ops::playtest_server::run(&args),
        ModCmd::Compile { args } => crate::commands::mod_ops::compile::run(&args),
        ModCmd::CompileSelftest => crate::commands::mod_ops::compile::run_selftest(),
        ModCmd::CompilePreflight => crate::commands::mod_ops::compile::run_preflight(),
        ModCmd::WorldBoot { args } => crate::commands::mod_ops::world_boot::run(&args),
        ModCmd::Wave { args } => crate::commands::mod_ops::wave_execution::run(&args),
    }
}
