//! Quantitative staging acceptance is checked independently of runner success messages.

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Observations {
    Fleet {
        server_ids: Vec<String>,
        client_count: u64,
        scenarios: Vec<String>,
        fixture_sha256: String,
    },
    Discord {
        scenarios: Vec<String>,
        fixture_sha256: String,
    },
    Load {
        duration_seconds: u64,
        member_accounts: u64,
        minimum_concurrent_clients: u64,
        completed_requests: u64,
        unexpected_errors: u64,
        json_read_count: u64,
        json_write_count: u64,
        p95_json_read_ms: f64,
        p95_json_write_ms: f64,
        workload_sha256: String,
        hardware: String,
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
                .ok_or_else(|| anyhow::anyhow!("request count overflow"))?;
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
        _ => anyhow::bail!("wrong or unsupported operational observation kind for {id}"),
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
