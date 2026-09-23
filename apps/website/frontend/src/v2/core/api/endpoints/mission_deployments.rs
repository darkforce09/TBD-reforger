//! The administrator routes of one server's mission deployments, and the reads a deployment request
//! chooses from.
//!
//! **Role:** the deployment collection, deployment and cancellation paths with one call per route,
//! plus the mission library and operation calendar reads the request form offers choices from.
//! **Position:** called by the server control screen's deployments panel.
//! **Signals & state:** none.
//! **Invariants:** a request answers 202 with the deployment it recorded; it is confirmed only by a
//! runtime session that reports the artifact, so the deployment is followed until it is confirmed,
//! failed or cancelled. A refused request persists nothing and names its reason in `details.code`
//! (`SERVER_INACTIVE`, `DEPLOYMENT_IN_PROGRESS`, `ARTIFACT_NOT_APPROVED`,
//! `EVENT_MISSION_NOT_ON_SERVER`, `MODPACK_MISMATCH`, `TERRAIN_NOT_RUNNABLE`,
//! `ORBAT_ARTIFACT_MISMATCH`); only a deployment in flight whose command is still queued can be
//! cancelled (`DEPLOYMENT_NOT_IN_FLIGHT`, `COMMAND_NOT_CANCELLABLE` otherwise). The choice reads ask
//! for the backend's largest page, a hundred rows.

use super::encode_path_segment as segment;

/// `GET` (the server's deployments, newest first) and `POST` (request one)
/// `/servers/:id/deployments`.
pub fn server_deployments_path(server_id: &str) -> String {
    format!("/servers/{}/deployments", segment(server_id))
}

/// `GET /servers/:id/deployments/:deploymentId`.
pub fn server_deployment_path(server_id: &str, deployment_id: &str) -> String {
    format!(
        "{}/{}",
        server_deployments_path(server_id),
        segment(deployment_id)
    )
}

/// `POST /servers/:id/deployments/:deploymentId/cancel`.
pub fn deployment_cancellation_path(server_id: &str, deployment_id: &str) -> String {
    format!(
        "{}/cancel",
        server_deployment_path(server_id, deployment_id)
    )
}

/// `GET /missions?limit=100`: the library as the viewer sees it — every live mission among it.
pub fn deployable_mission_choices_path() -> String {
    "/missions?limit=100".to_string()
}

/// `GET /events?scope=upcoming&limit=100`: the operations still to run, and the ones running now.
pub fn upcoming_operations_path() -> String {
    "/events?scope=upcoming&limit=100".to_string()
}

/// `GET /events/:id`: one operation with its missions.
pub fn operation_path(event_id: &str) -> String {
    format!("/events/{}", segment(event_id))
}

#[cfg(target_arch = "wasm32")]
pub use calls::*;

/// The browser-only calls, one per route.
#[cfg(target_arch = "wasm32")]
mod calls {
    use super::*;
    use crate::v2::core::api::client::{api_get, api_post_keeping_refusal, ApiErr, ApiRefusal};
    use crate::v2::core::api::dto::{
        DeploymentRequest, EventHub, EventListItem, MissionCard, MissionDeployment,
        MissionDeploymentPage, Paginated,
    };
    use crate::v2::core::api::endpoints::json_body;
    use crate::v2::core::auth::AuthStore;

    /// Request a deployment; the answer is the deployment recorded, not yet confirmed.
    pub async fn request_mission_deployment(
        store: AuthStore,
        server_id: &str,
        request: &DeploymentRequest,
    ) -> Result<MissionDeployment, ApiRefusal> {
        let path = server_deployments_path(server_id);
        api_post_keeping_refusal(store, &path, json_body(request)?).await
    }

    /// The server's deployments, newest first.
    pub async fn load_mission_deployments(
        store: AuthStore,
        server_id: &str,
    ) -> Result<MissionDeploymentPage, ApiErr> {
        api_get(store, &server_deployments_path(server_id)).await
    }

    /// One deployment as it stands now.
    pub async fn load_mission_deployment(
        store: AuthStore,
        server_id: &str,
        deployment_id: &str,
    ) -> Result<MissionDeployment, ApiErr> {
        api_get(store, &server_deployment_path(server_id, deployment_id)).await
    }

    /// Cancel a deployment whose command no executor has claimed.
    pub async fn cancel_mission_deployment(
        store: AuthStore,
        server_id: &str,
        deployment_id: &str,
    ) -> Result<MissionDeployment, ApiRefusal> {
        let path = deployment_cancellation_path(server_id, deployment_id);
        api_post_keeping_refusal(store, &path, serde_json::json!({})).await
    }

    /// The library rows a deployment may choose from.
    pub async fn load_deployable_mission_choices(
        store: AuthStore,
    ) -> Result<Paginated<MissionCard>, ApiErr> {
        api_get(store, &deployable_mission_choices_path()).await
    }

    /// The operations still to run.
    pub async fn load_upcoming_operations(
        store: AuthStore,
    ) -> Result<Paginated<EventListItem>, ApiErr> {
        api_get(store, &upcoming_operations_path()).await
    }

    /// One operation with its missions.
    pub async fn load_operation(store: AuthStore, event_id: &str) -> Result<EventHub, ApiErr> {
        api_get(store, &operation_path(event_id)).await
    }
}
