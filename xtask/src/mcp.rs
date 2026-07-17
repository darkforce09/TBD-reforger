//! MCP JSON-RPC helpers (T-162) — formerly scripts/mod/lib/mcp-*.py

use serde_json::{Value, json};
use std::io::{self, BufRead, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// Exit codes match mcp-consume.py (locked by mcp-call-selftest).
pub fn cmd_consume() -> i32 {
    let stdin = io::stdin();
    let mut saw_init = false;
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            continue;
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(obj) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(map) = obj.as_object() else {
            continue;
        };

        let rpc_id = map.get("id");
        if rpc_id_eq(rpc_id, 1) {
            saw_init = true;
            continue;
        }
        if rpc_id_eq(rpc_id, 2) {
            if let Some(err) = map.get("error") {
                let _ = writeln!(io::stderr(), "{}", compact_json(err));
                return 3;
            }
            let result = map.get("result").cloned().unwrap_or(json!({}));
            if let Some(obj) = result.as_object() {
                if obj.get("isError") == Some(&Value::Bool(true)) {
                    let texts: Vec<String> = obj
                        .get("content")
                        .and_then(|c| c.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|c| {
                                    let o = c.as_object()?;
                                    if o.get("type").and_then(|t| t.as_str()) == Some("text") {
                                        Some(
                                            o.get("text")
                                                .and_then(|t| t.as_str())
                                                .unwrap_or("")
                                                .to_string(),
                                        )
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    let msg = if texts.is_empty() {
                        compact_json(&result)
                    } else {
                        texts.join("\n")
                    };
                    let _ = writeln!(io::stderr(), "{msg}");
                    return 3;
                }
            }
            let mut printed = false;
            if let Some(arr) = result.get("content").and_then(|c| c.as_array()) {
                for chunk in arr {
                    if let Some(o) = chunk.as_object() {
                        if o.get("type").and_then(|t| t.as_str()) == Some("text") {
                            let text = o.get("text").and_then(|t| t.as_str()).unwrap_or("");
                            if write_stdout_line(text) != 0 {
                                return 0; // BrokenPipe → 0 (Python)
                            }
                            printed = true;
                        }
                    }
                }
            }
            if !printed {
                let pretty = serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".into());
                if write_stdout_line(&pretty) != 0 {
                    return 0;
                }
            }
            let _ = io::stdout().flush();
            return 0;
        }
    }
    if saw_init { 1 } else { 2 }
}

/// Exit 0 = got response line; 7 = daemon unavailable (Python contract).
pub fn cmd_socket_send(sock_path: &str, tool: &str, args_json: &str) -> i32 {
    let args: Value = serde_json::from_str(args_json).unwrap_or(json!({}));
    let timeout_s: f64 = std::env::var("MCP_CALL_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(180.0);
    let timeout = Duration::from_secs_f64(timeout_s.max(0.001));

    let req = json!({"tool": tool, "args": args});
    let line = match serde_json::to_string(&req) {
        Ok(s) => format!("{s}\n"),
        Err(e) => {
            let _ = writeln!(io::stderr(), "mcp-socket-send: {e}");
            return 7;
        }
    };

    let mut stream = match UnixStream::connect(Path::new(sock_path)) {
        Ok(s) => s,
        Err(e) => {
            let _ = writeln!(io::stderr(), "mcp-socket-send: {e}");
            return 7;
        }
    };
    if let Err(e) = stream.set_read_timeout(Some(timeout)) {
        let _ = writeln!(io::stderr(), "mcp-socket-send: {e}");
        return 7;
    }
    if let Err(e) = stream.set_write_timeout(Some(timeout)) {
        let _ = writeln!(io::stderr(), "mcp-socket-send: {e}");
        return 7;
    }
    if let Err(e) = stream.write_all(line.as_bytes()) {
        let _ = writeln!(io::stderr(), "mcp-socket-send: {e}");
        return 7;
    }

    let mut buf = Vec::new();
    let mut tmp = [0u8; 65536];
    loop {
        if buf.contains(&b'\n') {
            break;
        }
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(e) => {
                let _ = writeln!(io::stderr(), "mcp-socket-send: {e}");
                return 7;
            }
        }
    }

    if buf.iter().all(|b| b.is_ascii_whitespace()) {
        return 7;
    }
    let out = String::from_utf8_lossy(&buf);
    if write_stdout_raw(&out) != 0 {
        return 0;
    }
    let _ = io::stdout().flush();
    0
}

/// Exit 0 if AF_UNIX connect succeeds within 2s; else 1 (mcp-daemon is_running).
pub fn cmd_probe_sock(sock_path: &str) -> i32 {
    match UnixStream::connect(Path::new(sock_path)) {
        Ok(stream) => {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
            0
        }
        Err(_) => 1,
    }
}

fn rpc_id_eq(v: Option<&Value>, n: i64) -> bool {
    match v {
        Some(Value::Number(num)) => num.as_i64() == Some(n) || num.as_u64() == Some(n as u64),
        Some(Value::String(s)) => s.parse::<i64>().ok() == Some(n),
        _ => false,
    }
}

fn compact_json(v: &Value) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "{}".into())
}

fn write_stdout_line(s: &str) -> i32 {
    match writeln!(io::stdout(), "{s}") {
        Ok(()) => 0,
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => 1, // signal caller → exit 0
        Err(_) => 1,
    }
}

fn write_stdout_raw(s: &str) -> i32 {
    match write!(io::stdout(), "{s}") {
        Ok(()) => 0,
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => 1,
        Err(_) => 1,
    }
}
