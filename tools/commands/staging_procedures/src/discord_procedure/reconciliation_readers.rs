//! The API's own witnesses of Discord reconciliation: the `discord_reconciliation` log lines of
//! its unit journal and the `tbd_discord_reconcile_outcomes_total` counter of `/metrics`.
//!
//! **Role:** parses the log lines (with any colour escapes stripped) and builds the probes that
//! wait for a logged or counted outcome.
//!
//! **Position:** used by `outage_steps.rs` (`unavailable`) and `rate_limit_steps.rs`
//! (`rate_limited`); the reads run through `remote_observers/unit_journal_reader.rs` and
//! `remote_observers/metrics_reader.rs`.
//!
//! **Signals & state:** none; a logged outcome measures `<outcome>_logged_ms`.
//!
//! **Invariants:** only main-guild lines logged after the step's start count; the counters are
//! in-process, so a count compares with a baseline measured after the last API restart.

use super::membership_queries::DiscordTargets;
use crate::procedure_runner::step::{Probe, ProbeVerdict};
use crate::remote_observers::{metrics_reader, unit_journal_reader};

/// The counter of reconciliation outcomes.
pub(super) const OUTCOME_COUNTER: &str = "tbd_discord_reconcile_outcomes_total";

/// The `discord_reconciliation` log lines of an API journal: `(unix ms, outcome, guild_scope)`.
pub(super) fn reconciliation_outcomes(journal: &str) -> Vec<(u64, String, String)> {
    unit_journal_reader::parse(journal)
        .into_iter()
        .filter_map(|line| {
            let text = strip_ansi(&line.text);
            text.contains("discord_reconciliation").then(|| {
                (
                    line.unix_ms,
                    field(&text, "outcome"),
                    field(&text, "guild_scope"),
                )
            })
        })
        .collect()
}

fn strip_ansi(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            for inner in chars.by_ref() {
                if inner.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            plain.push(character);
        }
    }
    plain
}

fn field(text: &str, name: &str) -> String {
    text.split_whitespace()
        .find_map(|word| word.strip_prefix(&format!("{name}=")))
        .map(|value| value.trim_matches('"').to_string())
        .unwrap_or_default()
}

/// A probe that holds once the API journal since the step's start logs `outcome` for the main
/// guild.
pub(super) fn logged_outcome(targets: &DiscordTargets, outcome: &'static str) -> Probe {
    let unit = targets.settings.api_unit.clone();
    Probe::host(
        move |context| {
            Ok(unit_journal_reader::since(
                &unit,
                context.step_started_unix_ms / 1000,
            ))
        },
        move |text, context| match reconciliation_outcomes(text).into_iter().find(
            |(at, seen, scope)| {
                seen == outcome && scope == "main" && *at >= context.step_started_unix_ms
            },
        ) {
            Some((at, ..)) => ProbeVerdict::Satisfied(
                ProbeVerdict::satisfied(format!(
                    "discord_reconciliation outcome={outcome} guild_scope=main logged at {at}"
                ))
                .at(at)
                .measure(format!("{outcome}_logged_ms"), at),
            ),
            None => ProbeVerdict::Pending(format!("no {outcome} outcome logged yet")),
        },
    )
}

/// A probe of the outcome counter: holds once `outcome` counts more than `baseline` (a
/// measurement name) or than 0.
pub(super) fn counted_outcome(
    targets: &DiscordTargets,
    outcome: &'static str,
    baseline: Option<&'static str>,
) -> Probe {
    let settings = targets.settings.clone();
    Probe::host(
        move |_| {
            Ok(metrics_reader::exposition(
                &settings.api_env_file(),
                &settings.api_origin,
            ))
        },
        move |text, context| {
            let before = baseline
                .and_then(|key| context.measurements.get(key))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            match metrics_reader::sample(text, OUTCOME_COUNTER, &[("outcome", outcome)]) {
                Some(count) if count > before => ProbeVerdict::Satisfied(ProbeVerdict::satisfied(
                    format!("{OUTCOME_COUNTER}{{outcome=\"{outcome}\"}} {count} (was {before})"),
                )),
                Some(count) => ProbeVerdict::Pending(format!("{outcome} count {count}")),
                None => ProbeVerdict::Pending(format!("no {OUTCOME_COUNTER} {outcome} sample")),
            }
        },
    )
}
