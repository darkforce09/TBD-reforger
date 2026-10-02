//! The systemd state of the staging user units.
//!
//! **Role:** builds the `systemctl --user show` read of a set of units and parses each unit's
//! state, main PID, start time, memory and CPU use.
//!
//! **Position:** used by the fleet effects (unit active or inactive, same or new PID),
//! `staging status` and `staging status --capacity`.
//!
//! **Signals & state:** none; pure builder and parser.
//!
//! **Invariants:** `show` only reads; a property systemd reports as unset (`[not set]`, empty, or
//! `u64::MAX`) parses as `None`, never as zero.

use std::collections::BTreeMap;

use super::remote_command::{RemoteCommand, shell_words};

/// The properties every read asks for.
pub(crate) const UNIT_PROPERTIES: [&str; 7] = [
    "Id",
    "ActiveState",
    "SubState",
    "MainPID",
    "ExecMainStartTimestampMonotonic",
    "MemoryCurrent",
    "CPUUsageNSec",
];

/// One unit's state as `systemctl show` reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnitState {
    /// `Id`: the unit's full name.
    pub unit: String,
    /// `ActiveState`: `active`, `inactive`, `failed`, `activating`, …
    pub active_state: String,
    /// `SubState`: `running`, `dead`, …
    pub sub_state: String,
    /// `MainPID`, when the unit has a running main process.
    pub main_pid: Option<u32>,
    /// `ExecMainStartTimestampMonotonic` in microseconds, when the main process started.
    pub started_monotonic_microseconds: Option<u64>,
    /// `MemoryCurrent` in bytes, when accounted.
    pub memory_current_bytes: Option<u64>,
    /// `CPUUsageNSec`, when accounted.
    pub cpu_usage_nanoseconds: Option<u64>,
}

/// The read of `units`, one property block per unit.
pub(crate) fn show(units: &[String]) -> RemoteCommand {
    let mut words = vec![
        "systemctl".to_string(),
        "--user".into(),
        "show".into(),
        "--no-pager".into(),
        format!("--property={}", UNIT_PROPERTIES.join(",")),
    ];
    words.extend(units.iter().cloned());
    RemoteCommand::read("unit state", shell_words(&words))
}

/// Every unit block of `output`, keyed by its `Id`.
pub(crate) fn parse(output: &str) -> BTreeMap<String, UnitState> {
    let mut states = BTreeMap::new();
    for block in output.split("\n\n") {
        let values: BTreeMap<&str, &str> = block
            .lines()
            .filter_map(|line| line.split_once('='))
            .collect();
        let Some(unit) = values.get("Id").filter(|unit| !unit.is_empty()) else {
            continue;
        };
        let text = |key: &str| values.get(key).copied().unwrap_or_default().to_string();
        let number = |key: &str| {
            values
                .get(key)
                .and_then(|value| value.parse::<u64>().ok())
                .filter(|value| *value != u64::MAX)
        };
        states.insert(
            unit.to_string(),
            UnitState {
                unit: unit.to_string(),
                active_state: text("ActiveState"),
                sub_state: text("SubState"),
                main_pid: number("MainPID")
                    .filter(|pid| *pid != 0)
                    .and_then(|pid| u32::try_from(pid).ok()),
                started_monotonic_microseconds: number("ExecMainStartTimestampMonotonic")
                    .filter(|value| *value != 0),
                memory_current_bytes: number("MemoryCurrent"),
                cpu_usage_nanoseconds: number("CPUUsageNSec"),
            },
        );
    }
    states
}
