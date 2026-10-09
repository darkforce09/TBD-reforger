//! `cargo xtask mcp wbcall <APIFunc> ['<json>']` — a raw Workbench NET API call.
//!
//! **Role:** encodes one NET API request, sends it over a fresh TCP connection and decodes the
//! Workbench's answer; the command entry prints it as pretty JSON.
//! **Position:** called by [`crate::dispatch`] for `wbcall`; talks to the Workbench's NET API
//! port directly, beside the `enfusion-mcp` server rather than through it.
//! **Signals & state:** none; one connection per call, closed when the call returns.
//! **Invariants:** a request is the protocol version, the client id, the content type and the
//! JSON payload, each string length-prefixed; a status other than `Ok` is an error, never data.
//!
//! The enfusion-mcp server exposes typed tools but no generic NetApiHandler
//! bridge, and there is no Node bridge beside it; this
//! is its Rust port so the blueprint pipeline (`EMCP_WB_TbdBlueprint`: `recon`, `parity`,
//! `extract`, `probe`, `dump`) can be driven from xtask.
//!
//! Wire protocol (one fresh TCP connection per call, `dist/workbench/protocol.js`):
//! request = `i32 LE 1` · pascal(clientId) · pascal("JsonRPC") · pascal(JSON with `APIFunc`);
//! pascal = `i32 LE len` + UTF-8. Response = pascal(status) · pascal(JSON) — `status == "Ok"`
//! or an error message. Host/port from `ENFUSION_WORKBENCH_HOST` / `ENFUSION_WORKBENCH_PORT`
//! (default `127.0.0.1:5775`, the MCP server's defaults).
//!
//! Exit: 0 ok (JSON on stdout) · 1 usage · 2 cannot connect · 3 Workbench error.

use std::io::{Read as _, Write as _};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde_json::{Value, json};

use crate::error::{Error, Result};

const PROTOCOL_VERSION: i32 = 1;
const CONTENT_TYPE: &str = "JsonRPC";
const CLIENT_ID: &str = "TbdXtask";

fn pascal(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as i32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// Encode one request.
pub(crate) fn encode_request(client_id: &str, api_func: &str, params: &Value) -> Vec<u8> {
    let mut payload = params.clone();
    if !payload.is_object() {
        payload = json!({});
    }
    payload["APIFunc"] = Value::String(api_func.to_string());
    let mut out = Vec::new();
    out.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
    pascal(&mut out, client_id);
    pascal(&mut out, CONTENT_TYPE);
    pascal(&mut out, &payload.to_string());
    out
}

fn read_pascal(buf: &[u8], at: usize) -> Result<(String, usize)> {
    if buf.len() < at + 4 {
        return Err(Error::ResponseTooShort { at });
    }
    let len = i32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]]);
    if len < 0 || buf.len() < at + 4 + len as usize {
        return Err(Error::ResponseStringOverrun {
            length: len,
            at,
            total: buf.len(),
        });
    }
    let s = String::from_utf8_lossy(&buf[at + 4..at + 4 + len as usize]).into_owned();
    Ok((s, at + 4 + len as usize))
}

/// Decode one response: `Ok(json)` or the Workbench's error status.
pub(crate) fn decode_response(buf: &[u8]) -> Result<Value> {
    if buf.is_empty() {
        return Ok(json!({}));
    }
    let (status, next) = read_pascal(buf, 0)?;
    if status != "Ok" {
        return Err(Error::Workbench { status });
    }
    if buf.len() > next {
        let (payload, _) = read_pascal(buf, next)?;
        if !payload.is_empty() {
            return serde_json::from_str(&payload).map_err(|source| Error::ResponseJson {
                excerpt: payload[..payload.len().min(200)].to_string(),
                source,
            });
        }
    }
    Ok(json!({}))
}

/// Default endpoint (env overrides).
pub(crate) fn endpoint() -> (String, u16) {
    let host = std::env::var("ENFUSION_WORKBENCH_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("ENFUSION_WORKBENCH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(5775);
    (host, port)
}

/// One call: connect, send, half-close, read to EOF, decode.
pub(crate) fn wbcall(
    host: &str,
    port: u16,
    api_func: &str,
    params: &Value,
    timeout: Duration,
) -> Result<Value> {
    let endpoint = format!("{host}:{port}");
    let addr = (host, port)
        .to_socket_addrs()
        .map_err(|source| Error::Resolve {
            endpoint: endpoint.clone(),
            source,
        })?
        .next()
        .ok_or_else(|| Error::NoAddress {
            endpoint: endpoint.clone(),
        })?;
    let mut stream =
        TcpStream::connect_timeout(&addr, timeout).map_err(|source| Error::Connect {
            endpoint: endpoint.clone(),
            source,
        })?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    stream.write_all(&encode_request(CLIENT_ID, api_func, params))?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .map_err(|source| Error::ReadResponse {
            api_func: api_func.to_string(),
            source,
        })?;
    decode_response(&buf)
}

/// CLI entry.
pub(crate) fn cmd(api_func: Option<&str>, args_json: Option<&str>, timeout_s: u64) -> i32 {
    let Some(api_func) = api_func.filter(|s| !s.is_empty()) else {
        eprintln!("usage: cargo xtask mcp wbcall <APIFunc> ['<json object>'] [--timeout <s>]");
        return 1;
    };
    let params: Value = match args_json.filter(|s| !s.trim().is_empty()) {
        Some(s) => match serde_json::from_str(s) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("wbcall: args are not JSON: {e}");
                return 1;
            }
        },
        None => json!({}),
    };
    let (host, port) = endpoint();
    match wbcall(
        &host,
        port,
        api_func,
        &params,
        Duration::from_secs(timeout_s.max(1)),
    ) {
        Ok(v) => {
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            0
        }
        Err(e) => {
            // Every variant prints its cause after `: `, so this is the whole chain.
            let msg = e.to_string();
            eprintln!("wbcall {api_func}: {msg}");
            if msg.contains("connect to Workbench") {
                2
            } else {
                3
            }
        }
    }
}
