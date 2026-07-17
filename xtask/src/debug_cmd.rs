//! Debug helpers for scripts/mod/debug-direct-join.sh (T-162).

use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::io::Write;
use std::net::UdpSocket;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// UDP Source Engine Query probe → JSON on stdout.
pub fn cmd_a2s_probe(host: &str, ports: &[u16]) -> Result<()> {
    let mut out = serde_json::Map::new();
    for &port in ports {
        let key = format!("p{port}");
        out.insert(key, probe_one(host, port));
    }
    println!("{}", Value::Object(out));
    Ok(())
}

fn probe_one(host: &str, port: u16) -> Value {
    let sock = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => {
            return json!({"port": port, "ok": false, "error": e.to_string()});
        }
    };
    if let Err(e) = sock.set_read_timeout(Some(Duration::from_secs(2))) {
        return json!({"port": port, "ok": false, "error": e.to_string()});
    }
    // A2S_INFO / TSource Engine Query
    let payload: &[u8] = b"\xFF\xFF\xFF\xFFTSource Engine Query\x00";
    let addr = format!("{host}:{port}");
    if let Err(e) = sock.send_to(payload, &addr) {
        return json!({"port": port, "ok": false, "error": e.to_string()});
    }
    let mut buf = [0u8; 4096];
    match sock.recv_from(&mut buf) {
        Ok((n, from)) => json!({
            "port": port,
            "ok": true,
            "bytes": n,
            "from": from.to_string(),
        }),
        Err(e) => json!({"port": port, "ok": false, "error": e.to_string()}),
    }
}

/// Append one NDJSON debug line (replaces python log_json helper).
pub fn cmd_ndjson_append(
    log: &Path,
    hypothesis_id: &str,
    message: &str,
    data_json: &str,
    run_id: &str,
) -> Result<()> {
    let data: Value = serde_json::from_str(data_json).unwrap_or(json!({}));
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let row = json!({
        "sessionId": "8fc1e0",
        "timestamp": ts,
        "location": "scripts/debug-direct-join.sh",
        "message": message,
        "data": data,
        "hypothesisId": hypothesis_id,
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

/// Write the full direct-join debug block (H1–H6) that the Python heredoc produced.
#[allow(clippy::too_many_arguments)]
pub fn cmd_direct_join_log(
    log: &Path,
    run_id: &str,
    remote: &str,
    client_build: &str,
    server_build: &str,
    symlink: &str,
    ping: &str,
    a2s_json: &str,
) -> Result<()> {
    let a2s: Value = serde_json::from_str(a2s_json).unwrap_or(json!({}));
    let remote = remote.trim();
    append(
        log,
        run_id,
        "H1",
        "remote service and ports",
        json!({
            "raw": remote,
            "service_active": remote.contains("service=active"),
            "udp_2001": remote.contains("udp2001=1") || remote.contains("udp2001=2"),
            "udp_17777": remote.contains("udp17777=1"),
        }),
    )?;
    append(
        log,
        run_id,
        "H2",
        "steam build ids",
        json!({
            "client_build": client_build,
            "server_build": server_build,
            "builds_match": client_build == server_build && client_build != "unknown",
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
        json!({"ms": ping, "host": "192.168.0.140"}),
    )?;
    append(
        log,
        run_id,
        "H5",
        "client mod symlink",
        json!({"path": symlink, "exists": symlink != "missing"}),
    )?;
    append(log, run_id, "H6", "a2s port probe from client PC", a2s)?;
    Ok(())
}

fn append(log: &Path, run_id: &str, hid: &str, message: &str, data: Value) -> Result<()> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let row = json!({
        "sessionId": "8fc1e0",
        "timestamp": ts,
        "location": "scripts/debug-direct-join.sh",
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
