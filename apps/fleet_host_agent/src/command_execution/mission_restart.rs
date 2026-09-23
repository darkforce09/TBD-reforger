//! `restart_with_mission`: point the dedicated server's config at the deployment's scenario,
//! then restart the unit exactly as `restart` does, with the same dwell and the same read-back
//! verdict (loaded and active).
//!
//! The config is switched first. When it cannot be read, parsed or replaced, the command fails
//! with the file unchanged and nothing restarted. When the restart fails after the switch, the
//! command fails as well, and its reason and outcome say that the config already names the new
//! scenario, which the next start of the unit runs. The agent never fetches the artifact: the
//! game runtime reads its deployment from the API when it boots, loads the artifact and reports
//! it, and that report, not this command, confirms the deployment.

use serde_json::{Map, Value};
use tracing::info;

use super::mission_deployment::MissionDeployment;
use crate::action_verdict::ActionVerdict;
use crate::dedicated_server_config::DedicatedServerConfig;
use crate::process_control::{ProcessAction, ProcessControl};

pub(super) async fn restart_with_mission(
    server_config: &DedicatedServerConfig,
    process_control: &ProcessControl,
    deployment: &MissionDeployment,
) -> ActionVerdict {
    let scenario_id = deployment.scenario_id.as_str();
    let config_path = server_config.path().display().to_string();
    info!(
        deployment_id = %deployment.deployment_id,
        artifact_id = %deployment.artifact_id,
        artifact_sha256 = deployment.artifact_sha256,
        scenario_id,
        config_path,
        "switching the server config to the deployment's scenario"
    );
    let mut outcome = Map::new();
    outcome.insert("scenario_id".to_owned(), Value::from(scenario_id));
    outcome.insert("config_path".to_owned(), Value::from(config_path.clone()));
    // Reading, flushing and renaming block, so they run off the async workers.
    let switch = {
        let server_config = server_config.clone();
        let scenario_id = scenario_id.to_owned();
        tokio::task::spawn_blocking(move || server_config.switch_scenario(&scenario_id)).await
    };
    match switch {
        Ok(Ok(())) => {}
        Ok(Err(problem)) => {
            outcome.insert("config_updated".to_owned(), Value::from(false));
            return ActionVerdict::failure(
                &format!("the server config {config_path} {problem}; the unit was not restarted"),
                Some(outcome),
            );
        }
        Err(interrupted) => {
            return ActionVerdict::failure(
                &format!(
                    "the switch of the server config {config_path} did not finish ({interrupted}); \
                     the file names either scenario and the unit was not restarted"
                ),
                Some(outcome),
            );
        }
    }
    info!(
        scenario_id,
        config_path, "the server config names the deployment's scenario"
    );
    let report = process_control.perform(ProcessAction::Restart).await;
    let restart = report.verdict();
    let observed = report.unit_state.as_ref().ok();
    outcome.insert(
        "unit_active_state".to_owned(),
        observed.map_or(Value::Null, |state| Value::from(state.active_state.clone())),
    );
    if restart.succeeded() {
        return ActionVerdict::success(outcome);
    }
    outcome.insert("config_updated".to_owned(), Value::from(true));
    outcome.insert(
        "unit_load_state".to_owned(),
        observed.map_or(Value::Null, |state| Value::from(state.load_state.clone())),
    );
    outcome.insert(
        "systemctl".to_owned(),
        Value::from(report.systemctl.describe()),
    );
    ActionVerdict::failure(
        &format!(
            "the server config {config_path} now names {scenario_id}, but the restart failed: {}",
            restart.failure_reason().unwrap_or_default()
        ),
        Some(outcome),
    )
}
