//! `cargo xtask mcp wbcall <APIFunc> ['<json>']` — a raw Workbench NET API call
//! (T-090.11.3). The enfusion-mcp server exposes typed tools but no generic NetApiHandler
//! bridge, and the Node `wbcall.mjs` of the T-090.6 sessions was eradicated with T-165; this
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

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

const PROTOCOL_VERSION: i32 = 1;
const CONTENT_TYPE: &str = "JsonRPC";
const CLIENT_ID: &str = "TbdXtask";

fn pascal(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as i32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// Encode one request.
pub fn encode_request(client_id: &str, api_func: &str, params: &Value) -> Vec<u8> {
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
        bail!("response too short for a string length at {at}");
    }
    let len = i32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]]);
    if len < 0 || buf.len() < at + 4 + len as usize {
        bail!(
            "response string length {len} at {at} exceeds {} bytes",
            buf.len()
        );
    }
    let s = String::from_utf8_lossy(&buf[at + 4..at + 4 + len as usize]).into_owned();
    Ok((s, at + 4 + len as usize))
}

/// Decode one response: `Ok(json)` or the Workbench's error status.
pub fn decode_response(buf: &[u8]) -> Result<Value> {
    if buf.is_empty() {
        return Ok(json!({}));
    }
    let (status, next) = read_pascal(buf, 0)?;
    if status != "Ok" {
        bail!("Workbench error: {status}");
    }
    if buf.len() > next {
        let (payload, _) = read_pascal(buf, next)?;
        if !payload.is_empty() {
            return serde_json::from_str(&payload)
                .with_context(|| format!("response JSON: {}", &payload[..payload.len().min(200)]));
        }
    }
    Ok(json!({}))
}

/// Default endpoint (env overrides).
pub fn endpoint() -> (String, u16) {
    let host = std::env::var("ENFUSION_WORKBENCH_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("ENFUSION_WORKBENCH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(5775);
    (host, port)
}

/// One call: connect, send, half-close, read to EOF, decode.
pub fn wbcall(
    host: &str,
    port: u16,
    api_func: &str,
    params: &Value,
    timeout: Duration,
) -> Result<Value> {
    let addr = (host, port)
        .to_socket_addrs()
        .with_context(|| format!("resolve {host}:{port}"))?
        .next()
        .with_context(|| format!("no address for {host}:{port}"))?;
    let mut stream = TcpStream::connect_timeout(&addr, timeout)
        .with_context(|| format!("connect to Workbench NET API at {host}:{port}"))?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    stream.write_all(&encode_request(CLIENT_ID, api_func, params))?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .with_context(|| format!("read response for {api_func}"))?;
    decode_response(&buf)
}

/// CLI entry.
pub fn cmd(api_func: Option<&str>, args_json: Option<&str>, timeout_s: u64) -> i32 {
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
            let msg = format!("{e:#}");
            eprintln!("wbcall {api_func}: {msg}");
            if msg.contains("connect to Workbench") {
                2
            } else {
                3
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_and_response_framing_round_trip() {
        let req = encode_request(
            "TbdXtask",
            "EMCP_WB_TbdBlueprint",
            &json!({"action": "recon", "maxEntities": 600}),
        );
        assert_eq!(&req[0..4], &1i32.to_le_bytes());
        let (client, next) = read_pascal(&req, 4).unwrap();
        assert_eq!(client, "TbdXtask");
        let (ct, next) = read_pascal(&req, next).unwrap();
        assert_eq!(ct, "JsonRPC");
        let (payload, end) = read_pascal(&req, next).unwrap();
        assert_eq!(end, req.len());
        let v: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(v["APIFunc"], "EMCP_WB_TbdBlueprint");
        assert_eq!(v["action"], "recon");
        assert_eq!(v["maxEntities"], 600);

        let mut ok = Vec::new();
        pascal(&mut ok, "Ok");
        pascal(&mut ok, r#"{"status":"ok","message":"OK 181 entities"}"#);
        let v = decode_response(&ok).unwrap();
        assert_eq!(v["message"], "OK 181 entities");
        let mut bare = Vec::new();
        pascal(&mut bare, "Ok");
        assert_eq!(decode_response(&bare).unwrap(), json!({}));
        assert_eq!(decode_response(&[]).unwrap(), json!({}));
        let mut err = Vec::new();
        pascal(&mut err, "Unknown APIFunc");
        assert!(
            decode_response(&err)
                .unwrap_err()
                .to_string()
                .contains("Unknown APIFunc")
        );
        assert!(decode_response(&[5, 0, 0, 0, b'O']).is_err());
    }
}
