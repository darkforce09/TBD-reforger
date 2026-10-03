//! The Rust type generated from each contract schema, as a decoder.
//!
//! **Role:** maps a [`SchemaRef`] to a decoder that runs `serde_json::from_value` into the type
//! `cargo xtask ci schema-codegen` generates from that schema, for every schema a route answers
//! with that has a generated type.
//!
//! **Position:** consumed by the decoding case of `tests/contract_parity_goldens.rs`, which applies
//! the decoder to every part [`super::route_contracts`] splits a golden into.
//!
//! **Signals & state:** none; a pure lookup.
//!
//! **Invariants:** a schema maps to at most one type; `None` means the schema is not a codegen
//! target, never that a decode was skipped for a target that has one. The registry pages' rows
//! (`arsenal-envelopes.schema.json` `RegistryItemRow` and `RegistryCompatRow`) are such schemas:
//! they carry storage columns the generated `registry_items::Item` and `registry_compat::Edge`
//! catalogue types refuse, so no route decodes into those types.

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::route_contracts::{SchemaLocation, SchemaRef};
use contract_schema_types::administration::{audit_log, personnel_roster};
use contract_schema_types::community_content::equipment_data_viewer as equipment;
use contract_schema_types::community_content::{content_upload, vehicle_database, wiki_page};
use contract_schema_types::identity_and_access::current_profile;
use contract_schema_types::match_telemetry::match_telemetry;
use contract_schema_types::missions::faction_library;
use contract_schema_types::missions::{mission_deployment, mission_review};
use contract_schema_types::operations::{
    event_access_administration as event_access, event_hub, event_orbat, game_runtime_deployment,
    game_runtime_roster, reservation_response, waitlist_promotion_response,
};
use contract_schema_types::server_infrastructure::{
    fleet_command, game_runtime_session, machine_credential,
};

/// Decodes a value into one generated type; the error is serde's.
pub type Decoder = fn(&Value) -> Result<(), String>;

fn decode<T: DeserializeOwned>(value: &Value) -> Result<(), String> {
    serde_json::from_value::<T>(value.clone())
        .map(drop)
        .map_err(|error| error.to_string())
}

/// The decoder for the type generated from `schema`, when the schema is a codegen target.
pub fn decoder_for(schema: SchemaRef) -> Option<Decoder> {
    use SchemaLocation::{Definition, Root};
    let decoder: Decoder = match (schema.file, schema.location) {
        ("current-profile.schema.json", Root) => decode::<current_profile::CurrentProfileResponse>,
        ("audit-log.schema.json", Definition("AuditLogPage")) => decode::<audit_log::AuditLogPage>,
        ("audit-log.schema.json", Definition("AuditLogEntry")) => {
            decode::<audit_log::AuditLogEntry>
        }
        ("audit-log.schema.json", Definition("AuditStreamReady")) => {
            decode::<audit_log::AuditStreamReady>
        }
        ("audit-log.schema.json", Definition("AuditStreamReset")) => {
            decode::<audit_log::AuditStreamReset>
        }
        ("personnel-roster.schema.json", Definition("PersonnelPage")) => {
            decode::<personnel_roster::PersonnelPage>
        }
        ("content-upload.schema.json", Definition("UploadResponse")) => {
            decode::<content_upload::UploadResponse>
        }
        ("vehicle-database.schema.json", Definition("VehicleList")) => {
            decode::<vehicle_database::VehicleList>
        }
        ("vehicle-database.schema.json", Definition("Vehicle")) => {
            decode::<vehicle_database::Vehicle>
        }
        ("wiki-page.schema.json", Definition("WikiPageList")) => decode::<wiki_page::WikiPageList>,
        ("wiki-page.schema.json", Definition("WikiArticle")) => decode::<wiki_page::WikiArticle>,
        ("wiki-page.schema.json", Definition("WikiRevisionPage")) => {
            decode::<wiki_page::WikiRevisionPage>
        }
        ("wiki-page.schema.json", Definition("WikiRevision")) => decode::<wiki_page::WikiRevision>,
        ("equipment-data-viewer/dataset.schema.json", Root) => {
            decode::<equipment::dataset::EquipmentDatasetStatus>
        }
        ("equipment-data-viewer/resources.schema.json", Root) => {
            decode::<equipment::resources::EquipmentResourcePage>
        }
        ("equipment-data-viewer/relationships.schema.json", Root) => {
            decode::<equipment::relationships::EquipmentRelationshipPage>
        }
        ("equipment-data-viewer/field-inventory.schema.json", Root) => {
            decode::<equipment::field_inventory::EquipmentFieldPage>
        }
        ("equipment-data-viewer/resource-cards.schema.json", Root) => {
            decode::<equipment::resource_cards::EquipmentResourceCardPage>
        }
        ("equipment-data-viewer/source-inspection.schema.json", Root) => {
            decode::<equipment::source_inspection::EquipmentSourcePage>
        }
        ("match-telemetry.schema.json", Root) => decode::<match_telemetry::MatchEventPage>,
        ("match-telemetry.schema.json", Definition("MatchRegistrationAnswer")) => {
            decode::<match_telemetry::MatchRegistrationAnswer>
        }
        ("match-telemetry.schema.json", Definition("MatchResultsAnswer")) => {
            decode::<match_telemetry::MatchResultsAnswer>
        }
        ("match-telemetry.schema.json", Definition("MatchEventBatchAnswer")) => {
            decode::<match_telemetry::MatchEventBatchAnswer>
        }
        ("mission-review.schema.json", Root) => decode::<mission_review::MissionReviewHistory>,
        ("mission-review.schema.json", Definition("MissionRow")) => {
            decode::<mission_review::MissionRow>
        }
        ("mission-review.schema.json", Definition("MissionVersion")) => {
            decode::<mission_review::MissionVersion>
        }
        ("mission-review.schema.json", Definition("ReviewComment")) => {
            decode::<mission_review::ReviewComment>
        }
        ("mission-review.schema.json", Definition("MissionArtifact")) => {
            decode::<mission_review::MissionArtifact>
        }
        ("mission-review.schema.json", Definition("ReviewWorkspace")) => {
            decode::<mission_review::ReviewWorkspace>
        }
        ("mission-review.schema.json", Definition("ApprovalQueuePage")) => {
            decode::<mission_review::ApprovalQueuePage>
        }
        ("mission-deployment.schema.json", Definition("RuntimeDeployment")) => {
            decode::<mission_deployment::RuntimeDeployment>
        }
        ("mission-deployment.schema.json", Definition("DeployableMissionList")) => {
            decode::<mission_deployment::DeployableMissionList>
        }
        ("mission-deployment.schema.json", Definition("MissionDeployment")) => {
            decode::<mission_deployment::MissionDeployment>
        }
        ("mission-deployment.schema.json", Definition("MissionDeploymentPage")) => {
            decode::<mission_deployment::MissionDeploymentPage>
        }
        ("mission-deployment.schema.json", Definition("FleetScenarioList")) => {
            decode::<mission_deployment::FleetScenarioList>
        }
        ("mission-deployment.schema.json", Definition("FleetScenario")) => {
            decode::<mission_deployment::FleetScenario>
        }
        ("event-hub.schema.json", Root) => decode::<event_hub::EventHub>,
        ("event-access-administration.schema.json", Definition("EventAccessAdministration")) => {
            decode::<event_access::EventAccessAdministration>
        }
        ("event-access-administration.schema.json", Definition("ParticipantAccessExplanation")) => {
            decode::<event_access::ParticipantAccessExplanation>
        }
        ("event-access-administration.schema.json", Definition("AccessChangeOutcome")) => {
            decode::<event_access::AccessChangeOutcome>
        }
        ("event-orbat.schema.json", Root) => decode::<event_orbat::EventMissionOrbat>,
        ("reservation-response.schema.json", Root) => {
            decode::<reservation_response::ReservationResponse>
        }
        ("waitlist-promotion-response.schema.json", Root) => {
            decode::<waitlist_promotion_response::WaitlistPromotionResponse>
        }
        ("game-runtime-roster.schema.json", Root) => decode::<game_runtime_roster::EventRoster>,
        ("game-runtime-deployment.schema.json", Root) => {
            decode::<game_runtime_deployment::DeploymentDecision>
        }
        ("game-runtime-deployment.schema.json", Definition("EndedLife")) => {
            decode::<game_runtime_deployment::EndedLife>
        }
        ("fleet-command.schema.json", Definition("FleetCommandReceipt")) => {
            decode::<fleet_command::FleetCommandReceipt>
        }
        ("fleet-command.schema.json", Definition("FleetCommandList")) => {
            decode::<fleet_command::FleetCommandList>
        }
        ("fleet-command.schema.json", Definition("ClaimedFleetCommand")) => {
            decode::<fleet_command::ClaimedFleetCommand>
        }
        ("machine-credential.schema.json", Definition("MachineCredentialList")) => {
            decode::<machine_credential::MachineCredentialList>
        }
        ("machine-credential.schema.json", Root) => {
            decode::<machine_credential::IssuedMachineCredential>
        }
        ("machine-credential.schema.json", Definition("MachineCredential")) => {
            decode::<machine_credential::MachineCredential>
        }
        ("game-runtime-session.schema.json", Root) => {
            decode::<game_runtime_session::StartedRuntimeSession>
        }
        ("game-runtime-session.schema.json", Definition("RuntimeSessionEnd")) => {
            decode::<game_runtime_session::RuntimeSessionEnd>
        }
        ("faction-library.schema.json", Root) => decode::<faction_library::TbdFactionLibraryEntry>,
        _ => return None,
    };
    Some(decoder)
}
