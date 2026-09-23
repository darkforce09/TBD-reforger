//! The fleet side of a deployment, as an administrator provisions it: the terrain's fleet
//! scenario, the server row, the server's machine credentials, the deployment itself and the
//! fleet command that performs its transition.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::api_client::{ApiClient, text_at};
use super::http_exchange::encode_component;

/// A machine credential as issued: its id and the secret the platform shows exactly once.
pub struct IssuedCredential {
    pub credential_id: String,
    pub secret: String,
}

/// How a deployment ended while a tool waited for it.
#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentSettlement {
    Confirmed { runtime_session_id: String },
    Failed { state: String, reason: String },
    StillRequested,
}

/// The terrain's registered scenario, registering `scenario_id` when the terrain has none. An
/// existing registration is kept: deployments of every server use it.
pub fn ensure_fleet_scenario(
    client: &ApiClient<'_>,
    terrain_key: &str,
    scenario_id: &str,
    display_name: &str,
) -> Result<String> {
    let scenarios = client.expect_json("GET", "/api/v1/fleet/scenarios", None, 200)?;
    let registered = scenarios["items"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|item| item["terrain_key"] == terrain_key)
        .and_then(|item| item["scenario_id"].as_str().map(str::to_string));
    if let Some(registered) = registered {
        return Ok(registered);
    }
    client.expect(
        "PUT",
        &format!("/api/v1/fleet/scenarios/{}", encode_component(terrain_key)),
        Some(&json!({ "scenario_id": scenario_id, "display_name": display_name })),
        200,
    )?;
    Ok(scenario_id.to_string())
}

/// The id of the active server named `name`, registering it when there is none. The server's
/// required modpack is set to `modpack_id`, because a deployment runs only an artifact compiled
/// against the modpack its server requires.
pub fn ensure_server(
    client: &ApiClient<'_>,
    name: &str,
    port: u16,
    modpack_id: Option<&str>,
) -> Result<String> {
    let servers = client.expect_json("GET", "/api/v1/servers", None, 200)?;
    let existing = servers["data"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|server| server["name"] == name && server["is_active"] != false)
        .cloned();
    let Some(server) = existing else {
        let created = client.expect_json(
            "POST",
            "/api/v1/servers",
            Some(&json!({
                "name": name,
                "ip": "127.0.0.1",
                "port": port,
                "required_modpack_id": modpack_id,
                "is_active": true,
            })),
            201,
        )?;
        return text_at(&created, "/id", "POST /api/v1/servers");
    };
    let server_id = text_at(&server, "/id", "GET /api/v1/servers")?;
    if server["required_modpack_id"].as_str() != modpack_id {
        client.expect(
            "PATCH",
            &format!("/api/v1/servers/{}", encode_component(&server_id)),
            Some(&json!({ "required_modpack_id": modpack_id })),
            200,
        )?;
    }
    Ok(server_id)
}

/// Issue a machine credential (`host_agent` or `mod_runtime`) for the server.
pub fn issue_credential(
    client: &ApiClient<'_>,
    server_id: &str,
    executor_kind: &str,
    label: &str,
) -> Result<IssuedCredential> {
    let issued = client.expect_json(
        "POST",
        &format!(
            "/api/v1/servers/{}/credentials",
            encode_component(server_id)
        ),
        Some(&json!({ "executor_kind": executor_kind, "label": label })),
        201,
    )?;
    Ok(IssuedCredential {
        credential_id: text_at(&issued, "/credential/id", "the issued credential")?,
        secret: text_at(&issued, "/secret", "the issued credential")?,
    })
}

/// Revoke a machine credential, with the reason the platform records.
pub fn revoke_credential(
    client: &ApiClient<'_>,
    server_id: &str,
    credential_id: &str,
    reason: &str,
) -> Result<()> {
    client.expect(
        "DELETE",
        &format!(
            "/api/v1/servers/{}/credentials/{}?reason={}",
            encode_component(server_id),
            encode_component(credential_id),
            encode_component(reason)
        ),
        None,
        200,
    )?;
    Ok(())
}

/// Request a deployment of the approved artifact on the server; the recorded deployment.
pub fn request_deployment(
    client: &ApiClient<'_>,
    server_id: &str,
    mission_id: &str,
    artifact_id: &str,
    event_mission_id: Option<&str>,
) -> Result<Value> {
    let mut body = json!({ "mission_id": mission_id, "artifact_id": artifact_id });
    if let Some(event_mission_id) = event_mission_id {
        body["event_mission_id"] = json!(event_mission_id);
    }
    client.expect_json(
        "POST",
        &format!(
            "/api/v1/servers/{}/deployments",
            encode_component(server_id)
        ),
        Some(&body),
        202,
    )
}

/// `GET /api/v1/servers/{id}/deployments/{deployment}`.
pub fn deployment(client: &ApiClient<'_>, server_id: &str, deployment_id: &str) -> Result<Value> {
    client.expect_json(
        "GET",
        &format!(
            "/api/v1/servers/{}/deployments/{}",
            encode_component(server_id),
            encode_component(deployment_id)
        ),
        None,
        200,
    )
}

/// Poll the deployment until it settles or `patience` runs out.
pub fn wait_for_deployment(
    client: &ApiClient<'_>,
    server_id: &str,
    deployment_id: &str,
    patience: Duration,
    interval: Duration,
) -> Result<DeploymentSettlement> {
    let started = Instant::now();
    loop {
        let current = deployment(client, server_id, deployment_id)?;
        match current["state"].as_str().unwrap_or_default() {
            "confirmed" => {
                return Ok(DeploymentSettlement::Confirmed {
                    runtime_session_id: text_at(
                        &current,
                        "/confirmed_runtime_session_id",
                        "the confirmed deployment",
                    )?,
                });
            }
            "requested" => {}
            state => {
                return Ok(DeploymentSettlement::Failed {
                    state: state.to_string(),
                    reason: current["failure_reason"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                });
            }
        }
        if started.elapsed() >= patience {
            return Ok(DeploymentSettlement::StillRequested);
        }
        std::thread::sleep(interval);
    }
}

/// Cancel a fleet command no executor has claimed. `Ok(false)` when it is past cancelling (the
/// platform answers 409 with its state); any other answer is an error.
pub fn cancel_unclaimed_command(
    client: &ApiClient<'_>,
    server_id: &str,
    command_id: &str,
) -> Result<bool> {
    let path = format!(
        "/api/v1/servers/{}/commands/{}/cancel",
        encode_component(server_id),
        encode_component(command_id)
    );
    let answer = client
        .call("POST", &path, None)
        .context("cancel an unclaimed fleet command")?;
    match answer.status {
        200 => Ok(true),
        409 => Ok(false),
        status => bail!("POST {path} answered {status}: {}", answer.excerpt()),
    }
}
