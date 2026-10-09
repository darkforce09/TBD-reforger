//! Routes each `cargo xtask mod` subcommand to its implementation.
//!
//! **Role:** [`run`] maps one [`ModCmd`] to the entry function that answers it and returns that
//! entry's exit code.
//! **Position:** called by the xtask binary's `mod` group; it reaches this crate's modules and the
//! `remote_debugging`, `mod_script_checks`, `workstation_setup` and `database_operations` entries.
//! **Signals & state:** none.
//! **Invariants:** an exit code is the entry's verdict; an [`crate::Error`] means the command
//! could not do its work at all.

use crate::Result;
use crate::mod_command::ModCmd;
use repository_root::find_repository_root;

/// Run one `cargo xtask mod` subcommand and return its exit code.
pub fn run(cmd: ModCmd) -> Result<u8> {
    match cmd {
        ModCmd::ProjectEquipmentGameplay { input, output } => {
            crate::equipment_gameplay::project_command(&input, &output)
        }
        ModCmd::GenerateEquipmentGameplayPolicy { check } => {
            crate::equipment_gameplay::generate_command(check)
        }
        ModCmd::ValidateEquipmentVehicleExport { input } => {
            crate::equipment_vehicle_export::validate_command(&input)
        }
        ModCmd::PublishEquipmentVehicleExport { input } => {
            crate::equipment_vehicle_export::publish_command(&input)
        }
        ModCmd::RemoteLogs {
            file,
            selftest,
            instance,
        } => Ok(remote_debugging::debug::remote_logs::run(
            file, selftest, instance,
        )?),
        ModCmd::SpawnDeterminism {
            preflight,
            selftest,
            runs,
            world,
        } => Ok(mod_script_checks::spawn_determinism::run(
            &find_repository_root()?,
            preflight,
            selftest,
            runs.unwrap_or(5),
            world.as_deref().unwrap_or("worlds/TBD_Dev_POC.ent"),
        )?),
        ModCmd::SpawnVerify { selftest, pattern } => Ok(
            mod_script_checks::spawn_verification::run(selftest, pattern)?,
        ),
        ModCmd::DevBootstrap { args } => crate::development_bootstrap::run(&args),
        ModCmd::DevServer { args } => crate::development_server::run(&args),
        ModCmd::TestMission { target } => crate::mission_test::run(target.as_deref()),
        ModCmd::BootstrapStaging => Ok(workstation_setup::staging_server::run()?),
        ModCmd::SeedAnnouncement => Ok(database_operations::milestone_announcement::run()?),
        ModCmd::TestGameRuntimeApi => crate::game_runtime_api_smoke::run(),
        ModCmd::Playtest { args } => crate::playtest_server::run(&args),
        ModCmd::Compile { args } => crate::compile::run(&args),
        ModCmd::CompileSelftest => crate::compile::run_selftest(),
        ModCmd::CompilePreflight => crate::compile::run_preflight(),
        ModCmd::WorldBoot { args } => crate::world_boot::run(&args),
        ModCmd::Wave { args } => crate::wave_execution::run(&args),
    }
}
