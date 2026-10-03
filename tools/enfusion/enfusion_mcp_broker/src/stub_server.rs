//! The offline stub `mcpd` runs instead of the broker.
//!
//! **Role:** emulates the `enfusion-mcp` server's newline-JSON stdout without a Workbench, for
//! `cargo xtask mcp selftest`: one-shot (an initialise answer, one tool answer, then a linger) or
//! as a request/response server answering each request by id.
//! **Position:** chosen by [`crate::command_entry::run`] for `--stub` or `MCP_STUB=1` without
//! `--socket`; the selftest points `ENFUSION_MCP_BIN` at the `mcpd` binary so the one-shot call
//! path and the broker both start it as their server.
//! **Signals & state:** none; reads stdin and the variables below, writes stdout and one stderr
//! marker line.
//! **Invariants:** a closed stdout ends the stub with exit 0; the stub never starts a broker, so
//! broker-under-broker recursion cannot happen.
//!
//! ```text
//!   STUB_MODE    success | error | empty | initfail   (default success)
//!   STUB_DAEMON  1 → request/response server (replies to each id); else one-shot + linger
//!   STUB_LINGER  seconds to stay alive in one-shot mode (default 1)
//! ```

use std::io::Write as _;
use std::process::ExitCode;
use std::time::Duration;

use serde_json::{Value, json};

/// Emulates the server on this process's stdio and returns the stub's exit code.
pub(crate) fn run_stub() -> ExitCode {
    let mode = std::env::var("STUB_MODE").unwrap_or_else(|_| "success".into());
    let linger = std::env::var("STUB_LINGER")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(1.0)
        .max(0.0);
    // `false` when stdout is closed (the early-exit consumer lane): the stub then ends cleanly.
    let emit = |obj: Value| -> bool {
        let mut out = std::io::stdout().lock();
        if writeln!(out, "{obj}").is_err() {
            return false;
        }
        let _ = out.flush();
        true
    };
    eprintln!("STUB-STDERR-MARKER mode={mode}");

    if std::env::var("STUB_DAEMON").as_deref() == Ok("1") {
        // Request/response server: reply to each request with a matching id.
        let stdin = std::io::stdin();
        for line in std::io::BufRead::lines(stdin.lock()) {
            let Ok(line) = line else { break };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(o) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let answer = match o["method"].as_str() {
                Some("initialize") => json!({
                    "jsonrpc": "2.0", "id": o["id"],
                    "result": { "protocolVersion": "2024-11-05", "capabilities": {},
                                "serverInfo": { "name": "stub", "version": "0" } }
                }),
                Some("tools/call") => {
                    let name = o["params"]["name"].as_str().unwrap_or_default().to_string();
                    let args = o["params"]["arguments"].clone();
                    let args = if args.is_null() { json!({}) } else { args };
                    if mode == "error" {
                        json!({
                            "jsonrpc": "2.0", "id": o["id"],
                            "error": { "code": -32601, "message": format!("Stub error: {name}") }
                        })
                    } else {
                        json!({
                            "jsonrpc": "2.0", "id": o["id"],
                            "result": { "content": [{ "type": "text",
                                "text": format!("STUB-DAEMON-OK {name} args={args}") }] }
                        })
                    }
                }
                _ => continue, // notifications/* ignored
            };
            if !emit(answer) {
                return ExitCode::SUCCESS;
            }
        }
        ExitCode::SUCCESS
    } else {
        // One-shot mode: drain stdin in the background, emit once, linger like the real server.
        std::thread::spawn(|| {
            let mut sink = Vec::new();
            let _ = std::io::Read::read_to_end(&mut std::io::stdin().lock(), &mut sink);
        });
        if mode != "initfail"
            && !emit(json!({
                "jsonrpc": "2.0", "id": 1,
                "result": { "protocolVersion": "2024-11-05", "capabilities": {},
                            "serverInfo": { "name": "stub", "version": "0" } }
            }))
        {
            return ExitCode::SUCCESS;
        }
        let answer = if mode == "success" {
            Some(json!({
                "jsonrpc": "2.0", "id": 2,
                "result": { "content": [{ "type": "text", "text": "STUB-OK wb_state edit 123" }] }
            }))
        } else if mode == "error" {
            Some(json!({
                "jsonrpc": "2.0", "id": 2,
                "error": { "code": -32601, "message": "Stub error: unknown tool" }
            }))
        } else {
            None
        };
        if let Some(answer) = answer
            && !emit(answer)
        {
            return ExitCode::SUCCESS;
        }
        std::thread::sleep(Duration::from_secs_f64(linger));
        ExitCode::SUCCESS
    }
}
