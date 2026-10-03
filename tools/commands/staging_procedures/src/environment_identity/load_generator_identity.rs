//! The load generator's identity: this workstation's CPU, memory, the interface its source
//! addresses sit on, and the round-trip time to the staging host.
//!
//! **Role:** reads the local hardware and network the load receipt records as `hardware` and
//! `network`, and parses each source.
//!
//! **Position:** used by [`super::collect`] for the load check and by the load procedure's
//! observations.
//!
//! **Signals & state:** none; reads `/proc` and runs `ip` and `ping` locally.
//!
//! **Invariants:** only reads; a value that cannot be read is written `unavailable (<why>)`,
//! never guessed.

use std::time::Duration;

use process_runner::Run;

use crate::staging_settings::StagingSettings;

/// `<cpu model>, <n> CPUs, <memory> kB memory`.
pub(crate) fn hardware() -> String {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    format!(
        "{}, {} CPUs, {} kB memory",
        cpu_model(&cpuinfo).unwrap_or_else(|| "unavailable (no model name)".to_string()),
        cpuinfo
            .lines()
            .filter(|line| line.starts_with("processor"))
            .count(),
        memory_kib(&meminfo).map_or_else(|| "unavailable".to_string(), |kib| kib.to_string())
    )
}

/// `<interface> for <addresses>, <rtt> ms average round trip to <host>`.
pub(crate) fn network(settings: &StagingSettings) -> String {
    let addresses = Run::new("ip")
        .args(["-o", "addr", "show"])
        .timeout(Duration::from_secs(10))
        .output()
        .map(|output| output.stdout)
        .unwrap_or_default();
    let interface = settings
        .load_source_addresses
        .first()
        .and_then(|address| interface_of(&addresses, address))
        .unwrap_or_else(|| "unavailable (no source address on an interface)".to_string());
    let ping = Run::new("ping")
        .args(["-c", "3", "-q", settings.host.host()])
        .timeout(Duration::from_secs(20))
        .output()
        .map(|output| output.stdout)
        .unwrap_or_default();
    let round_trip = average_round_trip_ms(&ping)
        .map_or_else(|| "unavailable".to_string(), |ms| format!("{ms:.3}"));
    format!(
        "{interface} for {}, {round_trip} ms average round trip to {}",
        settings.load_source_addresses.join(" "),
        settings.host.host()
    )
}

/// The first `model name` of `/proc/cpuinfo`.
pub(crate) fn cpu_model(cpuinfo: &str) -> Option<String> {
    cpuinfo.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == "model name").then(|| value.trim().to_string())
    })
}

/// `MemTotal` of `/proc/meminfo`, in kB.
pub(crate) fn memory_kib(meminfo: &str) -> Option<u64> {
    meminfo.lines().find_map(|line| {
        line.strip_prefix("MemTotal:")?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    })
}

/// The interface of `ip -o addr show` that holds `address`.
pub(crate) fn interface_of(ip_output: &str, address: &str) -> Option<String> {
    ip_output.lines().find_map(|line| {
        let words: Vec<&str> = line.split_whitespace().collect();
        let holds = words
            .windows(2)
            .any(|pair| pair[0] == "inet" && pair[1].split('/').next() == Some(address));
        holds
            .then(|| words.get(1).map(|name| name.to_string()))
            .flatten()
    })
}

/// The average of `ping -q`'s `rtt min/avg/max/mdev = a/b/c/d ms` line.
pub(crate) fn average_round_trip_ms(ping_output: &str) -> Option<f64> {
    ping_output.lines().find_map(|line| {
        let (_, values) = line.split_once(" = ")?;
        values.split('/').nth(1)?.parse().ok()
    })
}
