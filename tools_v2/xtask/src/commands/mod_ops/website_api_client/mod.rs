//! A small client for the website API, used by the mod tooling (`mod playtest`, `mod world-boot`,
//! `mod test-mission`, `mod test-game-runtime-api`) to drive the platform the way an
//! administrator does in development: log in, publish a mission (submit, approve, read the
//! artifact document), provision the fleet (scenario, server, machine credentials) and deploy.
//! Requests go through `curl` like the rest of the tooling; the transport is a trait, so the
//! flows are tested without a network.

mod api_client;
mod development_login;
mod fleet_provisioning;
mod http_exchange;
mod mission_artifact_cache;
mod mission_publication;

pub use api_client::ApiClient;
pub use development_login::development_login;
pub use fleet_provisioning::{
    DeploymentSettlement, cancel_unclaimed_command, ensure_fleet_scenario, ensure_server,
    issue_credential, request_deployment, revoke_credential, wait_for_deployment,
};
pub use http_exchange::{ApiAnswer, ApiTransport, CurlTransport, encode_component};
pub use mission_artifact_cache::{
    ARTIFACT_CACHE_DIRECTORY, StagedArtifact, clear_artifact_cache, stage_artifact_cache,
};
pub use mission_publication::{
    approved_artifact, artifact_document, create_mission, delete_mission, mission,
    own_missions_titled, pending_review_artifact, sha256_hex, submit_mission,
};

#[cfg(test)]
#[path = "tests/flows.rs"]
mod flow_tests;

#[cfg(test)]
#[path = "tests/transport_parsing.rs"]
mod transport_parsing_tests;
