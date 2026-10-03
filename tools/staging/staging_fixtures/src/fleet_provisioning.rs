//! `provision-fleet`: registers the staging fleet's servers, `"TBD Staging 1"` to
//! `"TBD Staging <n>"`, and writes each server's two machine credentials into private files.
//!
//! **Role:** turns `--instances`, `--ip`, `--game-port-base` and `--secrets-root` into new server
//! rows, a `host_agent` and a `mod_runtime` credential per server, and one mode-600 file per
//! credential, attributed to `--actor`.
//!
//! **Position:** a subcommand of the table in `main.rs`. Writes through
//! [`register_server`] and [`issue_machine_credential`], the same services the administrator
//! routes use, and through `secret_files`; `credential_rotation` finds the servers again by
//! [`fleet_server_name`].
//!
//! **Signals & state:** one database transaction and the secret files it writes.
//!
//! **Invariants:** a dry run and a refusal write nothing, and a dry run runs every check an apply
//! runs. An apply is all or nothing: the servers, credentials and their `server.create` and
//! `server.credential_issued` audit rows commit only after every secret file is written and
//! synced, and a failure rolls the transaction back and removes the files it wrote (the private
//! directories it created after the database writes stay, empty). Instance `n` listens on game
//! port `base + n`. No secret reaches stdout or stderr.

use api_identifiers::DiscordUserId;
use std::path::PathBuf;

use api_server_infrastructure::services::machine_credentials::issue_machine_credential;
use api_server_infrastructure::services::server_registration::{
    ServerRegistration, register_server,
};
use fleet_wire_contract::ExecutorKind;

use crate::argument_list::ArgumentList;
use crate::guarded_context::{GuardedContext, ParsedSubcommand};
use crate::secret_files::{
    DirectoryState, SecretFileLayout, create_private_directory, inspect_secrets_directory,
    remove_files, require_absent, sync_directory, write_new_secret_file,
};
use crate::tool_failure::ToolFailure;

/// The most instances one run registers, a guard against a mistyped count.
pub(crate) const MAXIMUM_FLEET_INSTANCES: u32 = 10;
/// Instance `n` publishes game port `2000 + n` unless `--game-port-base` says otherwise.
const DEFAULT_GAME_PORT_BASE: i64 = 2000;
/// Every fleet server gets one credential of each executor kind.
const FLEET_EXECUTORS: [ExecutorKind; 2] = [ExecutorKind::HostAgent, ExecutorKind::ModRuntime];

/// The registered name of instance `instance`.
pub(crate) fn fleet_server_name(instance: u32) -> String {
    format!("TBD Staging {instance}")
}

/// The label of `executor`'s credential on instance `instance`, such as
/// `TBD Staging 1 host agent`.
pub(crate) fn credential_label(instance: u32, executor: ExecutorKind) -> String {
    format!(
        "{} {}",
        fleet_server_name(instance),
        executor.as_str().replace('_', " ")
    )
}

/// One instance to register.
struct InstancePlan {
    instance: u32,
    registration: ServerRegistration,
}

/// Everything a provisioning run registers and writes.
struct FleetPlan {
    actor: DiscordUserId,
    layout: SecretFileLayout,
    instances: Vec<InstancePlan>,
}

/// One credential issued in the run's transaction, with the file its secret goes to.
struct IssuedFile {
    instance: u32,
    executor: ExecutorKind,
    credential_id: uuid::Uuid,
    secret: String,
    path: PathBuf,
}

/// Parse `--instances <n> --actor <discord id> --ip <address> --secrets-root <path>
/// [--game-port-base <port>]` and validate every server registration before any guard runs.
pub(crate) fn parse(arguments: &mut ArgumentList) -> Result<ParsedSubcommand, ToolFailure> {
    let count = arguments.required_parsed::<u32>("--instances", "a whole number")?;
    if !(1..=MAXIMUM_FLEET_INSTANCES).contains(&count) {
        return Err(ToolFailure::refused(format!(
            "--instances takes 1 to {MAXIMUM_FLEET_INSTANCES}, not {count}"
        )));
    }
    let actor = arguments.required("--actor")?;
    let ip = arguments.required("--ip")?;
    let port_base = arguments
        .optional_parsed::<i64>("--game-port-base", "a port number")?
        .unwrap_or(DEFAULT_GAME_PORT_BASE);
    let layout = SecretFileLayout::new(arguments.required("--secrets-root")?)?;
    let instances = (1..=count)
        .map(|instance| {
            let name = fleet_server_name(instance);
            let port = port_base.saturating_add(i64::from(instance));
            ServerRegistration::new(&name, &ip, port, None, true)
                .map(|registration| InstancePlan {
                    instance,
                    registration,
                })
                .map_err(|error| {
                    ToolFailure::refused(format!("server {name} at {ip}:{port}: {}", error.message))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let plan = FleetPlan {
        actor: actor.into(),
        layout,
        instances,
    };
    Ok(Box::new(move |context| Box::pin(provision(context, plan))))
}

async fn provision(context: GuardedContext, plan: FleetPlan) -> Result<(), ToolFailure> {
    let mut transaction = context.pool.begin().await?;
    context
        .require_administrator_actor(&mut transaction, &plan.actor)
        .await?;
    refuse_registered_names(&mut transaction, &plan).await?;
    let mut absent_directories = Vec::new();
    for planned in &plan.instances {
        let directory = plan.layout.secrets_directory(planned.instance);
        if inspect_secrets_directory(&directory)? == DirectoryState::Absent {
            absent_directories.push(directory.clone());
        }
        for executor in FLEET_EXECUTORS {
            require_absent(&plan.layout.credential_file(planned.instance, executor))?;
        }
        println!(
            "plan instance={} server=\"{}\" address={}:{} secrets={}",
            planned.instance,
            planned.registration.name(),
            planned.registration.ip(),
            planned.registration.port(),
            directory.display()
        );
    }
    if !context.mode.writes() {
        return Ok(());
    }

    let mut servers = Vec::new();
    let mut issued = Vec::new();
    for planned in &plan.instances {
        let server = register_server(&mut transaction, &planned.registration, &plan.actor).await?;
        for executor in FLEET_EXECUTORS {
            let credential = issue_machine_credential(
                &mut transaction,
                server.id,
                executor,
                &credential_label(planned.instance, executor),
                &plan.actor,
            )
            .await?;
            issued.push(IssuedFile {
                instance: planned.instance,
                executor,
                credential_id: credential.credential.id.into_inner(),
                secret: credential.secret,
                path: plan.layout.credential_file(planned.instance, executor),
            });
        }
        servers.push((planned.instance, server));
    }

    for directory in &absent_directories {
        create_private_directory(directory)?;
    }
    let mut written = Vec::new();
    for file in &issued {
        if let Err(failure) = write_new_secret_file(&file.path, &file.secret) {
            return Err(undo_files(failure, &written));
        }
        written.push(file.path.clone());
    }
    for planned in &plan.instances {
        if let Err(failure) = sync_directory(&plan.layout.secrets_directory(planned.instance)) {
            return Err(undo_files(failure, &written));
        }
    }
    if let Err(error) = transaction.commit().await {
        return Err(undo_files(ToolFailure::from(error), &written));
    }

    for (instance, server) in &servers {
        println!(
            "server instance={instance} id={} name=\"{}\" address={}:{}",
            server.id, server.name, server.ip, server.port
        );
    }
    for file in &issued {
        println!(
            "credential instance={} executor={} id={} file={}",
            file.instance,
            file.executor.as_str(),
            file.credential_id,
            file.path.display()
        );
    }
    println!(
        "provisioned {} servers and {} credentials",
        servers.len(),
        issued.len()
    );
    Ok(())
}

/// Refuse when any planned name is already registered: the fleet's servers are always new.
async fn refuse_registered_names(
    connection: &mut sqlx::PgConnection,
    plan: &FleetPlan,
) -> Result<(), ToolFailure> {
    let names: Vec<String> = plan
        .instances
        .iter()
        .map(|planned| planned.registration.name().to_owned())
        .collect();
    let taken: Vec<String> =
        sqlx::query_scalar("SELECT name FROM servers WHERE name = ANY($1) ORDER BY name")
            .bind(&names)
            .fetch_all(connection)
            .await?;
    if taken.is_empty() {
        Ok(())
    } else {
        Err(ToolFailure::refused(format!(
            "servers named {} are already registered; provision-fleet registers new servers only",
            taken.join(", ")
        )))
    }
}

/// Remove the files a failed apply wrote and name any that stayed; their credentials never
/// committed, so they authenticate nothing.
pub(crate) fn undo_files(failure: ToolFailure, written: &[PathBuf]) -> ToolFailure {
    let remaining = remove_files(written);
    if remaining.is_empty() {
        return failure;
    }
    let (ToolFailure::Refused(reason) | ToolFailure::Failed(reason)) = failure;
    let paths: Vec<String> = remaining
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    ToolFailure::failed(format!(
        "{reason}; these files hold no valid credential and could not be removed: {}",
        paths.join(", ")
    ))
}
