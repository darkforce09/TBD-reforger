//! The normalisation table: every golden field whose value the server generates per request.
//!
//! **Role:** names, by the request that answers it and a JSON pointer into the answer, each id,
//! secret and request-time instant a write golden cannot pin, with the kind that fixes its format
//! check and its placeholder ([`super::golden_normalisation::NormalisedKind`]).
//!
//! **Position:** read by [`super::golden_normalisation`].
//!
//! **Signals & state:** none; one constant.
//!
//! **Invariants:** a row's method and path are spelled exactly as the golden's `_index.tsv` row
//! spells them (query included); every row names a field its golden holds as the kind's
//! placeholder, and the index case of `tests/contract_parity/goldens.rs` fails on any row that
//! does not (no dead rows). A field absent from this table compares byte for byte.

use super::golden_normalisation::NormalisedKind::{
    self, AccessTokenJwt, LinkCodeSixDigits, MachineCredentialSecret, OpaqueTokenHex64,
    RequestTime, RequestTimePlusSeconds, ServerUuid,
};

/// One normalised field of one indexed golden.
#[derive(Debug)]
pub(crate) struct NormalisedField {
    /// The request method, as the golden file name's prefix spells it.
    pub method: &'static str,
    /// The request path, as the golden's `_index.tsv` row spells it.
    pub path: &'static str,
    /// The RFC 6901 pointer of the field inside the answer.
    pub pointer: &'static str,
    /// What the field holds.
    pub kind: NormalisedKind,
}

const fn field(
    method: &'static str,
    path: &'static str,
    pointer: &'static str,
    kind: NormalisedKind,
) -> NormalisedField {
    NormalisedField {
        method,
        path,
        pointer,
        kind,
    }
}

/// The access token's lifetime under the recipe and test configuration (15 minutes).
const ACCESS_TOKEN_SECONDS: i64 = 15 * 60;
/// An Arma link code's lifetime (10 minutes).
const LINK_CODE_SECONDS: i64 = 10 * 60;
/// How long a mission deployment may take before it is settled as failed (20 minutes).
const DEPLOYMENT_DEADLINE_SECONDS: i64 = 20 * 60;
/// How long an operator's fleet command stays queued (5 minutes).
const QUEUED_COMMAND_SECONDS: i64 = 5 * 60;

const REFRESH: &str = "/api/v1/auth/refresh";
const LINK_CODE: &str = "/api/v1/me/link";
const MODPACKS: &str = "/api/v1/modpacks";
const VEHICLES: &str = "/api/v1/vehicle-database";
const WIKI_PAGE: &str = "/api/v1/wiki/night-operations";
const REJECT: &str = "/api/v1/approvals/00000000-0000-4000-c000-000000000004/reject";
const SUBMIT: &str = "/api/v1/missions/00000000-0000-4000-c000-000000000004/submit";
const APPROVE: &str = "/api/v1/approvals/00000000-0000-4000-c000-000000000004/approve";
const REVIEW_COMMENT: &str =
    "/api/v1/missions/00000000-0000-4000-c000-000000000004/review-comments";
const DEPLOYMENTS: &str = "/api/v1/servers/00000000-0000-4000-d000-000000000001/deployments";
const DEPLOYMENT_CANCEL: &str = "/api/v1/servers/00000000-0000-4000-d000-000000000002\
    /deployments/00000000-0000-4000-f400-000000000003/cancel";
const FACTIONS: &str = "/api/v1/factions";
const FACTION: &str = "/api/v1/factions/00000000-0000-4000-b100-000000000003";
const FIRE_MISSIONS: &str = "/api/v1/fire-missions";
const BALLISTICS_CATALOGS: &str = "/api/v1/ballistics-catalogs";
const EVENT_GROUPS: &str = "/api/v1/events/c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7/groups";
const LEAVE_REQUESTS: &str = "/api/v1/me/leave-requests";
const COMMANDS: &str = "/api/v1/servers/00000000-0000-4000-d000-000000000001/commands";
const COMMAND_CANCEL: &str = "/api/v1/servers/00000000-0000-4000-d000-000000000001\
    /commands/00000000-0000-4000-f200-000000000003/cancel";
const CREDENTIALS: &str = "/api/v1/servers/00000000-0000-4000-d000-000000000001/credentials";
const CREDENTIAL_REVOKE: &str = "/api/v1/servers/00000000-0000-4000-d000-000000000002\
    /credentials/00000000-0000-4000-e000-000000000004?reason=Host%20retired";
const FLEET_SCENARIO: &str = "/api/v1/fleet/scenarios/everon";
const SERVERS: &str = "/api/v1/servers";

/// Every normalised field, grouped by golden in index order.
pub(crate) const NORMALISED_FIELDS: &[NormalisedField] = &[
    field(
        "GET",
        BALLISTICS_CATALOGS,
        "/data/0/uploaded_at",
        RequestTime,
    ),
    field("POST", REJECT, "/reviewed_at", RequestTime),
    field("POST", REJECT, "/updated_at", RequestTime),
    field("POST", SUBMIT, "/updated_at", RequestTime),
    field("POST", APPROVE, "/reviewed_at", RequestTime),
    field("POST", APPROVE, "/updated_at", RequestTime),
    field("POST", REVIEW_COMMENT, "/id", ServerUuid),
    field("POST", REVIEW_COMMENT, "/created_at", RequestTime),
    field("POST", DEPLOYMENTS, "/id", ServerUuid),
    field("POST", DEPLOYMENTS, "/fleet_command_id", ServerUuid),
    field("POST", DEPLOYMENTS, "/requested_at", RequestTime),
    field(
        "POST",
        DEPLOYMENTS,
        "/deadline_at",
        RequestTimePlusSeconds(DEPLOYMENT_DEADLINE_SECONDS),
    ),
    field("POST", DEPLOYMENT_CANCEL, "/finished_at", RequestTime),
    field("POST", REFRESH, "/access_token", AccessTokenJwt),
    field(
        "POST",
        REFRESH,
        "/expires_at",
        RequestTimePlusSeconds(ACCESS_TOKEN_SECONDS),
    ),
    field("POST", REFRESH, "/refresh_token", OpaqueTokenHex64),
    field("POST", LINK_CODE, "/code", LinkCodeSixDigits),
    field(
        "POST",
        LINK_CODE,
        "/expires_at",
        RequestTimePlusSeconds(LINK_CODE_SECONDS),
    ),
    field("POST", MODPACKS, "/id", ServerUuid),
    field("POST", MODPACKS, "/created_at", RequestTime),
    field("POST", MODPACKS, "/mods/0/id", ServerUuid),
    field("POST", MODPACKS, "/mods/0/modpack_id", ServerUuid),
    field("POST", VEHICLES, "/id", ServerUuid),
    field("PUT", WIKI_PAGE, "/id", ServerUuid),
    field("PUT", WIKI_PAGE, "/updated_at", RequestTime),
    field("POST", FACTIONS, "/id", ServerUuid),
    field("POST", FACTIONS, "/created_at", RequestTime),
    field("POST", FACTIONS, "/updated_at", RequestTime),
    field("PUT", FACTION, "/updated_at", RequestTime),
    field("POST", FIRE_MISSIONS, "/fire_mission/id", ServerUuid),
    field(
        "POST",
        FIRE_MISSIONS,
        "/fire_mission/created_at",
        RequestTime,
    ),
    field("POST", EVENT_GROUPS, "/access/groups/1/id", ServerUuid),
    field(
        "POST",
        EVENT_GROUPS,
        "/access/groups/1/provenance/created_at",
        RequestTime,
    ),
    field("POST", LEAVE_REQUESTS, "/id", ServerUuid),
    field("POST", LEAVE_REQUESTS, "/created_at", RequestTime),
    field("POST", COMMANDS, "/id", ServerUuid),
    field("POST", COMMANDS, "/requested_at", RequestTime),
    field(
        "POST",
        COMMANDS,
        "/expires_at",
        RequestTimePlusSeconds(QUEUED_COMMAND_SECONDS),
    ),
    field("POST", COMMAND_CANCEL, "/finished_at", RequestTime),
    field("POST", CREDENTIALS, "/credential/id", ServerUuid),
    field("POST", CREDENTIALS, "/credential/created_at", RequestTime),
    field("POST", CREDENTIALS, "/secret", MachineCredentialSecret),
    field("DELETE", CREDENTIAL_REVOKE, "/revoked_at", RequestTime),
    field("PUT", FLEET_SCENARIO, "/updated_at", RequestTime),
    field("POST", SERVERS, "/id", ServerUuid),
];
