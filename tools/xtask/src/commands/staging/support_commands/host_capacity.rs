//! `cargo xtask staging status --capacity`: whether the staging host carries the fleet.
//!
//! **Role:** reads the host's load average and memory and every fleet unit's `MemoryCurrent` and
//! `CPUUsageNSec`, and prints them as a table.
//!
//! **Position:** called by `dispatch.rs`; the setup checklist's capacity trial reads it with the
//! five instances idle and again with one client connected.
//!
//! **Signals & state:** none; one read.
//!
//! **Invariants:** only reads; a value systemd does not account is shown as `-`, never as 0.

use std::fmt::Write as _;

use super::super::remote_observers::remote_command::RemoteCommand;
use super::super::remote_observers::unit_state_reader::{self, UnitState};

/// The read: `loadavg=`, `mem_total_kib=`, `mem_available_kib=`, a blank line, then one unit
/// block per unit.
pub(crate) fn command(units: &[String]) -> RemoteCommand {
    let show = unit_state_reader::show(units).command_line;
    RemoteCommand::read_script(
        "host capacity",
        format!(
            "set -uo pipefail\n\
             echo \"loadavg=$(cut -d ' ' -f 1-3 /proc/loadavg)\"\n\
             echo \"mem_total_kib=$(awk '/^MemTotal:/ {{print $2}}' /proc/meminfo)\"\n\
             echo \"mem_available_kib=$(awk '/^MemAvailable:/ {{print $2}}' /proc/meminfo)\"\n\
             echo\n\
             {show}\n"
        ),
    )
}

/// The capacity table of an answer.
pub(crate) fn render(output: &str, units: &[String]) -> String {
    let (host_part, unit_part) = output.split_once("\n\n").unwrap_or((output, ""));
    let host_value = |key: &str| {
        host_part
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .unwrap_or("-")
            .to_string()
    };
    let states = unit_state_reader::parse(unit_part);
    let mut text = String::new();
    let _ = writeln!(text, "load average (1/5/15 min): {}", host_value("loadavg"));
    let _ = writeln!(
        text,
        "memory: {} kB available of {} kB",
        host_value("mem_available_kib"),
        host_value("mem_total_kib")
    );
    let width = units.iter().map(String::len).max().unwrap_or(4).max(4);
    let _ = writeln!(
        text,
        "  {:<width$}  {:<10}  {:>14}  {:>16}",
        "unit", "state", "memory (MiB)", "cpu (s)"
    );
    for unit in units {
        let (state, memory, cpu) = match states.get(unit) {
            Some(UnitState {
                active_state,
                memory_current_bytes,
                cpu_usage_nanoseconds,
                ..
            }) => (
                active_state.clone(),
                memory_current_bytes.map_or("-".to_string(), |bytes| {
                    format!("{:.1}", bytes as f64 / 1_048_576.0)
                }),
                cpu_usage_nanoseconds
                    .map_or("-".to_string(), |ns| format!("{:.1}", ns as f64 / 1e9)),
            ),
            None => ("unreadable".to_string(), "-".to_string(), "-".to_string()),
        };
        let _ = writeln!(
            text,
            "  {unit:<width$}  {state:<10}  {memory:>14}  {cpu:>16}"
        );
    }
    text
}
