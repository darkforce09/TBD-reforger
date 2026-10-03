//! The fleet receipt's observations: the servers the run reached, the largest player listing,
//! and the judge's scenarios named by the declared cases.
//!
//! **Role:** builds [`Observations::Fleet`] from a [`ProcedureRun`] and the fixture manifest,
//! and names the measurement keys the waves write for it.
//!
//! **Position:** called by `FleetProcedure::observations` for every recorded run, a partial or
//! stopped one included; `operational.rs` judges the result.
//!
//! **Signals & state:** none; pure functions over the run's cases and measurements.
//!
//! **Invariants:** a per-server scenario is named only when all five servers' cases of that
//! name are `ok`; `lost_acknowledgement` only when both lost-acknowledgement cases are `ok`;
//! `client_count` is the largest number of distinct Arma ids in one player listing, 0 when no
//! listing was recorded; `server_ids` lists the ids the run observed, in instance order;
//! `fixture_sha256` is the manifest's own digest.

use serde_json::Value;

use super::fleet_cases::{
    FLEET_SERVER_COUNT, IDENTITY_LINK, KICK, LOST_ACKNOWLEDGEMENT_CLAIM_RESPONSE,
    LOST_ACKNOWLEDGEMENT_RESULT_RESPONSE, per_server_case_name,
};
use crate::procedure_runner::procedure::ProcedureRun;
use crate::procedure_runner::step::Measurements;
use api_readiness_checks::operational_recording::{
    CaseStatus, FixtureManifest, Observations, RecordedCase,
};

/// The judge's scenarios every fleet server must pass, each named after its per-server case.
pub(crate) const PER_SERVER_SCENARIOS: [&str; 6] = [
    "stop",
    "start",
    "restart",
    "custom_console",
    "same_terrain",
    "cross_terrain",
];

/// The judge's fleet-wide scenarios and the cases each needs.
pub(crate) const FLEET_WIDE_SCENARIOS: [(&str, &[&str]); 3] = [
    ("kick", &[KICK]),
    ("identity_link", &[IDENTITY_LINK]),
    (
        "lost_acknowledgement",
        &[
            LOST_ACKNOWLEDGEMENT_CLAIM_RESPONSE,
            LOST_ACKNOWLEDGEMENT_RESULT_RESPONSE,
        ],
    ),
];

/// The prefix of every player-listing measurement.
const PLAYER_LISTING_PREFIX: &str = "player_listing.";

/// The measurement key of the server id the run observed for `instance`.
pub(crate) fn server_id_measurement(instance: u16) -> String {
    format!("fleet.server{instance}.id")
}

/// The measurement key of the distinct Arma ids one listing of `step_id` showed on `instance`.
pub(crate) fn player_listing_measurement(step_id: &str, instance: u16) -> String {
    format!("{PLAYER_LISTING_PREFIX}{step_id}.server{instance}")
}

/// The observations the fleet receipt carries, whatever its verdict.
pub(crate) fn fleet_observations(run: &ProcedureRun, manifest: &FixtureManifest) -> Observations {
    Observations::Fleet {
        server_ids: server_ids(&run.measurements),
        client_count: client_count(&run.measurements),
        scenarios: judge_scenarios(&run.cases),
        fixture_sha256: manifest.sha256().to_string(),
    }
}

/// The judge's scenarios the recorded cases establish.
pub(crate) fn judge_scenarios(cases: &[RecordedCase]) -> Vec<String> {
    let ok = |name: &str| {
        cases
            .iter()
            .any(|case| case.name.as_str() == name && case.status == CaseStatus::Ok)
    };
    let per_server = PER_SERVER_SCENARIOS.into_iter().filter(|scenario| {
        (1..=FLEET_SERVER_COUNT).all(|instance| ok(&per_server_case_name(instance, scenario)))
    });
    let fleet_wide = FLEET_WIDE_SCENARIOS
        .into_iter()
        .filter(|(_, needed)| needed.iter().all(|case| ok(case)))
        .map(|(scenario, _)| scenario);
    per_server.chain(fleet_wide).map(str::to_string).collect()
}

/// The largest number of distinct Arma ids one recorded player listing showed.
pub(crate) fn client_count(measurements: &Measurements) -> u64 {
    measurements
        .iter()
        .filter(|(key, _)| key.starts_with(PLAYER_LISTING_PREFIX))
        .filter_map(|(_, count)| count.as_u64())
        .max()
        .unwrap_or(0)
}

/// The server ids the run observed, in instance order.
pub(crate) fn server_ids(measurements: &Measurements) -> Vec<String> {
    (1..=FLEET_SERVER_COUNT)
        .filter_map(|instance| {
            measurements
                .get(&server_id_measurement(instance))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect()
}
