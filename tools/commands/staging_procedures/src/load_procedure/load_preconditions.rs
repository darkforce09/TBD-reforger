//! What must hold before member load reaches staging: the proxy trust, the empty bot token, the
//! source addresses, the per-address keying, the seeded population and the token file.
//!
//! **Role:** the load procedure's `staging preflight` checks, the keying probe (one invalid
//! refresh per source address, each answered 401, leaving one `strict|<address>` bucket each),
//! and the judges of the population, fixture events, token file and keying effects.
//!
//! **Position:** `mod.rs` hands the checks to `staging preflight`; `load_steps.rs` builds the
//! `population` and `keying_probe` effects from the judges; `load_run.rs` and the rehearsal send
//! the keying refreshes and read the token file.
//!
//! **Signals & state:** none; the keying sends go through [`WorkstationLoad`].
//!
//! **Invariants:** no secret is read or printed: the bot token is reported as set or empty by
//! the host, the proxy list is not secret, and the token file is read for its Discord ids only;
//! the keying refresh carries a fixed invalid token; a bucket counts only for its own address.

use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

use crate::error::{Result, ResultExt};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::committed_load_data::CommittedLoadData;
use super::load_queries::{fixture_events, population_census, strict_buckets};
use super::workstation_load::WorkstationLoad;
use crate::procedure_runner::step::{ProbeVerdict, StepContext};
use crate::remote_observers::remote_command::{RemoteCommand, shell_quote};

/// The measurement holding the token file's shape.
pub(crate) const ACCOUNT_FILE_MEASUREMENT: &str = "account_file";
/// The measurement holding the keying answers.
pub(crate) const KEYING_MEASUREMENT: &str = "keying_answers";
/// The measurement holding the seeded fixture events.
pub(crate) const FIXTURE_EVENTS_MEASUREMENT: &str = "fixture_events";
/// The status an invalid refresh must be answered with.
pub(crate) const KEYING_STATUS: u16 = 401;
/// How far the host's clock may trail the workstation's when a bucket's update is compared with
/// the step's start.
pub(crate) const CLOCK_SLACK_MS: u64 = 300_000;
/// The peer the API sees for every request through Caddy on the host network.
pub(crate) const PROXY_PEER: &str = "127.0.0.1";

/// One keying refresh and how it was answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct KeyingAnswer {
    pub address: String,
    pub status: Option<u16>,
    pub error: Option<String>,
}

/// The token file's shape: never its tokens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AccountFileShape {
    pub(crate) accounts: u64,
    pub(crate) first_discord_id: String,
    pub(crate) last_discord_id: String,
    /// Whether account `k` holds id `first + k` for every `k`.
    pub(crate) consecutive: bool,
}

#[derive(Deserialize)]
struct AccountFileDocument {
    accounts: Vec<AccountEntry>,
}

#[derive(Deserialize)]
struct AccountEntry {
    discord_id: String,
}

/// Reads the token file's Discord ids; its tokens are dropped unread.
pub(crate) fn account_file_shape(path: &Path) -> Result<AccountFileShape> {
    let text = std::fs::read(path)
        .with_context(|| format!("reading the token file {}", path.display()))?;
    let document: AccountFileDocument = serde_json::from_slice(&text)
        .with_context(|| format!("{} is not an account file", path.display()))?;
    let ids: Vec<&str> = document
        .accounts
        .iter()
        .map(|a| a.discord_id.as_str())
        .collect();
    let first = ids.first().and_then(|id| id.parse::<u64>().ok());
    let consecutive = first.is_some_and(|first| {
        ids.iter()
            .enumerate()
            .all(|(k, id)| id.parse::<u64>().ok() == Some(first + k as u64))
    });
    Ok(AccountFileShape {
        accounts: ids.len() as u64,
        first_discord_id: ids.first().unwrap_or(&"").to_string(),
        last_discord_id: ids.last().unwrap_or(&"").to_string(),
        consecutive,
    })
}

/// Sends one invalid refresh from each address and records the answers.
pub(crate) fn send_keying_refreshes(
    workstation: &Arc<dyn WorkstationLoad>,
    origin: &str,
    addresses: &[IpAddr],
) -> Vec<KeyingAnswer> {
    addresses
        .iter()
        .map(
            |address| match workstation.keying_refresh(origin, *address) {
                Ok(status) => KeyingAnswer {
                    address: address.to_string(),
                    status: Some(status),
                    error: None,
                },
                Err(error) => KeyingAnswer {
                    address: address.to_string(),
                    status: None,
                    error: Some(format!("{error:#}")),
                },
            },
        )
        .collect()
}

/// Whether every address answered [`KEYING_STATUS`], with the evidence or the reason.
pub(crate) fn keying_answers_hold(
    answers: &[KeyingAnswer],
    addresses: &[IpAddr],
) -> Result<String, String> {
    let described = answers
        .iter()
        .map(|answer| match (answer.status, &answer.error) {
            (Some(status), _) => format!("{} {status}", answer.address),
            (None, Some(error)) => format!("{} failed: {error}", answer.address),
            (None, None) => format!("{} unanswered", answer.address),
        })
        .collect::<Vec<_>>()
        .join(", ");
    let all = addresses.len() == answers.len()
        && addresses.iter().all(|address| {
            answers
                .iter()
                .any(|a| a.address == address.to_string() && a.status == Some(KEYING_STATUS))
        });
    if all && !addresses.is_empty() {
        Ok(described)
    } else {
        Err(described)
    }
}

/// The keying answers judge.
pub(crate) fn judge_keying_answers(
    addresses: Vec<IpAddr>,
) -> impl Fn(&str, &StepContext<'_>) -> ProbeVerdict {
    move |text: &str, _: &StepContext<'_>| match serde_json::from_str::<Vec<KeyingAnswer>>(text) {
        Err(error) => {
            ProbeVerdict::Contradicted(format!("the keying answers are unreadable: {error}"))
        }
        Ok(answers) => match keying_answers_hold(&answers, &addresses) {
            Ok(summary) => ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!(
                "every source address was answered {KEYING_STATUS}: {summary}"
            ))),
            Err(why) => ProbeVerdict::Contradicted(format!(
                "an invalid refresh was not answered {KEYING_STATUS}: {why}"
            )),
        },
    }
}

/// Whether each address has its own `strict|<address>` bucket updated since `since_unix_ms`.
pub(crate) fn strict_buckets_hold(
    output: &str,
    addresses: &[IpAddr],
    since_unix_ms: u64,
) -> Result<String, String> {
    let buckets = strict_buckets(output);
    let missing: Vec<String> = addresses
        .iter()
        .filter(|address| {
            let key = format!("strict|{address}");
            !buckets.iter().any(|(bucket, updated)| {
                *bucket == key && updated.saturating_add(CLOCK_SLACK_MS) >= since_unix_ms
            })
        })
        .map(ToString::to_string)
        .collect();
    if missing.is_empty() && !addresses.is_empty() {
        Ok(format!(
            "one strict bucket per source address ({} buckets)",
            addresses.len()
        ))
    } else {
        Err(format!("no fresh strict bucket for {}", missing.join(", ")))
    }
}

/// The strict bucket judge: pending until every address's bucket is there.
pub(crate) fn judge_strict_buckets(
    addresses: Vec<IpAddr>,
) -> impl Fn(&str, &StepContext<'_>) -> ProbeVerdict {
    move |output: &str, context: &StepContext<'_>| match strict_buckets_hold(
        output,
        &addresses,
        context.step_started_unix_ms,
    ) {
        Ok(summary) => ProbeVerdict::Satisfied(ProbeVerdict::satisfied(summary)),
        Err(seen) => ProbeVerdict::Pending(seen),
    }
}

/// The synthetic population judge: exactly the committed accounts, all enlisted members.
pub(crate) fn judge_population(
    data: &CommittedLoadData,
) -> Result<impl Fn(&str, &StepContext<'_>) -> ProbeVerdict + 'static> {
    let accounts = u64::from(data.population.accounts);
    let lowest = data.id_base()?.to_string();
    let highest = data.last_account_id()?.to_string();
    Ok(
        move |output: &str, _: &StepContext<'_>| match population_census(output) {
            None => ProbeVerdict::Pending("the population census returned no row".into()),
            Some(census) => {
                let summary = format!(
                    "{} synthetic accounts ({} enlisted), ids {} to {}",
                    census.accounts, census.enlisted_accounts, census.lowest_id, census.highest_id
                );
                if census.accounts == accounts
                    && census.enlisted_accounts == accounts
                    && census.lowest_id == lowest
                    && census.highest_id == highest
                {
                    ProbeVerdict::Satisfied(ProbeVerdict::satisfied(summary))
                } else {
                    ProbeVerdict::Contradicted(format!(
                        "{summary}; expected {accounts} enlisted, {lowest} to {highest}"
                    ))
                }
            }
        },
    )
}

/// The fixture events judge: the committed events, each with its full ORBAT on the committed
/// mission; measures the events the member load writes to.
pub(crate) fn judge_fixture_events(
    data: &CommittedLoadData,
) -> impl Fn(&str, &StepContext<'_>) -> ProbeVerdict + 'static {
    let plan = data.population.fixture_events.clone();
    move |output: &str, _: &StepContext<'_>| {
        let events = fixture_events(output);
        let titles: Vec<String> = (1..=plan.count).map(|number| plan.title(number)).collect();
        let slots = plan.slots_per_event() as usize;
        let summary = format!(
            "{} fixture events, slots {:?}, missions {:?}",
            events.len(),
            events.iter().map(|e| e.slot_ids.len()).collect::<Vec<_>>(),
            events
                .iter()
                .map(|e| e.mission_title.as_str())
                .collect::<std::collections::BTreeSet<_>>()
        );
        let holds = events.len() == titles.len()
            && events.iter().zip(&titles).all(|(event, title)| {
                event.title == *title
                    && event.slot_ids.len() == slots
                    && event.mission_title == plan.mission_title
            })
            && events
                .windows(2)
                .all(|pair| pair[0].mission_id == pair[1].mission_id);
        if !holds {
            return ProbeVerdict::Contradicted(format!(
                "{summary}; expected {} events titled {:?} … with {slots} slots on {:?}",
                plan.count,
                titles.first(),
                plan.mission_title
            ));
        }
        let bindings: Vec<serde_json::Value> = events
            .iter()
            .map(|event| {
                json!({
                    "event_id": event.event_id,
                    "event_mission_id": event.event_mission_id,
                    "mission_id": event.mission_id,
                    "slot_ids": event.slot_ids,
                })
            })
            .collect();
        ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(summary).measure(FIXTURE_EVENTS_MEASUREMENT, bindings),
        )
    }
}

/// The token file judge: the committed account count, consecutive from the id base.
pub(crate) fn judge_account_file(
    data: &CommittedLoadData,
) -> Result<impl Fn(&str, &StepContext<'_>) -> ProbeVerdict + 'static> {
    let accounts = u64::from(data.population.accounts);
    let first = data.id_base()?.to_string();
    Ok(move |text: &str, _: &StepContext<'_>| {
        match serde_json::from_str::<AccountFileShape>(text) {
            Err(error) => {
                ProbeVerdict::Contradicted(format!("the token file shape is unreadable: {error}"))
            }
            Ok(shape) => {
                let summary = format!(
                    "the token file holds {} accounts, {} to {}, consecutive: {}",
                    shape.accounts,
                    shape.first_discord_id,
                    shape.last_discord_id,
                    shape.consecutive
                );
                if shape.accounts == accounts
                    && shape.first_discord_id == first
                    && shape.consecutive
                {
                    ProbeVerdict::Satisfied(ProbeVerdict::satisfied(summary))
                } else {
                    ProbeVerdict::Contradicted(format!(
                        "{summary}; expected {accounts} from {first}"
                    ))
                }
            }
        }
    })
}

/// The host read of one key of the API env file, printing only whether it holds a value.
pub(crate) fn env_key_presence(api_env_file: &str, key: &str) -> RemoteCommand {
    RemoteCommand::read_script(
        "api env file",
        format!(
            "set -euo pipefail\n\
             value=\"$(sed -n 's/^{key}=//p' {} | tail -n 1 | tr -d '\"\\r')\"\n\
             if [ -n \"$value\" ]; then echo set; else echo empty; fi\n",
            shell_quote(api_env_file)
        ),
    )
}

/// The host read of the API env file's `TRUSTED_PROXIES` list, which is not a secret.
pub(crate) fn trusted_proxies_read(api_env_file: &str) -> RemoteCommand {
    RemoteCommand::read_script(
        "api env file",
        format!(
            "set -euo pipefail\nsed -n 's/^TRUSTED_PROXIES=//p' {} | tail -n 1 | tr -d '\"\\r'\n",
            shell_quote(api_env_file)
        ),
    )
}

/// Whether the trusted proxies cover [`PROXY_PEER`] and no source address.
pub(crate) fn trusted_proxies_hold(list: &str, sources: &[IpAddr]) -> Result<String, String> {
    let entries: Vec<&str> = list
        .split([',', ' '])
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .collect();
    let peer: IpAddr = PROXY_PEER
        .parse()
        .map_err(|_| "the proxy peer does not parse".to_string())?;
    let covers = |ip: &IpAddr| entries.iter().any(|entry| cidr_covers(entry, ip));
    if !covers(&peer) {
        return Err(format!(
            "TRUSTED_PROXIES {entries:?} does not cover the Caddy peer {PROXY_PEER}"
        ));
    }
    let trusted: Vec<String> = sources
        .iter()
        .filter(|ip| covers(ip))
        .map(ToString::to_string)
        .collect();
    if trusted.is_empty() {
        Ok(format!(
            "TRUSTED_PROXIES {entries:?} covers {PROXY_PEER} and no source address"
        ))
    } else {
        Err(format!(
            "TRUSTED_PROXIES {entries:?} also trusts source addresses {}",
            trusted.join(", ")
        ))
    }
}

/// Whether `entry` (an address or a CIDR block) covers `ip`; an entry that does not parse
/// covers nothing.
pub(crate) fn cidr_covers(entry: &str, ip: &IpAddr) -> bool {
    let (address, length) = match entry.split_once('/') {
        Some((address, length)) => (address, length.parse::<u32>().ok()),
        None => (entry, None),
    };
    match (address.parse::<IpAddr>(), ip) {
        (Ok(IpAddr::V4(network)), IpAddr::V4(ip)) => {
            let bits = length.unwrap_or(32).min(32);
            let mask = if bits == 0 {
                0
            } else {
                u32::MAX << (32 - bits)
            };
            u32::from(network) & mask == u32::from(*ip) & mask
        }
        (Ok(IpAddr::V6(network)), IpAddr::V6(ip)) => {
            let bits = length.unwrap_or(128).min(128);
            let mask = if bits == 0 {
                0
            } else {
                u128::MAX << (128 - bits)
            };
            u128::from(network) & mask == u128::from(*ip) & mask
        }
        _ => false,
    }
}
