//! Routes each `cargo xtask staging` subcommand to its work.
//!
//! **Role:** loads the settings and the host transport a command needs, runs it, and returns the
//! exit code.
//!
//! **Position:** called by `tools/xtask/src/cli/dispatch.rs` with the parsed [`StagingCmd`].
//!
//! **Signals & state:** none held; one ssh transport per command.
//!
//! **Invariants:** `fingerprints` and `load --rehearse-local` need no `deploy.env`; a confirmed
//! action under `--dry-run` prints its command and opens no connection; a recorded run hands the
//! recorder this process's environment for its run discipline, and its exit code is the
//! recorder's.

use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;

use super::cli::{PlanOnly, ProcedureName, StagingCmd};
use super::discord_procedure::DiscordProcedure;
use super::fleet_procedure::FleetProcedure;
use super::load_procedure::{self, LoadProcedure};
use super::operator_coordination::action_list::{render, render_cases};
use super::procedure_runner::clock::SystemClock;
use super::procedure_runner::procedure::StagingProcedure;
use super::procedure_runner::recording::{RecordingInputs, record};
use super::remote_actions::database_backup;
use super::remote_actions::game_server_update;
use super::remote_actions::host_fixture_commands::{self, RotationStage};
use super::remote_observers::host_shell::HostShell;
use super::remote_observers::remote_command::{HostCommandRunner, RemoteCommand};
use super::run_identity::recorded_command;
use super::staging_settings::StagingSettings;
use super::support_commands::{fingerprints, host_capacity, preflight, status};
use repository_layout::find_repository_root;

/// Runs one `cargo xtask staging` subcommand and returns its exit code.
pub(crate) fn run(cmd: StagingCmd) -> Result<u8> {
    let root = find_repository_root()?;
    let mut stdout = io::stdout();
    let output: &mut dyn Write = &mut stdout;
    match cmd {
        StagingCmd::Fingerprints => fingerprints::run(&root, output),
        StagingCmd::Load {
            rehearse_local: true,
            ..
        } => load_procedure::rehearse_local(&root),
        StagingCmd::ActionList {
            procedure,
            cases,
            recovery,
        } => {
            let settings = StagingSettings::load(&root)?;
            let procedure = procedure_named(procedure);
            let id = procedure.check().id();
            let text = if cases {
                render_cases(id, &procedure.plan(&settings)?.declared_cases)
            } else if recovery {
                render(
                    &format!("{id} recovery actions"),
                    &procedure.recovery_action_list(&settings),
                )
            } else {
                render(&format!("{id} actions"), &procedure.action_list(&settings))
            };
            output.write_all(text.as_bytes())?;
            Ok(0)
        }
        cmd => {
            let settings = StagingSettings::load(&root)?;
            let mut host = HostShell::new(&settings);
            with_host(cmd, &root, &settings, &mut host, output)
        }
    }
}

/// The subcommands that reach the host.
fn with_host(
    cmd: StagingCmd,
    root: &Path,
    settings: &StagingSettings,
    host: &mut dyn HostCommandRunner,
    output: &mut dyn Write,
) -> Result<u8> {
    let recording = |procedure: &dyn StagingProcedure,
                     host: &mut dyn HostCommandRunner,
                     output: &mut dyn Write| {
        record(
            procedure,
            RecordingInputs {
                root,
                settings,
                host,
                clock: &SystemClock,
                command: recorded_command(std::env::args()),
                process_environment: std::env::vars_os().collect(),
                output,
            },
        )
    };
    match cmd {
        StagingCmd::Preflight { discord } => {
            let mut checks = preflight::harness_checks(settings, root);
            checks.extend(FleetProcedure.preflight_checks(settings));
            checks.extend(LoadProcedure::default().preflight_checks(settings));
            if discord {
                checks.extend(DiscordProcedure.preflight_checks(settings));
            }
            preflight::run(&checks, host, output)
        }
        StagingCmd::Status { capacity: true } => {
            let mut units = settings.game_server_units();
            units.extend(settings.host_agent_units());
            units.extend(settings.relay_unit().map(|(_, unit)| unit));
            let answer = host.run(&host_capacity::command(&units))?;
            output.write_all(host_capacity::render(&answer.stdout, &units).as_bytes())?;
            Ok(u8::from(answer.exit_code != 0))
        }
        StagingCmd::Status { capacity: false } => {
            let items = status::items(settings, host);
            output.write_all(status::render(&items).as_bytes())?;
            Ok(status::exit_code(&items))
        }
        StagingCmd::Backup { label, plan } => {
            let command = database_backup::backup(settings, &label)?;
            let answer = confirmed_answer(&command, &plan, host, output)?;
            names_its_result(
                answer,
                "a backup file",
                database_backup::backup_line,
                output,
            )
        }
        StagingCmd::UpdateGameServer { plan } => {
            let command = game_server_update::update(settings);
            let answer = confirmed_answer(&command, &plan, host, output)?;
            names_its_result(
                answer,
                "an installed build id",
                game_server_update::build_id,
                output,
            )
        }
        StagingCmd::ProvisionFleet { plan } => {
            let command = host_fixture_commands::provision_fleet(settings)?;
            confirmed(&[command], &plan, host, output)
        }
        StagingCmd::RotateCredential {
            instance,
            executor,
            stage,
            plan,
            ..
        } => {
            let stage = if stage {
                RotationStage::Stage
            } else {
                RotationStage::Promote
            };
            let command =
                host_fixture_commands::rotate_credential(settings, instance, executor, stage)?;
            confirmed(&[command], &plan, host, output)
        }
        StagingCmd::SeedLoad { token_file, plan } => {
            load_procedure::seed_load(root, settings, &token_file, &plan, host, output)
        }
        StagingCmd::CleanLoad { plan } => load_procedure::clean_load(settings, &plan, host, output),
        StagingCmd::Fleet { .. } => recording(&FleetProcedure, host, output),
        StagingCmd::Discord { .. } => recording(&DiscordProcedure, host, output),
        StagingCmd::Load { token_file, .. } => recording(
            &LoadProcedure::recording(token_file, root, settings),
            host,
            output,
        ),
        StagingCmd::Fingerprints | StagingCmd::ActionList { .. } => {
            anyhow::bail!("this subcommand does not reach the host")
        }
    }
}

/// The procedure `name` names, for its lists and declared cases.
fn procedure_named(name: ProcedureName) -> Box<dyn StagingProcedure> {
    match name {
        ProcedureName::Fleet => Box::new(FleetProcedure),
        ProcedureName::Discord => Box::new(DiscordProcedure),
        ProcedureName::Load => Box::new(LoadProcedure::default()),
    }
}

/// Runs `commands` in order, stopping at the first that fails; under `--dry-run` prints them.
pub(crate) fn confirmed(
    commands: &[RemoteCommand],
    plan: &PlanOnly,
    host: &mut dyn HostCommandRunner,
    output: &mut dyn Write,
) -> Result<u8> {
    for command in commands {
        if let ConfirmedAnswer::Failed = confirmed_answer(command, plan, host, output)? {
            return Ok(1);
        }
    }
    Ok(0)
}

/// How one confirmed action ended.
pub(crate) enum ConfirmedAnswer {
    /// `--dry-run`: printed, not run.
    Planned,
    /// Ran and exited 0, with this standard output.
    Succeeded(String),
    /// Ran and failed; its output and exit code are printed.
    Failed,
}

/// Runs one confirmed action, or prints it under `--dry-run`.
pub(crate) fn confirmed_answer(
    command: &RemoteCommand,
    plan: &PlanOnly,
    host: &mut dyn HostCommandRunner,
    output: &mut dyn Write,
) -> Result<ConfirmedAnswer> {
    if plan.dry_run {
        writeln!(
            output,
            "[dry-run] {}: {}",
            command.observer, command.command_line
        )?;
        if let Some(stdin) = &command.stdin {
            writeln!(output, "[dry-run] stdin:\n{stdin}")?;
        }
        return Ok(ConfirmedAnswer::Planned);
    }
    writeln!(output, "==> {}", command.observer)?;
    let answer = host.run(command)?;
    output.write_all(answer.stdout.as_bytes())?;
    if answer.exit_code != 0 {
        output.write_all(answer.stderr.as_bytes())?;
        writeln!(output, "{} exited {}", command.observer, answer.exit_code)?;
        return Ok(ConfirmedAnswer::Failed);
    }
    Ok(ConfirmedAnswer::Succeeded(answer.stdout))
}

/// The exit code of an action whose successful answer must name its result (`what`): a backup
/// names its file, an update its build; a success that names nothing fails.
pub(crate) fn names_its_result(
    answer: ConfirmedAnswer,
    what: &str,
    evidence: fn(&str) -> Option<&str>,
    output: &mut dyn Write,
) -> Result<u8> {
    match answer {
        ConfirmedAnswer::Planned => Ok(0),
        ConfirmedAnswer::Failed => Ok(1),
        ConfirmedAnswer::Succeeded(stdout) => match evidence(&stdout) {
            Some(result) => {
                writeln!(output, "result: {result}")?;
                Ok(0)
            }
            None => {
                writeln!(
                    output,
                    "the answer names no {what}; treating the action as failed"
                )?;
                Ok(1)
            }
        },
    }
}
