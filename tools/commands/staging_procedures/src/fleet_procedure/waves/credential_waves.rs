//! W11 and W12: the machine credential rotations, the host agent's on the first fleet server and
//! the game runtime's on the second: a new credential staged on the host, the old one revoked in
//! the browser, the new one promoted and read by a restarted program.
//!
//! **Role:** builds the three steps of each rotation and their judges, deciding
//! `machine_credential_rotation_host_agent` and `machine_credential_rotation_mod_runtime`.
//!
//! **Position:** called by `waves/mod.rs`; runs `remote_actions/host_fixture_commands.rs`'
//! `rotate-credential` as host actions, and the staging deploy's profile commands
//! ([`instance_profile_commands`]) in the instance's folders ([`InstanceFolder`]) after a
//! `mod_runtime` promotion; reads `single_server_reads.rs`' credentials (judged in
//! `credential_judges.rs`), the host agent's unit journal and the game runtime's `console.log`.
//!
//! **Signals & state:** none; builders and boxed judges. The stage step measures the staged and
//! the previous credential ids, and the game runtime's revocation the generation it ended, for
//! the steps after them.
//!
//! **Invariants:** a rotation replaces exactly one live credential with exactly one new one; the
//! revocation must name the previous credential, never the staged one; once revoked, the previous
//! credential is answered 401 (the host agent's journal) or its session ends
//! `credential_revoked` and the game runtime logs its credential refused, and it authenticates
//! nothing after the revocation; after the promotion the new credential authenticates the agent's
//! claims, or opens a newer runtime-session generation that heartbeats. The game runtime reads
//! its credential from the instance profile's `TBD_BackendConfig.json` at start, so its promotion
//! rewrites that profile before the restart with the very commands `cargo xtask deploy staging`
//! writes it with, never a second writer of the file; no secret is an argument or reaches stdout.

use crate::error::Result;

use super::credential_judges::{
    Rotation, new_credential_used, new_generation, previous_revoked, session_revoked, staged,
};
use super::single_server_probes::{journal_probe, single_server_step, step_effect};
use super::wave_table::{
    PROMOTE_HOST_AGENT_STEP, PROMOTE_MOD_RUNTIME_STEP, REVOKE_HOST_AGENT_STEP,
    REVOKE_MOD_RUNTIME_STEP, STAGE_HOST_AGENT_STEP, STAGE_MOD_RUNTIME_STEP,
};
use super::{FleetServer, WaveTargets};
use crate::fleet_procedure::fleet_cases::{
    MACHINE_CREDENTIAL_ROTATION_HOST_AGENT, MACHINE_CREDENTIAL_ROTATION_MOD_RUNTIME,
};
use crate::fleet_procedure::fleet_reads::json_rows;
use crate::fleet_procedure::single_server_reads::{self, CredentialRow};
use crate::procedure_receipts::CaseName;
use crate::procedure_runner::step::{
    Probe, ProbeVerdict, RequestPredicate, Step, StepContext, StepKind,
};
use crate::remote_actions::host_fixture_commands::{
    CredentialExecutor, RotationStage, rotate_credential,
};
use crate::remote_observers::console_log_reader;
use crate::remote_observers::remote_command::{RemoteCommand, shell_quote};
use crate::staging_settings::StagingSettings;
use deployment::remote_rust_toolchain::PUT_RUST_TOOLCHAIN_ON_PATH;
use deployment::staging::fleet_instances::InstanceFolder;
use deployment::staging::payloads::instance_profile_commands;

/// The words the game runtime logs when its session loop stops.
const SESSION_LOOP_STOPPED: &str = "runtime session loop STOPPED";

const HOST_AGENT_ROTATION: Rotation = Rotation {
    executor: CredentialExecutor::HostAgent,
    stage: &STAGE_HOST_AGENT_STEP,
    revoke: &REVOKE_HOST_AGENT_STEP,
    promote: &PROMOTE_HOST_AGENT_STEP,
    case: MACHINE_CREDENTIAL_ROTATION_HOST_AGENT,
};

const MOD_RUNTIME_ROTATION: Rotation = Rotation {
    executor: CredentialExecutor::ModRuntime,
    stage: &STAGE_MOD_RUNTIME_STEP,
    revoke: &REVOKE_MOD_RUNTIME_STEP,
    promote: &PROMOTE_MOD_RUNTIME_STEP,
    case: MACHINE_CREDENTIAL_ROTATION_MOD_RUNTIME,
};

/// The steps of W11 and W12.
pub(super) fn steps(targets: &WaveTargets, settings: &StagingSettings) -> Result<Vec<Step>> {
    let mut steps = rotation_steps(targets, settings, &HOST_AGENT_ROTATION)?;
    steps.extend(rotation_steps(targets, settings, &MOD_RUNTIME_ROTATION)?);
    Ok(steps)
}

fn rotation_steps(
    targets: &WaveTargets,
    settings: &StagingSettings,
    rotation: &'static Rotation,
) -> Result<Vec<Step>> {
    let server = targets.server(rotation.stage.server)?;
    let case = CaseName::new(rotation.case)?;
    let executor = rotation.executor.as_str();
    let stage_command = rotate_credential(
        settings,
        server.instance,
        rotation.executor,
        RotationStage::Stage,
    )?;
    let stage = single_server_step(
        rotation.stage,
        server,
        StepKind::HostAction(stage_command),
        None,
        vec![step_effect(
            rotation.stage,
            server,
            "staged",
            &format!("new {executor} credential staged beside the live one"),
            credentials_probe(targets, server, rotation, move |rows, context| {
                staged(rows, context, rotation)
            }),
            &case,
        )],
    )?;
    let request = RequestPredicate {
        description: format!(
            "the previous {executor} credential of {} revoked",
            server.name
        ),
        probe: credentials_probe(targets, server, rotation, move |rows, context| {
            previous_revoked(rows, context, rotation)
        }),
    };
    let (revoke_effects, promote_command, promote_effect) = match rotation.executor {
        CredentialExecutor::HostAgent => (
            vec![step_effect(
                rotation.revoke,
                server,
                "old_refused",
                "host agent's claims with the revoked credential answered 401",
                journal_probe(&server.host_agent_unit, |lines, _| {
                    match lines.iter().find(|line| {
                        line.text.contains("401")
                            && line.text.contains("machine credential revoked")
                    }) {
                        Some(line) => ProbeVerdict::Satisfied(
                            ProbeVerdict::satisfied(format!(
                                "the host agent logged at {}: {}",
                                line.unix_ms,
                                line.text.trim()
                            ))
                            .at(line.unix_ms),
                        ),
                        None => ProbeVerdict::Pending(
                            "no claim answered 401 `machine credential revoked` in the host \
                             agent's journal since the revocation"
                                .into(),
                        ),
                    }
                }),
                &case,
            )],
            promote_host_agent(settings, server)?,
            step_effect(
                rotation.promote,
                server,
                "new_accepted",
                "host agent authenticates with the promoted credential",
                credentials_probe(targets, server, rotation, move |rows, context| {
                    new_credential_used(rows, context, rotation)
                }),
                &case,
            ),
        ),
        CredentialExecutor::ModRuntime => (
            vec![
                step_effect(
                    rotation.revoke,
                    server,
                    "session_revoked",
                    "runtime session of the revoked credential ended credential_revoked",
                    credentials_probe(targets, server, rotation, move |rows, context| {
                        session_revoked(rows, context, rotation)
                    }),
                    &case,
                ),
                step_effect(
                    rotation.revoke,
                    server,
                    "old_refused",
                    "game runtime logged its credential refused",
                    runtime_refusal_logged(targets, server),
                    &case,
                ),
            ],
            promote_mod_runtime(settings, targets, server)?,
            step_effect(
                rotation.promote,
                server,
                "new_generation",
                "runtime session opened with the promoted credential heartbeats",
                credentials_probe(targets, server, rotation, move |rows, context| {
                    new_generation(rows, context, rotation)
                }),
                &case,
            ),
        ),
    };
    let revoke = single_server_step(
        rotation.revoke,
        server,
        StepKind::ChromeAction,
        Some(request),
        revoke_effects,
    )?;
    let promote = single_server_step(
        rotation.promote,
        server,
        StepKind::HostAction(promote_command),
        None,
        vec![promote_effect],
    )?;
    Ok(vec![stage, revoke, promote])
}

/// Promotes the staged `host_agent` credential of `server` and restarts its host agent, which
/// reads its credential file only at start.
fn promote_host_agent(settings: &StagingSettings, server: &FleetServer) -> Result<RemoteCommand> {
    let promote = rotate_credential(
        settings,
        server.instance,
        CredentialExecutor::HostAgent,
        RotationStage::Promote,
    )?;
    Ok(RemoteCommand::change(
        "host tool",
        format!(
            "{} && systemctl --user restart {}",
            promote.command_line,
            shell_quote(&server.host_agent_unit)
        ),
    ))
}

/// Promotes the staged `mod_runtime` credential of `server`, rewrites the instance profile's
/// `TBD_BackendConfig.json` with the staging deploy's own profile commands
/// ([`instance_profile_commands`], which read the promoted secret on the host) in the instance's
/// folder and secrets folder ([`InstanceFolder`]), and restarts the game server.
fn promote_mod_runtime(
    settings: &StagingSettings,
    targets: &WaveTargets,
    server: &FleetServer,
) -> Result<RemoteCommand> {
    let promote = rotate_credential(
        settings,
        server.instance,
        CredentialExecutor::ModRuntime,
        RotationStage::Promote,
    )?;
    let folder = InstanceFolder::under(&targets.fleet_root, server.instance);
    let script = format!(
        "set -euo pipefail\n\
         {promote}\n\
         {PUT_RUST_TOOLCHAIN_ON_PATH}\n\
         INSTANCE={instance}\n\
         SECRETS={secrets}\n\
         {profile}\
         systemctl --user restart {unit}\n",
        promote = promote.command_line,
        instance = shell_quote(folder.path()),
        secrets = shell_quote(&folder.secrets()),
        profile = instance_profile_commands(&settings.checkout, &settings.api_origin),
        unit = shell_quote(&server.unit),
    );
    Ok(RemoteCommand::change_script("host tool", script))
}

/// A probe of `server`'s credentials of the rotation's executor, judged by `judge`.
fn credentials_probe(
    targets: &WaveTargets,
    server: &FleetServer,
    rotation: &'static Rotation,
    judge: impl Fn(&[CredentialRow], &StepContext<'_>) -> ProbeVerdict + 'static,
) -> Probe {
    let container = targets.database_container.clone();
    let name = server.name.clone();
    Probe::host(
        move |_| {
            single_server_reads::machine_credentials(&container, &name, rotation.executor.as_str())
        },
        move |text, context| match json_rows::<CredentialRow>(text) {
            Err(error) => ProbeVerdict::Contradicted(format!("{error:#}")),
            Ok(rows) => judge(&rows, context),
        },
    )
}

/// Holds when `server`'s newest `console.log` shows the session loop stopped over its machine
/// credential.
fn runtime_refusal_logged(targets: &WaveTargets, server: &FleetServer) -> Probe {
    let fleet_root = targets.fleet_root.clone();
    let instance = server.instance;
    Probe::host(
        move |_| Ok(console_log_reader::newest(&fleet_root, instance)),
        move |text, _| {
            let Some(log) = console_log_reader::parse(text) else {
                return ProbeVerdict::Pending("the console log read names no log".into());
            };
            match log.text.lines().find(|line| {
                line.contains(SESSION_LOOP_STOPPED) && line.contains("machine credential")
            }) {
                Some(line) => ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!(
                    "{} logged `{}`",
                    log.path,
                    line.trim()
                ))),
                None => ProbeVerdict::Pending(format!(
                    "{} shows no session loop stopped over the machine credential",
                    log.path
                )),
            }
        },
    )
}
