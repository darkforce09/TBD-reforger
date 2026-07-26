//! Server-Sent Events consumer — the useServerTelemetry.ts port (T-159.25). Same transport as
//! React: a Bearer-authenticated `fetch` + ReadableStream reader (NOT `EventSource`, which can't
//! carry the Authorization header), frames split on `\n\n`, `data:` lines JSON-parsed.
//!
//! Byte handling: chunks accumulate in a `Vec<u8>` and frames are parsed per-frame with
//! `from_utf8_lossy` — no `TextDecoder`, and a multi-byte codepoint split across reads can only
//! land inside one frame, never across the `\n\n` boundary the splitter keys on.
//!
//! Lifetime: like the editor's engine host, the stream is NOT torn down on SPA nav (leptos
//! `on_cleanup` is Send-bound, and the `AbortController` handle is `!Send`) — the connection ends
//! when the tab closes or the server drops it. One page = one stream; navigation leaks at most one
//! idle reader, the documented editor-host tradeoff.
//!
//! **T-306 — a rejected frame is audited, not swallowed.** The parse used to be
//! `if let Ok(json) = serde_json::from_str::<ServerStatusDto>(..)`, so a frame the DTO could not
//! read was dropped with no trace on a stream that had already reported `connected = true`. That is
//! how a `server_fps: i64`/`f64` mismatch survived a month: the page looked like a dead server while
//! the backend was sending complete, healthy frames. The frame is still best-effort — one bad frame
//! must not tear down a live feed — but it now leaves a `console.warn` audit trail and sets the
//! `error` signal, the same best-effort-with-audit shape T-316/T-326 used rather than either
//! propagating or dropping in silence.
//!
//! The decode itself lives in [`crate::dto`] ([`decode_server_status_frame`]), not here, and that
//! placement is deliberate: **this module is `#[cfg(target_arch = "wasm32")]` in `main.rs`, so a
//! `#[cfg(test)] mod` in this file would never be compiled, let alone run, by `cargo test`.** A
//! frame-decoding policy nobody can test is how the original defect stayed invisible, so the pure
//! half sits in the natively-compiled wire-contract module beside the golden that pins it, and this
//! file keeps only the `web_sys` transport.
use crate::auth::AuthStore;
use crate::dto::{decode_server_status_frame, ServerStatusDto, SseFrame};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Subscribe to `/servers/:id/status/stream`; the latest status/connected/error land in the given
/// signals (the React hook's return triple).
pub fn stream_server_status(
    store: AuthStore,
    server_id: String,
    status: RwSignal<Option<ServerStatusDto>>,
    connected: RwSignal<bool>,
    error: RwSignal<Option<String>>,
) {
    leptos::task::spawn_local(async move {
        let Some(token) = store.access_token.get_untracked() else {
            return;
        };
        let url = format!("/api/v1/servers/{server_id}/status/stream");
        let run = async {
            let headers = web_sys::Headers::new().map_err(|_| "headers")?;
            headers
                .set("Authorization", &format!("Bearer {token}"))
                .map_err(|_| "auth header")?;
            let init = web_sys::RequestInit::new();
            init.set_method("GET");
            init.set_headers(&headers);
            let req =
                web_sys::Request::new_with_str_and_init(&url, &init).map_err(|_| "request")?;
            let win = web_sys::window().ok_or("window")?;
            let resp: web_sys::Response =
                wasm_bindgen_futures::JsFuture::from(win.fetch_with_request(&req))
                    .await
                    .map_err(|_| "fetch")?
                    .dyn_into()
                    .map_err(|_| "response")?;
            if !resp.ok() {
                return Err("SSE connection failed");
            }
            let body = resp.body().ok_or("SSE connection failed")?;
            let reader: web_sys::ReadableStreamDefaultReader = body.get_reader().unchecked_into();
            connected.set(true);
            error.set(None);
            let mut buf: Vec<u8> = Vec::new();
            loop {
                let chunk = wasm_bindgen_futures::JsFuture::from(reader.read())
                    .await
                    .map_err(|_| "read")?;
                let done = js_sys::Reflect::get(&chunk, &"done".into())
                    .ok()
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                if done {
                    break;
                }
                if let Ok(value) = js_sys::Reflect::get(&chunk, &"value".into()) {
                    let arr: js_sys::Uint8Array = value.unchecked_into();
                    let mut bytes = vec![0u8; arr.length() as usize];
                    arr.copy_to(&mut bytes);
                    buf.extend_from_slice(&bytes);
                }
                // Split complete `\n\n` frames; the tail stays buffered (React's split/pop).
                while let Some(pos) = buf.windows(2).position(|w| w == b"\n\n") {
                    let frame: Vec<u8> = buf.drain(..pos + 2).collect();
                    let text = String::from_utf8_lossy(&frame);
                    match decode_server_status_frame(&text) {
                        SseFrame::Status(dto) => {
                            status.set(Some(*dto));
                            // A good frame clears a previous rejection, so a transient bad frame
                            // does not leave the panel permanently accusing the stream.
                            error.set(None);
                        }
                        SseFrame::Rejected {
                            error: e,
                            payload: p,
                        } => {
                            // Best-effort: keep reading, keep `connected` true (it *is* connected —
                            // claiming otherwise would be a second lie), but stop pretending nothing
                            // happened. `status` is deliberately left as-is rather than cleared: the
                            // last good frame is better intel than a blank panel.
                            let msg = crate::dto::audit_rejected_frame(
                                "sse stream_server_status",
                                &e,
                                &p,
                            );
                            error.set(Some(msg));
                        }
                        SseFrame::NotData => {}
                    }
                }
            }
            Ok(())
        };
        if let Err(e) = run.await {
            error.set(Some(e.to_string()));
            connected.set(false);
        }
    });
}
