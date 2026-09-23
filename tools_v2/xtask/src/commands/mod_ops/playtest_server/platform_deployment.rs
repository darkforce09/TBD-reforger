//! The mission a playtest server runs, provisioned the way the platform deploys it.
//!
//! With `--mission=<uuid>` the playtest logs in as a development administrator and:
//! 1. takes the mission's approved artifact, submitting and approving its current version first
//!    when it has none;
//! 2. makes sure the artifact's terrain has a fleet scenario (Everon's is the dev profile's
//!    scenario; any other terrain must already be registered);
//! 3. uses `--server=<uuid>` or the "TBD Playtest" server row, whose required modpack follows the
//!    artifact's;
//! 4. issues this run a `mod_runtime` machine credential, which goes into the profile's backend
//!    config;
//! 5. requests the deployment, whose scenario the rendered `server.json` runs.
//!
//! Once the server is up the playtest waits for the runtime session to confirm the deployment
//! with the exact artifact and cancels the transition command no executor claimed, so no host
//! agent later acts on it; when the server stops, the run's credential is revoked.
//!
//! With `--artifact-file=<path>` nothing touches the platform: the document is staged as the
//! mod's last verified artifact and boots offline.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};

use super::Opts;
use crate::commands::mod_ops::website_api_client::{
    ApiClient, CurlTransport, DeploymentSettlement, StagedArtifact, approved_artifact,
    cancel_unclaimed_command, development_login, ensure_fleet_scenario, ensure_server,
    issue_credential, request_deployment, revoke_credential, sha256_hex, stage_artifact_cache,
    wait_for_deployment,
};

/// The name of the server row a playtest deploys to when no `--server` is given.
const PLAYTEST_SERVER_NAME: &str = "TBD Playtest";
/// How long the playtest waits for the booted runtime to confirm its deployment.
const CONFIRMATION_PATIENCE: Duration = Duration::from_secs(180);

/// Everything the playtest provisioned on the platform for this run.
pub struct ProvisionedDeployment {
    pub api_base: String,
    pub admin_token: String,
    pub server_id: String,
    pub credential_id: String,
    pub credential_secret: String,
    pub deployment_id: String,
    pub scenario_id: String,
    pub transition_command_id: String,
    pub artifact_id: String,
}

/// Provision the deployment of `--mission` (see the module header).
pub fn provision(o: &Opts, dev_scenario: &str) -> Result<ProvisionedDeployment> {
    let transport = CurlTransport::new(Path::new(&o.run_dir).join("api"), 60);
    let token = development_login(&transport, &o.backend_url, "admin")
        .context("log in as a development administrator")?;
    let client = ApiClient::new(&transport, &o.backend_url, &token);
    let approved = approved_artifact(&client, &o.mission)
        .with_context(|| format!("take the approved artifact of mission {}", o.mission))?;
    if approved.terrain == "everon" {
        ensure_fleet_scenario(&client, "everon", dev_scenario, "Everon")?;
    }
    let server_id = if o.server.is_empty() {
        let port: u16 = o.game_port.parse().unwrap_or(2001);
        ensure_server(
            &client,
            PLAYTEST_SERVER_NAME,
            port,
            approved.modpack_id.as_deref(),
        )?
    } else {
        o.server.clone()
    };
    let credential = issue_credential(
        &client,
        &server_id,
        "mod_runtime",
        &format!("Playtest runtime ({})", o.run_dir),
    )?;
    let event_mission = (!o.event_mission.is_empty()).then_some(o.event_mission.as_str());
    let deployment = request_deployment(
        &client,
        &server_id,
        &o.mission,
        &approved.artifact_id,
        event_mission,
    )
    .inspect_err(|_| {
        let _ = revoke_credential(
            &client,
            &server_id,
            &credential.credential_id,
            "the deployment was refused",
        );
    })
    .with_context(|| {
        format!(
            "deploy mission {} on server {server_id} (a terrain other than Everon needs its fleet \
                 scenario registered in Server Control first)",
            o.mission
        )
    })?;
    let text = |pointer: &str| {
        deployment
            .pointer(pointer)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    Ok(ProvisionedDeployment {
        api_base: o.backend_url.clone(),
        admin_token: token,
        server_id,
        credential_id: credential.credential_id,
        credential_secret: credential.secret,
        deployment_id: text("/id"),
        scenario_id: text("/scenario_id"),
        transition_command_id: text("/fleet_command_id"),
        artifact_id: approved.artifact_id,
    })
}

/// After the boot: wait for the runtime to confirm the deployment, then cancel the transition
/// command no executor claimed.
pub fn confirm(provisioned: &ProvisionedDeployment, run_dir: &str) {
    let transport = CurlTransport::new(Path::new(run_dir).join("api"), 30);
    let client = ApiClient::new(&transport, &provisioned.api_base, &provisioned.admin_token);
    println!(
        "==> waiting for the runtime to confirm deployment {}",
        provisioned.deployment_id
    );
    match wait_for_deployment(
        &client,
        &provisioned.server_id,
        &provisioned.deployment_id,
        CONFIRMATION_PATIENCE,
        Duration::from_secs(5),
    ) {
        Ok(DeploymentSettlement::Confirmed { runtime_session_id }) => println!(
            "    CONFIRMED: runtime session {runtime_session_id} loaded artifact {}",
            provisioned.artifact_id
        ),
        Ok(DeploymentSettlement::Failed { state, reason }) => {
            println!("    DEPLOYMENT {}: {reason}", state.to_uppercase())
        }
        Ok(DeploymentSettlement::StillRequested) => println!(
            "    not confirmed within {} s — the server log's [TBD][Mission] lines say why",
            CONFIRMATION_PATIENCE.as_secs()
        ),
        Err(error) => println!("    could not read the deployment: {error:#}"),
    }
    match cancel_unclaimed_command(
        &client,
        &provisioned.server_id,
        &provisioned.transition_command_id,
    ) {
        Ok(true) => println!(
            "    cancelled the unclaimed transition command (the playtest ran the restart itself)"
        ),
        Ok(false) => {}
        Err(error) => println!("    could not cancel the transition command: {error:#}"),
    }
}

/// Revoke the run's credential once the server has stopped.
pub fn release(provisioned: &ProvisionedDeployment, run_dir: &str) {
    let transport = CurlTransport::new(Path::new(run_dir).join("api"), 30);
    let client = ApiClient::new(&transport, &provisioned.api_base, &provisioned.admin_token);
    match revoke_credential(
        &client,
        &provisioned.server_id,
        &provisioned.credential_id,
        "the playtest server stopped",
    ) {
        Ok(()) => println!("==> revoked this run's machine credential"),
        Err(error) => println!(
            "==> could not revoke credential {} ({error:#}); revoke it in Server Control",
            provisioned.credential_id
        ),
    }
}

/// `--artifact-file`: stage the document as the mod's last verified artifact; its artifact id.
pub fn stage_offline_artifact(o: &Opts) -> Result<String> {
    let document = std::fs::read(&o.artifact_file)
        .with_context(|| format!("read --artifact-file={}", o.artifact_file))?;
    let parsed: serde_json::Value = serde_json::from_slice(&document)
        .with_context(|| format!("--artifact-file={} is not JSON", o.artifact_file))?;
    let Some(mission_id) = parsed.pointer("/meta/id").and_then(|v| v.as_str()) else {
        bail!(
            "--artifact-file={} has no meta.id: it is not a compiled mission document",
            o.artifact_file
        );
    };
    let artifact_id = format!("playtest-offline-{}", &sha256_hex(&document)[..12]);
    let profile = Path::new(&o.run_dir).join("profile/profile");
    stage_artifact_cache(
        &profile,
        &document,
        &StagedArtifact {
            artifact_id: &artifact_id,
            mission_id,
            terrain_key: parsed
                .pointer("/meta/terrain")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        },
    )?;
    Ok(artifact_id)
}
