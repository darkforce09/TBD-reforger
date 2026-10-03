//! `cargo xtask staging status`: the staging host's resting state and content, each item beside
//! the value it should hold.
//!
//! **Role:** reads the synthetic account and fixture event counts, the outage drop-in, the relay's
//! armed state and the fleet units, plus the setup content (servers, missions, fleet scenario
//! rows, ballistics catalogs), and prints them as one table.
//!
//! **Position:** called by `staging_dispatch.rs`; `--capacity` goes to `host_capacity.rs` instead; the
//! recovery runbook reads this table after a stopped run.
//!
//! **Signals & state:** none; every item is a read.
//!
//! **Invariants:** only reads; a read that fails shows `unreadable (<why>)` and counts as off its
//! resting value; the exit code is 0 only when every resting item holds its expected value.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::remote_actions::{outage_dropin, relay_control};
use crate::remote_observers::database_reader::{self, RESTING_STATE_COUNTS};
use crate::remote_observers::remote_command::{HostCommandRunner, RemoteCommand};
use crate::remote_observers::unit_state_reader;
use crate::staging_settings::StagingSettings;

/// One row of the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatusItem {
    pub name: String,
    pub observed: String,
    pub expected: String,
    /// Whether the item belongs to the resting state (and so to the exit code), rather than
    /// being setup content shown for reference.
    pub resting: bool,
}

impl StatusItem {
    /// Whether the observed value is the expected one.
    pub(crate) fn holds(&self) -> bool {
        self.observed == self.expected
    }
}

/// Reads every item.
pub(crate) fn items(
    settings: &StagingSettings,
    host: &mut dyn HostCommandRunner,
) -> Vec<StatusItem> {
    let mut items = Vec::new();
    let counts: BTreeMap<String, String> =
        match database_reader::select(&settings.database_container, &RESTING_STATE_COUNTS, &[])
            .and_then(|command| answer(host, &command))
        {
            Ok(text) => database_reader::rows(&text)
                .into_iter()
                .filter_map(|row| Some((row.first()?.clone(), row.get(1)?.clone())))
                .collect(),
            Err(why) => [
                "synthetic_accounts",
                "load_fixture_events",
                "active_servers",
                "live_missions_with_artifacts",
                "fleet_scenarios",
                "ballistics_catalogs",
            ]
            .into_iter()
            .map(|key| (key.to_string(), format!("unreadable ({why:#})")))
            .collect(),
        };
    let count = |key: &str| {
        counts
            .get(key)
            .cloned()
            .unwrap_or_else(|| "unreadable (no row)".into())
    };
    items.push(item(
        "synthetic accounts",
        count("synthetic_accounts"),
        "0",
        true,
    ));
    items.push(item(
        "[Load fixture] events",
        count("load_fixture_events"),
        "0",
        true,
    ));
    let dropin = answer(host, &outage_dropin::state(settings))
        .map(|text| text.trim().to_string())
        .unwrap_or_else(|why| format!("unreadable ({why:#})"));
    items.push(item("API outage drop-in", dropin, "absent", true));
    if let Some((instance, _)) = settings.relay_unit() {
        let relay = answer(host, &relay_control::status(instance))
            .and_then(|text| relay_control::is_disarmed(&text))
            .map(|disarmed| if disarmed { "disarmed" } else { "armed" }.to_string())
            .unwrap_or_else(|why| format!("unreadable ({why:#})"));
        items.push(item(
            &format!("relay of instance {instance}"),
            relay,
            "disarmed",
            true,
        ));
    }
    let mut units = settings.game_server_units();
    units.extend(settings.host_agent_units());
    units.extend(settings.relay_unit().map(|(_, unit)| unit));
    let states =
        answer(host, &unit_state_reader::show(&units)).map(|text| unit_state_reader::parse(&text));
    for unit in &units {
        let observed = match &states {
            Ok(states) => states.get(unit).map_or_else(
                || "unreadable (no state)".to_string(),
                |state| state.active_state.clone(),
            ),
            Err(why) => format!("unreadable ({why:#})"),
        };
        items.push(item(unit, observed, "active", true));
    }
    let fleet = settings.fleet.instance_count.to_string();
    items.push(item(
        "active servers",
        count("active_servers"),
        &fleet,
        false,
    ));
    items.push(item(
        "live missions with artifacts",
        count("live_missions_with_artifacts"),
        "2",
        false,
    ));
    items.push(item(
        "fleet scenario rows",
        count("fleet_scenarios"),
        "2",
        false,
    ));
    items.push(item(
        "ballistics catalogs",
        count("ballistics_catalogs"),
        "1",
        false,
    ));
    items
}

/// The table: `item | observed | expected | ok`, resting items first, then the setup content.
pub(crate) fn render(items: &[StatusItem]) -> String {
    let width = items
        .iter()
        .map(|item| item.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let mut text = String::new();
    for (heading, resting) in [("resting state", true), ("setup content", false)] {
        let _ = writeln!(text, "{heading}:");
        let _ = writeln!(
            text,
            "  {:<width$}  {:<24}  {:<10}  ok",
            "item", "observed", "expected"
        );
        for item in items.iter().filter(|item| item.resting == resting) {
            let mark = if item.holds() { "yes" } else { "NO" };
            let _ = writeln!(
                text,
                "  {:<width$}  {:<24}  {:<10}  {mark}",
                item.name, item.observed, item.expected
            );
        }
    }
    text
}

/// 0 when every resting item holds, else 1.
pub(crate) fn exit_code(items: &[StatusItem]) -> u8 {
    u8::from(items.iter().any(|item| item.resting && !item.holds()))
}

fn item(name: &str, observed: String, expected: &str, resting: bool) -> StatusItem {
    StatusItem {
        name: name.to_string(),
        observed,
        expected: expected.to_string(),
        resting,
    }
}

/// The standard output of `command`, or why there is none.
fn answer(
    host: &mut dyn HostCommandRunner,
    command: &RemoteCommand,
) -> crate::error::Result<String> {
    let output = host.run(command)?;
    crate::error::ensure!(
        output.exit_code == 0,
        "{} exited {}",
        command.observer,
        output.exit_code
    );
    Ok(output.stdout)
}
