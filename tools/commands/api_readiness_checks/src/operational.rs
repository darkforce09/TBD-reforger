//! Quantitative staging acceptance is checked independently of runner success messages.
//!
//! **Role:** Defines the measurements an operational receipt carries for the staging fleet,
//! Discord and load checks, and the thresholds each must meet.
//!
//! **Position:** `evidence.rs` calls [`validate`] on every `operational` receipt it judges;
//! `operational_recording.rs` calls it before it writes a staging receipt and re-exports
//! [`Observations`] to the `cargo xtask staging` procedures that measure them.
//!
//! **Signals & state:** none; pure functions over deserialized values.
//!
//! **Invariants:** a check id accepts only its own measurement kind. The fleet needs five
//! distinct non-empty server ids, two clients, every fleet scenario and a SHA-256 fixture digest;
//! Discord needs every Discord scenario and a fixture digest; load needs 30 minutes, 1000 member
//! accounts, 100 concurrent clients, 20 completed requests a second, no unexpected error, reads
//! and writes that are both present and together no more than the completed requests, p95 JSON
//! reads within 500 ms and writes within 1000 ms, a recorded hardware and network, and a
//! workload digest.

use crate::error::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The measured values of one staging run, tagged by `kind`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Observations {
    /// The `staging_fleet` run: the servers and clients it drove and the scenarios it covered.
    Fleet {
        /// The distinct non-empty ids of the game servers; five are required.
        server_ids: Vec<String>,
        /// The clients that joined; two are required.
        client_count: u64,
        /// The fleet scenarios the run covered; every one is required.
        scenarios: Vec<String>,
        /// The SHA-256 of the fixture manifest written beside the receipt.
        fixture_sha256: String,
    },
    /// The `staging_discord` run: the scenarios it covered.
    Discord {
        /// The Discord scenarios the run covered; every one is required.
        scenarios: Vec<String>,
        /// The SHA-256 of the fixture manifest written beside the receipt.
        fixture_sha256: String,
    },
    /// The `staging_load` run: its population, rate, latencies and environment.
    Load {
        /// How long the load ran, in seconds; at least 1800.
        duration_seconds: u64,
        /// The member accounts the workload used; at least 1000.
        member_accounts: u64,
        /// The fewest clients active at once; at least 100.
        minimum_concurrent_clients: u64,
        /// The requests that completed; at least 20 per second of the run.
        completed_requests: u64,
        /// The requests that failed unexpectedly; none allowed.
        unexpected_errors: u64,
        /// The JSON reads among the completed requests; more than zero.
        json_read_count: u64,
        /// The JSON writes among the completed requests; more than zero.
        json_write_count: u64,
        /// The 95th-percentile JSON read latency in milliseconds; at most 500.
        p95_json_read_ms: f64,
        /// The 95th-percentile JSON write latency in milliseconds; at most 1000.
        p95_json_write_ms: f64,
        /// The SHA-256 of the workload definition.
        workload_sha256: String,
        /// The hardware the load generator ran on; never blank.
        hardware: String,
        /// The network between the generator and the API; never blank.
        network: String,
    },
}

pub(super) fn validate(id: &str, observations: &Observations) -> Result<()> {
    match (id, observations) {
        (
            "staging_fleet",
            Observations::Fleet {
                server_ids,
                client_count,
                scenarios,
                fixture_sha256,
            },
        ) => {
            let servers: BTreeSet<_> = server_ids.iter().filter(|id| !id.is_empty()).collect();
            ensure!(
                servers.len() >= 5 && *client_count >= 2,
                "fleet acceptance requires five distinct servers and real clients"
            );
            digest(fixture_sha256)?;
            includes(
                scenarios,
                &[
                    "start",
                    "stop",
                    "restart",
                    "kick",
                    "custom_console",
                    "same_terrain",
                    "cross_terrain",
                    "lost_acknowledgement",
                    "identity_link",
                ],
            )?;
        }
        (
            "staging_discord",
            Observations::Discord {
                scenarios,
                fixture_sha256,
            },
        ) => {
            digest(fixture_sha256)?;
            includes(
                scenarios,
                &[
                    "role_demotion",
                    "departure_to_guest",
                    "partner_membership",
                    "rate_limit",
                    "network_outage",
                    "cached_grace",
                    "staleness_warning",
                    "admin_override",
                    "eligibility_release",
                ],
            )?;
        }
        (
            "staging_load",
            Observations::Load {
                duration_seconds,
                member_accounts,
                minimum_concurrent_clients,
                completed_requests,
                unexpected_errors,
                json_read_count,
                json_write_count,
                p95_json_read_ms,
                p95_json_write_ms,
                workload_sha256,
                hardware,
                network,
            },
        ) => {
            ensure!(
                *duration_seconds >= 1800
                    && *member_accounts >= 1000
                    && *minimum_concurrent_clients >= 100,
                "load population, concurrency or duration below acceptance target"
            );
            let required = duration_seconds
                .checked_mul(20)
                .ok_or_else(|| crate::error::refusal!("request count overflow"))?;
            ensure!(
                *completed_requests >= required && *unexpected_errors == 0,
                "insufficient request rate or unexpected failures"
            );
            ensure!(
                *json_read_count > 0
                    && *json_write_count > 0
                    && json_read_count
                        .checked_add(*json_write_count)
                        .is_some_and(|n| n <= *completed_requests),
                "invalid read/write population"
            );
            ensure!(
                p95_json_read_ms.is_finite() && (0.0..=500.0).contains(p95_json_read_ms),
                "JSON read latency fails acceptance"
            );
            ensure!(
                p95_json_write_ms.is_finite() && (0.0..=1000.0).contains(p95_json_write_ms),
                "JSON write latency fails acceptance"
            );
            ensure!(
                !hardware.trim().is_empty() && !network.trim().is_empty(),
                "missing benchmark environment"
            );
            digest(workload_sha256)?;
        }
        _ => crate::error::bail!("wrong or unsupported operational observation kind for {id}"),
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    ensure!(
        value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()),
        "missing SHA256 fixture/workload identity"
    );
    Ok(())
}

fn includes(actual: &[String], required: &[&str]) -> Result<()> {
    for scenario in required {
        ensure!(
            actual.iter().any(|value| value == scenario),
            "missing staging scenario: {scenario}"
        );
    }
    Ok(())
}
