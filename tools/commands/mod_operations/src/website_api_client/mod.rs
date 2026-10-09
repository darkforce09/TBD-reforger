//! A small client for the website API, used by the mod tooling to drive the platform the way an
//! administrator does in development.
//!
//! **Role:** log in, publish a mission (create, submit, approve, read the artifact document),
//! provision the fleet (scenario, server, machine credentials), deploy, stage or clear the mod's
//! cached artifact, and read back the telemetry the platform recorded for a server and its match.
//! **Position:** used by `mod playtest`, `mod world-boot`, `mod test-mission` and
//! `mod test-game-runtime-api`; requests go through `curl` ([`CurlTransport`]) behind the
//! [`ApiTransport`] trait, so the flows are tested without a network.
//! **Signals & state:** none held; each call is one request.
//! **Invariants:** a call names the status it expects and any other status is an error carrying
//! the platform's refusal; only no answer at all is a transport error.

mod api_client;
mod development_login;
mod fleet_provisioning;
mod http_exchange;
mod mission_artifact_cache;
mod mission_publication;
mod runtime_telemetry_reads;

pub(crate) use api_client::ApiClient;
pub(crate) use development_login::development_login;
pub(crate) use fleet_provisioning::{
    DeploymentSettlement, cancel_unclaimed_command, ensure_fleet_scenario, ensure_server,
    issue_credential, request_deployment, revoke_credential, wait_for_deployment,
};
pub(crate) use http_exchange::{ApiAnswer, ApiTransport, CurlTransport, encode_component};
pub(crate) use mission_artifact_cache::{
    ARTIFACT_CACHE_DIRECTORY, StagedArtifact, clear_artifact_cache, stage_artifact_cache,
};
pub(crate) use mission_publication::{
    approved_artifact, artifact_document, create_mission, delete_mission, mission,
    own_missions_titled, pending_review_artifact, submit_mission,
};
pub(crate) use runtime_telemetry_reads::{
    ServerTelemetryStatus, TelemetryQueueReading, match_has_acknowledged_events,
    server_telemetry_status,
};
