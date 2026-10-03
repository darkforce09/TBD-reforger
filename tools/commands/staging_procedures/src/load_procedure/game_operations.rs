//! The game operations measured beside the member load: the API's own counters for the
//! game-runtime, fleet-executor and ingest routes, and the once-a-minute heartbeat census.
//!
//! **Role:** parses the `/metrics` exposition into per-family request counts by status and
//! cumulative latency buckets ([`GameOperationSample`]), turns two samples into a delta with a
//! p95 bound taken from the buckets ([`GameOperationDelta`]), and judges the heartbeat census
//! ([`HeartbeatCensus`]) the run takes while the load runs.
//!
//! **Position:** the judges of the plan's `game_operations_baseline`, `game_operations_delta`
//! and `member_load` steps; the census samples come from `load_run.rs`.
//!
//! **Signals & state:** none; pure parsers and judges over values the run measured.
//!
//! **Invariants:** a family's p95 bound is the smallest bucket bound under which at least 95 %
//! of the family's requests since the baseline finished, or unbounded when only `+Inf` holds
//! them; a counter that went down (an API restart) counts as zero, never negative; the census
//! holds only when every sample saw each fleet server heartbeat within the session expiry.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::load_queries::heartbeat_census;
use crate::procedure_runner::step::{ProbeVerdict, StepContext};

/// The route families of the game operations, by the route fragment that marks them.
pub(crate) const GAME_ROUTE_FAMILIES: [(&str, &str); 3] = [
    ("game_runtime", "/game-runtime/"),
    ("fleet_executor", "/fleet-executor/"),
    ("ingest", "/ingest/"),
];
/// The measurement holding the baseline sample.
pub(crate) const BASELINE_MEASUREMENT: &str = "game_operations_before";
/// The measurement holding the census.
pub(crate) const CENSUS_MEASUREMENT: &str = "heartbeat_census";
/// A heartbeat older than this, by the database's clock, no longer keeps a session alive (the
/// API's session expiry).
pub(crate) const HEARTBEAT_FRESHNESS_MS: u64 = 60_000;

/// Request counts and latency buckets of the game route families at one moment.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct GameOperationSample {
    /// Family → status → requests.
    pub requests: BTreeMap<String, BTreeMap<String, u64>>,
    /// Family → bucket bound (`le`) → requests finished within it, summed over the routes.
    pub latency_buckets: BTreeMap<String, BTreeMap<String, u64>>,
}

/// One family's requests between two samples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct FamilyDelta {
    pub family: String,
    pub requests: u64,
    pub by_status: BTreeMap<String, u64>,
    pub server_errors: u64,
    /// Seconds under which 95 % of the requests finished; `None` when unbounded or empty.
    pub p95_bound_seconds: Option<f64>,
}

/// The game operations between the baseline and the end of the load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct GameOperationDelta {
    pub families: Vec<FamilyDelta>,
}

/// Parses the exposition's `tbd_http_requests_total` and latency bucket series of the game
/// route families.
pub(crate) fn sample(exposition: &str) -> GameOperationSample {
    let mut sample = GameOperationSample::default();
    for line in exposition.lines().filter(|line| !line.starts_with('#')) {
        let Some((series, value)) = line.rsplit_once(' ') else {
            continue;
        };
        let Ok(value) = value.trim().parse::<f64>() else {
            continue;
        };
        let Some((metric, labels)) = series.split_once('{') else {
            continue;
        };
        let Some(family) = label(labels, "route").and_then(family_of) else {
            continue;
        };
        let count = value.max(0.0) as u64;
        let (table, key) = match metric {
            "tbd_http_requests_total" => (&mut sample.requests, label(labels, "status")),
            "tbd_http_request_duration_seconds_bucket" => {
                (&mut sample.latency_buckets, label(labels, "le"))
            }
            _ => continue,
        };
        if let Some(key) = key {
            *table
                .entry(family.to_string())
                .or_default()
                .entry(key.to_string())
                .or_default() += count;
        }
    }
    sample
}

/// Whether the exposition carries the request counter at all.
pub(crate) fn exposes_request_counter(exposition: &str) -> bool {
    exposition
        .lines()
        .any(|line| line.starts_with("tbd_http_requests_total"))
}

/// The requests of each family from `before` to `after`.
pub(crate) fn delta(
    before: &GameOperationSample,
    after: &GameOperationSample,
) -> GameOperationDelta {
    let difference = |table: &BTreeMap<String, BTreeMap<String, u64>>,
                      earlier: &BTreeMap<String, BTreeMap<String, u64>>,
                      family: &str| {
        let empty = BTreeMap::new();
        let old = earlier.get(family).unwrap_or(&empty);
        table
            .get(family)
            .unwrap_or(&empty)
            .iter()
            .map(|(key, count)| {
                (
                    key.clone(),
                    count.saturating_sub(old.get(key).copied().unwrap_or(0)),
                )
            })
            .filter(|(_, count)| *count > 0)
            .collect::<BTreeMap<String, u64>>()
    };
    let families = GAME_ROUTE_FAMILIES
        .iter()
        .map(|(family, _)| {
            let by_status = difference(&after.requests, &before.requests, family);
            let buckets = difference(&after.latency_buckets, &before.latency_buckets, family);
            FamilyDelta {
                family: family.to_string(),
                requests: by_status.values().sum(),
                server_errors: by_status
                    .iter()
                    .filter(|(status, _)| status.starts_with('5'))
                    .map(|(_, count)| count)
                    .sum(),
                by_status,
                p95_bound_seconds: p95_bound(&buckets),
            }
        })
        .collect();
    GameOperationDelta { families }
}

/// The smallest finite bucket bound holding 95 % of the `+Inf` bucket's requests.
pub(crate) fn p95_bound(buckets: &BTreeMap<String, u64>) -> Option<f64> {
    let total = *buckets.get("+Inf")?;
    let needed = (total * 95).div_ceil(100);
    let mut bounds: Vec<(f64, u64)> = buckets
        .iter()
        .filter_map(|(bound, count)| Some((bound.parse::<f64>().ok()?, *count)))
        .filter(|(bound, _)| bound.is_finite())
        .collect();
    bounds.sort_by(|a, b| a.0.total_cmp(&b.0));
    bounds
        .into_iter()
        .find(|(_, count)| total > 0 && *count >= needed)
        .map(|(bound, _)| bound)
}

/// The baseline judge: any exposition carrying the request counter is the baseline.
pub(crate) fn judge_baseline(exposition: &str, _: &StepContext<'_>) -> ProbeVerdict {
    if !exposes_request_counter(exposition) {
        return ProbeVerdict::Pending("the exposition holds no tbd_http_requests_total".into());
    }
    let baseline = sample(exposition);
    let summary = format!("baseline of {}", describe_totals(&baseline));
    match serde_json::to_value(&baseline) {
        Ok(value) => ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(summary).measure(BASELINE_MEASUREMENT, value),
        ),
        Err(error) => ProbeVerdict::Contradicted(format!("the baseline does not encode: {error}")),
    }
}

/// The delta judge: game-runtime requests arrived since the baseline and no game route
/// answered 5xx.
pub(crate) fn judge_delta(exposition: &str, context: &StepContext<'_>) -> ProbeVerdict {
    let Some(before) = context
        .measurements
        .get(BASELINE_MEASUREMENT)
        .and_then(|value| serde_json::from_value::<GameOperationSample>(value.clone()).ok())
    else {
        return ProbeVerdict::Contradicted("no baseline sample was measured".into());
    };
    let change = delta(&before, &sample(exposition));
    let summary = change
        .families
        .iter()
        .map(|family| {
            let bound = family
                .p95_bound_seconds
                .map_or_else(|| "unbounded".to_string(), |bound| format!("<= {bound} s"));
            format!(
                "{} {} requests {:?} p95 {bound}",
                family.family, family.requests, family.by_status
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    if change
        .families
        .iter()
        .any(|family| family.server_errors > 0)
    {
        return ProbeVerdict::Contradicted(format!("a game route answered 5xx: {summary}"));
    }
    if change
        .families
        .first()
        .is_none_or(|family| family.requests == 0)
    {
        return ProbeVerdict::Pending(format!(
            "no game-runtime request since the baseline: {summary}"
        ));
    }
    match serde_json::to_value(&change) {
        Ok(value) => ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(summary).measure("game_operations", value),
        ),
        Err(error) => ProbeVerdict::Contradicted(format!("the delta does not encode: {error}")),
    }
}

/// One server of one census sample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CensusServer {
    pub(crate) server_name: String,
    pub(crate) server_id: String,
    pub(crate) generation: i64,
    pub(crate) heartbeat_age_ms: u64,
}

/// One read of the heartbeat census.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CensusSample {
    pub observed_unix_ms: u64,
    /// The archived read's digest; absent when the read failed.
    pub artifact_sha256: Option<String>,
    pub servers: Vec<CensusServer>,
    /// Why the read failed, when it did.
    pub error: Option<String>,
}

impl CensusSample {
    /// The sample of one census read's output.
    pub(crate) fn of_output(observed_unix_ms: u64, sha256: String, output: &str) -> Self {
        Self {
            observed_unix_ms,
            artifact_sha256: Some(sha256),
            servers: heartbeat_census(output)
                .iter()
                .map(|row| CensusServer {
                    server_name: row.server_name.clone(),
                    server_id: row.server_id.clone(),
                    generation: row.generation,
                    heartbeat_age_ms: row.heartbeat_age_ms(),
                })
                .collect(),
            error: None,
        }
    }
}

/// Every census sample of the run, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HeartbeatCensus {
    /// Fleet servers every sample must see heartbeating.
    pub expected_servers: u16,
    pub samples: Vec<CensusSample>,
}

/// The census judge: at least two samples, each seeing every fleet server heartbeat within
/// [`HEARTBEAT_FRESHNESS_MS`].
pub(crate) fn judge_census(text: &str, _: &StepContext<'_>) -> ProbeVerdict {
    let census: HeartbeatCensus = match serde_json::from_str(text) {
        Ok(census) => census,
        Err(error) => {
            return ProbeVerdict::Contradicted(format!("the census is unreadable: {error}"));
        }
    };
    if census.samples.len() < 2 {
        return ProbeVerdict::Contradicted(format!(
            "{} census samples; the load needs one at its start and one at its end at least",
            census.samples.len()
        ));
    }
    let mut oldest = 0;
    for (index, sample) in census.samples.iter().enumerate() {
        if let Some(error) = &sample.error {
            return ProbeVerdict::Contradicted(format!("census sample {index} failed: {error}"));
        }
        let fresh: Vec<&CensusServer> = sample
            .servers
            .iter()
            .filter(|server| server.heartbeat_age_ms <= HEARTBEAT_FRESHNESS_MS)
            .collect();
        if fresh.len() < usize::from(census.expected_servers) {
            return ProbeVerdict::Contradicted(format!(
                "census sample {index} at {} saw {} of {} fleet servers heartbeating",
                sample.observed_unix_ms,
                fresh.len(),
                census.expected_servers
            ));
        }
        oldest = fresh
            .iter()
            .map(|server| server.heartbeat_age_ms)
            .fold(oldest, u64::max);
    }
    ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!(
        "{} census samples, each with {} heartbeating fleet servers (oldest heartbeat {:.1} s)",
        census.samples.len(),
        census.expected_servers,
        oldest as f64 / 1000.0
    )))
}

fn describe_totals(sample: &GameOperationSample) -> String {
    GAME_ROUTE_FAMILIES
        .iter()
        .map(|(family, _)| {
            let total: u64 = sample
                .requests
                .get(*family)
                .map_or(0, |statuses| statuses.values().sum());
            format!("{family} {total}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn family_of(route: &str) -> Option<&'static str> {
    GAME_ROUTE_FAMILIES
        .iter()
        .find(|(_, fragment)| route.contains(fragment))
        .map(|(family, _)| *family)
}

fn label<'a>(labels: &'a str, key: &str) -> Option<&'a str> {
    let start = labels.find(&format!("{key}=\""))? + key.len() + 2;
    let end = labels[start..].find('"')?;
    Some(&labels[start..start + end])
}
