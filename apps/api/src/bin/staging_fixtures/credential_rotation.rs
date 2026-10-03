//! `rotate-credential`: rotates one machine credential of a fleet instance in two steps. `--stage`
//! issues a new credential into a staged file beside the live one; the operator then revokes the
//! old credential in the Server Control page; `--promote` moves the staged file over the live one,
//! and the next restart of the program reads it.
//!
//! **Role:** the host half of a rotation: issuing the new secret into a file and switching the file
//! the host agent or game runtime reads. Revocation stays in the administrator interface.
//!
//! **Position:** a subcommand of the table in `main.rs`. Finds the instance's server by
//! [`fleet_server_name`], issues through [`issue_machine_credential`], and writes, reads and
//! promotes files through `secret_files`.
//!
//! **Signals & state:** one database transaction for `--stage`; `--promote` only reads the
//! database and renames one file.
//!
//! **Invariants:** a dry run and a refusal write nothing. `--stage` commits its credential and
//! `server.credential_issued` audit row only after the staged file is written and synced, and
//! never replaces a staged file. `--promote` moves a file only when its secret is an unrevoked
//! credential of that instance's server and executor, and the move is one rename, so the live
//! path holds the old secret or the new one at every instant. No secret reaches stdout or stderr.

use api::core::authentication_primitives::hash_token;
use api::server_infrastructure::services::machine_credentials::issue_machine_credential;
use chrono::Utc;
use fleet_wire_contract::ExecutorKind;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::argument_list::ArgumentList;
use crate::fleet_provisioning::{
    MAXIMUM_FLEET_INSTANCES, credential_label, fleet_server_name, undo_files,
};
use crate::guarded_context::{GuardedContext, ParsedSubcommand};
use crate::secret_files::{
    DirectoryState, SecretFileLayout, inspect_secrets_directory, promote_staged_file,
    read_private_secret_file, require_absent, sync_directory, write_new_secret_file,
};
use crate::tool_failure::ToolFailure;

/// Which half of a rotation a run performs.
enum RotationStep {
    /// Issue a new credential for `actor` into the staged file.
    Stage { actor: String },
    /// Move the staged file over the live one.
    Promote,
}

/// One parsed `rotate-credential` run.
struct RotationRequest {
    instance: u32,
    executor: ExecutorKind,
    step: RotationStep,
    layout: SecretFileLayout,
}

/// A stored credential as the promotion checks it.
#[derive(sqlx::FromRow)]
struct StoredCredential {
    id: Uuid,
    server_id: Uuid,
    executor_kind: String,
    revoked: bool,
}

/// Parse `--instance <n> --executor host_agent|mod_runtime --secrets-root <path>` with either
/// `--stage --actor <discord id>` or `--promote`.
pub(crate) fn parse(arguments: &mut ArgumentList) -> Result<ParsedSubcommand, ToolFailure> {
    let instance = arguments.required_parsed::<u32>("--instance", "a whole number")?;
    if !(1..=MAXIMUM_FLEET_INSTANCES).contains(&instance) {
        return Err(ToolFailure::refused(format!(
            "--instance takes 1 to {MAXIMUM_FLEET_INSTANCES}, not {instance}"
        )));
    }
    let executor_text = arguments.required("--executor")?;
    let executor = ExecutorKind::parse(&executor_text).ok_or_else(|| {
        ToolFailure::refused(format!(
            "--executor takes host_agent or mod_runtime, not `{executor_text}`"
        ))
    })?;
    let layout = SecretFileLayout::new(arguments.required("--secrets-root")?)?;
    let step = match (arguments.switch("--stage")?, arguments.switch("--promote")?) {
        (true, false) => RotationStep::Stage {
            actor: arguments.required("--actor")?,
        },
        (false, true) => RotationStep::Promote,
        _ => {
            return Err(ToolFailure::refused(
                "pass exactly one of --stage and --promote",
            ));
        }
    };
    let request = RotationRequest {
        instance,
        executor,
        step,
        layout,
    };
    Ok(Box::new(move |context| Box::pin(rotate(context, request))))
}

async fn rotate(context: GuardedContext, request: RotationRequest) -> Result<(), ToolFailure> {
    match &request.step {
        RotationStep::Stage { actor } => stage(&context, &request, actor).await,
        RotationStep::Promote => promote(&context, &request).await,
    }
}

async fn stage(
    context: &GuardedContext,
    request: &RotationRequest,
    actor: &str,
) -> Result<(), ToolFailure> {
    let mut transaction = context.pool.begin().await?;
    context
        .require_administrator_actor(&mut transaction, actor)
        .await?;
    let server = locked_fleet_server(&mut transaction, request.instance).await?;
    let directory = provisioned_secrets_directory(request)?;
    let staged = request
        .layout
        .staged_credential_file(request.instance, request.executor);
    require_absent(&staged)?;
    println!(
        "plan instance={} executor={} server={server} staged-file={}",
        request.instance,
        request.executor.as_str(),
        staged.display()
    );
    for (id, label) in active_credentials(&mut transaction, server, request.executor).await? {
        println!("revoke-after-staging credential id={id} label=\"{label}\"");
    }
    if !context.mode.writes() {
        return Ok(());
    }

    let label = format!(
        "{} rotation {}",
        credential_label(request.instance, request.executor),
        Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
    );
    let issued =
        issue_machine_credential(&mut transaction, server, request.executor, &label, actor).await?;
    write_new_secret_file(&staged, &issued.secret)?;
    let written = [staged.clone()];
    if let Err(failure) = sync_directory(&directory) {
        return Err(undo_files(failure, &written));
    }
    if let Err(error) = transaction.commit().await {
        return Err(undo_files(ToolFailure::from(error), &written));
    }
    println!(
        "credential instance={} executor={} id={} file={} state=staged",
        request.instance,
        request.executor.as_str(),
        issued.credential.id,
        staged.display()
    );
    Ok(())
}

async fn promote(context: &GuardedContext, request: &RotationRequest) -> Result<(), ToolFailure> {
    let mut transaction = context.pool.begin().await?;
    let server = locked_fleet_server(&mut transaction, request.instance).await?;
    provisioned_secrets_directory(request)?;
    let staged = request
        .layout
        .staged_credential_file(request.instance, request.executor);
    let live = request
        .layout
        .credential_file(request.instance, request.executor);
    let secret = read_private_secret_file(&staged)?.ok_or_else(|| {
        ToolFailure::refused(format!(
            "no staged credential at {}; run rotate-credential --stage first",
            staged.display()
        ))
    })?;
    let credential: StoredCredential = sqlx::query_as(
        "SELECT id, server_id, executor_kind, revoked_at IS NOT NULL AS revoked
         FROM server_machine_credentials WHERE secret_sha256 = $1",
    )
    .bind(hash_token(&secret))
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| {
        ToolFailure::refused(format!(
            "{} holds no credential the database knows",
            staged.display()
        ))
    })?;
    if credential.server_id != server || credential.executor_kind != request.executor.as_str() {
        return Err(ToolFailure::refused(format!(
            "the staged credential {} belongs to another server or executor",
            credential.id
        )));
    }
    if credential.revoked {
        return Err(ToolFailure::refused(format!(
            "the staged credential {} is revoked; stage a new one",
            credential.id
        )));
    }
    println!(
        "plan instance={} executor={} credential={} promote {} -> {}",
        request.instance,
        request.executor.as_str(),
        credential.id,
        staged.display(),
        live.display()
    );
    for (id, label) in active_credentials(&mut transaction, server, request.executor).await? {
        if id != credential.id {
            println!("still-active credential id={id} label=\"{label}\"");
        }
    }
    if !context.mode.writes() {
        return Ok(());
    }
    promote_staged_file(&staged, &live)?;
    println!(
        "credential instance={} executor={} id={} file={} state=promoted",
        request.instance,
        request.executor.as_str(),
        credential.id,
        live.display()
    );
    Ok(())
}

/// The id of instance `instance`'s active server, locked as the credential routes lock it.
async fn locked_fleet_server(
    connection: &mut PgConnection,
    instance: u32,
) -> Result<Uuid, ToolFailure> {
    let name = fleet_server_name(instance);
    let servers: Vec<(Uuid, bool)> =
        sqlx::query_as("SELECT id, is_active FROM servers WHERE name = $1 FOR NO KEY UPDATE")
            .bind(&name)
            .fetch_all(connection)
            .await?;
    match servers.as_slice() {
        [(id, true)] => Ok(*id),
        [] => Err(ToolFailure::refused(format!(
            "no server is named \"{name}\"; provision-fleet registers it"
        ))),
        [(_, false)] => Err(ToolFailure::refused(format!(
            "server \"{name}\" is deactivated"
        ))),
        several => Err(ToolFailure::refused(format!(
            "{} servers are named \"{name}\"; the instance is ambiguous",
            several.len()
        ))),
    }
}

/// The instance's secrets directory, which provisioning created and which must still be private.
fn provisioned_secrets_directory(
    request: &RotationRequest,
) -> Result<std::path::PathBuf, ToolFailure> {
    let directory = request.layout.secrets_directory(request.instance);
    match inspect_secrets_directory(&directory)? {
        DirectoryState::Private => Ok(directory),
        DirectoryState::Absent => Err(ToolFailure::refused(format!(
            "{} does not exist; provision-fleet creates it",
            directory.display()
        ))),
    }
}

/// The unrevoked credentials of `executor` on `server`, oldest first, with their labels.
async fn active_credentials(
    connection: &mut PgConnection,
    server: Uuid,
    executor: ExecutorKind,
) -> Result<Vec<(Uuid, String)>, ToolFailure> {
    Ok(sqlx::query_as(
        "SELECT id, label FROM server_machine_credentials
         WHERE server_id = $1 AND executor_kind = $2 AND revoked_at IS NULL
         ORDER BY created_at, id",
    )
    .bind(server)
    .bind(executor.as_str())
    .fetch_all(connection)
    .await?)
}
