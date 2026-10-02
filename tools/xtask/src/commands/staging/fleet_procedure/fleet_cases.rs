//! The declared cases of the fleet check and how their names are built.
//!
//! **Role:** names the eight per-server cases of every fleet server (`server<N>_<suffix>`) and
//! the fleet-wide cases of W8–W14, in declaration order; the two cases that need a second game
//! client are declared `NOT RUN`.
//!
//! **Position:** read by `fleet_procedure/mod.rs` (the plan's declared cases), by `waves/` (the
//! case each effect decides) and by `judge_mapping.rs` (the cases that name a judge scenario).
//!
//! **Signals & state:** none; constants and pure functions.
//!
//! **Invariants:** every name matches `[a-z0-9_]+`; the per-server cases come first, server by
//! server in instance order, then the fleet-wide cases; no name is declared twice.

use anyhow::Result;

use crate::commands::staging::procedure_runner::step::DeclaredCase;
use crate::verifications::api_readiness::operational_recording::CaseName;

/// The number of fleet servers the procedure drives, one per staging instance.
pub(crate) const FLEET_SERVER_COUNT: u16 = 5;

/// The per-server case suffixes in declaration order; each server declares `server<N>_<suffix>`.
pub(crate) const PER_SERVER_CASES: [&str; 8] = [
    "stop",
    "start",
    "restart",
    "custom_console",
    "list_players",
    "same_terrain",
    "cross_terrain",
    "runtime_session_succession",
];

/// The fleet-wide case of W8: every server deployed back onto the origin terrain.
pub(crate) const RETURN_TO_ORIGIN_TERRAIN: &str = "return_to_origin_terrain";

/// W9: the operator's Arma identity linked through `#tbd link` on the first server.
pub(crate) const IDENTITY_LINK: &str = "identity_link";
/// W10: the operator kicked from the first server by a fleet command the game runtime ran.
pub(crate) const KICK: &str = "kick";
/// W10: a kick naming an ended runtime session refused with 409 `RUNTIME_SESSION_ENDED`.
pub(crate) const REJECTION_ENDED_SESSION_KICK: &str = "rejection_ended_session_kick";
/// W11: the host agent's machine credential rotated on the first server.
pub(crate) const MACHINE_CREDENTIAL_ROTATION_HOST_AGENT: &str =
    "machine_credential_rotation_host_agent";
/// W12: the game runtime's machine credential rotated on the second server.
pub(crate) const MACHINE_CREDENTIAL_ROTATION_MOD_RUNTIME: &str =
    "machine_credential_rotation_mod_runtime";
/// W13: a claim answer the relay withheld, recovered by a lease lapse and a second claim.
pub(crate) const LOST_ACKNOWLEDGEMENT_CLAIM_RESPONSE: &str = "lost_acknowledgement_claim_response";
/// W14: a result answer the relay withheld, whose retried report the ledger refused.
pub(crate) const LOST_ACKNOWLEDGEMENT_RESULT_RESPONSE: &str =
    "lost_acknowledgement_result_response";

/// The fleet-wide cases of W9–W14 in the order their waves run.
pub(crate) const SINGLE_SERVER_CASES: [&str; 7] = [
    IDENTITY_LINK,
    KICK,
    REJECTION_ENDED_SESSION_KICK,
    MACHINE_CREDENTIAL_ROTATION_HOST_AGENT,
    MACHINE_CREDENTIAL_ROTATION_MOD_RUNTIME,
    LOST_ACKNOWLEDGEMENT_CLAIM_RESPONSE,
    LOST_ACKNOWLEDGEMENT_RESULT_RESPONSE,
];

/// The dependency the staging fleet lacks for the two-client cases: the operator is its only
/// game client.
pub(crate) const SECOND_GAME_CLIENT: &str = "second game client";

/// The cases that need two game clients on one server, recorded `NOT RUN`.
pub(crate) const TWO_CLIENT_CASES: [&str; 2] = [
    "kick_targets_one_of_two_clients",
    "same_terrain_carries_two_clients",
];

/// `server<instance>_<suffix>`.
pub(crate) fn per_server_case_name(instance: u16, suffix: &str) -> String {
    format!("server{instance}_{suffix}")
}

/// `server<instance>_<suffix>` as a validated case name.
pub(crate) fn per_server_case(instance: u16, suffix: &str) -> Result<CaseName> {
    CaseName::new(&per_server_case_name(instance, suffix))
}

/// Every declared case: the forty per-server cases, then the two two-client cases recorded
/// `NOT RUN`, then the return to the origin terrain and the seven cases of W9–W14.
pub(crate) fn declared_cases() -> Result<Vec<DeclaredCase>> {
    let mut cases = Vec::new();
    for instance in 1..=FLEET_SERVER_COUNT {
        for suffix in PER_SERVER_CASES {
            cases.push(DeclaredCase::runs(&per_server_case_name(instance, suffix))?);
        }
    }
    for name in TWO_CLIENT_CASES {
        cases.push(DeclaredCase::not_run(name, SECOND_GAME_CLIENT)?);
    }
    cases.push(DeclaredCase::runs(RETURN_TO_ORIGIN_TERRAIN)?);
    for name in SINGLE_SERVER_CASES {
        cases.push(DeclaredCase::runs(name)?);
    }
    Ok(cases)
}
