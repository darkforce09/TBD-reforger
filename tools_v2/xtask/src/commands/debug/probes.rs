//! The probes of `cargo xtask debug …`.
//!
//! **Role:** the A2S query, the NDJSON row writer and the six-row direct-join block, each also a
//! subcommand (`a2s-probe`, `ndjson-append`, `direct-join-log`).
//!
//! **Position:** called by [`crate::commands::debug::dispatch`] and by the orchestrator,
//! [`crate::commands::debug::direct_join`].
//!
//! **Signals & state:** appends to the log file it is given; nothing else.
//!
//! **Invariants:** a host is resolved to one IPv4 address before any datagram is sent, and every
//! A2S row names that address, or carries the resolution error; no row names a host the caller
//! did not pass; the H1 row keys its listener counts by the ports the caller probed, never by a
//! fixed pair.

use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::core::deploy_environment::first_ipv4_address;

/// A2S query of `host` on each port → one JSON object keyed `p<port>`.
pub fn a2s_probe_json(host: &str, ports: &[u16]) -> String {
    a2s_probe_json_for(host, &first_ipv4_address(host), ports)
}

/// [`a2s_probe_json`] for a host already resolved; a resolution error becomes each port's
/// `error`.
pub fn a2s_probe_json_for(host: &str, address: &Result<Ipv4Addr, String>, ports: &[u16]) -> String {
    let mut out = serde_json::Map::new();
    for &port in ports {
        let row = match address {
            Ok(ipv4) => probe_one(SocketAddrV4::new(*ipv4, port)),
            Err(reason) => json!({
                "port": port,
                "ok": false,
                "error": format!("{host} has no IPv4 address from here: {reason}"),
            }),
        };
        out.insert(format!("p{port}"), row);
    }
    Value::Object(out).to_string()
}

/// UDP Source Engine Query probe → JSON on stdout.
pub fn cmd_a2s_probe(host: &str, ports: &[u16]) -> Result<()> {
    println!("{}", a2s_probe_json(host, ports));
    Ok(())
}

fn probe_one(target: SocketAddrV4) -> Value {
    let port = target.port();
    let address = target.ip().to_string();
    let failed = |error: std::io::Error| json!({"port": port, "address": address, "ok": false, "error": error.to_string()});
    let sock = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => return failed(e),
    };
    if let Err(e) = sock.set_read_timeout(Some(Duration::from_secs(2))) {
        return failed(e);
    }
    // A2S_INFO / TSource Engine Query
    let payload: &[u8] = b"\xFF\xFF\xFF\xFFTSource Engine Query\x00";
    if let Err(e) = sock.send_to(payload, target) {
        return failed(e);
    }
    let mut buf = [0u8; 4096];
    match sock.recv_from(&mut buf) {
        Ok((n, from)) => json!({
            "port": port,
            "address": address,
            "ok": true,
            "bytes": n,
            "from": from.to_string(),
        }),
        Err(e) => failed(e),
    }
}

/// Append one NDJSON debug line.
pub fn cmd_ndjson_append(
    log: &Path,
    hypothesis_id: &str,
    message: &str,
    data_json: &str,
    run_id: &str,
) -> Result<()> {
    let data: Value = serde_json::from_str(data_json).unwrap_or(json!({}));
    append(log, run_id, hypothesis_id, message, data)
}

/// What one direct-join run observed, written as the six hypothesis rows.
pub struct DirectJoinObservations<'a> {
    pub run_id: &'a str,
    /// The remote probe's output: service state, UDP listeners and log lines.
    pub remote: &'a str,
    /// The probed server's game and A2S ports, whose listener counts in `remote` H1 reads under
    /// `udp_<port>`; `None` when the settings named no server, and H1 then carries no such key.
    pub listener_ports: Option<[u16; 2]>,
    pub client_build: &'a str,
    pub server_build: &'a str,
    pub symlink: &'a str,
    /// The ping round trip in milliseconds, `fail` or `skipped`.
    pub ping_ms: &'a str,
    /// The host name pinged, as `TBD_SSH_HOST` or `--host` gave it.
    pub ping_host: &'a str,
    /// The IPv4 address the ping and the A2S probe went to.
    pub ping_address: &'a str,
    /// The A2S result as JSON; text that is not JSON is written as `{}`.
    pub a2s_json: &'a str,
}

/// Write the six direct-join rows, H1 to H6.
pub fn cmd_direct_join_log(log: &Path, observed: &DirectJoinObservations<'_>) -> Result<()> {
    let run_id = observed.run_id;
    let a2s: Value = serde_json::from_str(observed.a2s_json).unwrap_or(json!({}));
    let remote = observed.remote.trim();
    append(
        log,
        run_id,
        "H1",
        "remote service and ports",
        service_and_listeners(remote, observed.listener_ports),
    )?;
    append(
        log,
        run_id,
        "H2",
        "steam build ids",
        json!({
            "client_build": observed.client_build,
            "server_build": observed.server_build,
            "builds_match": observed.client_build == observed.server_build
                && observed.client_build != "unknown",
        }),
    )?;
    append(
        log,
        run_id,
        "H3",
        "a2s and listen log",
        json!({"remote_snippet": remote}),
    )?;
    append(
        log,
        run_id,
        "H4",
        "ping",
        json!({
            "ms": observed.ping_ms,
            "host": observed.ping_host,
            "address": observed.ping_address,
        }),
    )?;
    append(
        log,
        run_id,
        "H5",
        "client mod symlink",
        json!({"path": observed.symlink, "exists": observed.symlink != "missing"}),
    )?;
    append(log, run_id, "H6", "a2s port probe from client PC", a2s)?;
    Ok(())
}

/// H1's data: the service state and, per probed port, whether the remote probe's
/// `udp<port>=<count>` shows it listening: one or two sockets on the game port, one on the A2S
/// port.
fn service_and_listeners(remote: &str, listener_ports: Option<[u16; 2]>) -> Value {
    let mut row = serde_json::Map::new();
    row.insert("raw".into(), json!(remote));
    row.insert(
        "service_active".into(),
        json!(remote.contains("service=active")),
    );
    if let Some([game, a2s]) = listener_ports {
        let counted = |port: u16, count: u8| remote.contains(&format!("udp{port}={count}"));
        row.insert(
            format!("udp_{game}"),
            json!(counted(game, 1) || counted(game, 2)),
        );
        row.insert(format!("udp_{a2s}"), json!(counted(a2s, 1)));
    }
    Value::Object(row)
}

fn append(log: &Path, run_id: &str, hid: &str, message: &str, data: Value) -> Result<()> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let row = json!({
        "sessionId": "8fc1e0",
        "timestamp": ts,
        "location": "cargo xtask debug direct-join",
        "message": message,
        "data": data,
        "hypothesisId": hid,
        "runId": run_id,
    });
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .with_context(|| format!("open {}", log.display()))?;
    writeln!(f, "{}", serde_json::to_string(&row)?)?;
    Ok(())
}
