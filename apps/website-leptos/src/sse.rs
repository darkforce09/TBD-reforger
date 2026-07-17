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
use crate::auth::AuthStore;
use crate::dto::ServerStatusDto;
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
                    let line = text.trim();
                    if let Some(data) = line.strip_prefix("data:") {
                        if let Ok(json) = serde_json::from_str::<ServerStatusDto>(data.trim()) {
                            status.set(Some(json));
                        }
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
