//! Server-Sent Events consumer — the useServerTelemetry.ts port (T-159.25). Same transport as
//! React: a Bearer-authenticated `fetch` + ReadableStream reader (NOT `EventSource`, which can't
//! carry the Authorization header), frames split on `\n\n`, `data:` lines JSON-parsed.
//!
//! Byte handling: chunks accumulate in a `Vec<u8>` and frames are parsed per-frame with
//! `from_utf8_lossy` — no `TextDecoder`, and a multi-byte codepoint split across reads can only
//! land inside one frame, never across the `\n\n` boundary the splitter keys on.
//!
//! # Where frames come from (T-272)
//!
//! The stream handler may emit one snapshot row on connect, then every later frame is a hub
//! publish. Producers:
//! 1. **Ingest** — `POST /ingest/server-status` → `publish_server_status` (immediate).
//! 2. **Scheduled republish** — boot + interval poll of `server_statuses` (env
//!    `SERVER_STATUS_PUBLISH_INTERVAL_SECS`, default 10s) so the pipe stays live without a
//!    game-server bridge. Same JSON shape; decode is unchanged ([`decode_server_status_frame`]).
//!
//! Measured 2026-07-27: initial-snapshot + DTO decode are sound (R-api `LIVE_SSE_FRAME` pin).
//! The pre-T-272 defect was zero *producers* after connect, not a client decode bug.
//!
//! # Lifetime (T-287)
//!
//! The fetch is aborted on SPA route-leave / component dispose. `AbortController` is `!Send`, so
//! it cannot live inside Leptos `on_cleanup` (which is `Send + Sync`-bound). Same workaround as
//! T-189's unload guard in `mission_history.rs`: park the controller in a `thread_local`, and
//! register the zero-capture [`abort_server_status_stream`] under the page's `on_cleanup` (see
//! `server_intel.rs`). Wasm is single-threaded, so the cleanup always runs on the thread that
//! installed the controller. Result: one page = one live stream; navigation tears it down.
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
//! placement is deliberate: **the wasm transport body is `#[cfg(target_arch = "wasm32")]`, so a
//! `#[cfg(test)]` that only lived behind that gate would never be compiled by native `cargo test`.**
//! The Class-R source guard below is therefore ungated (it `include_str!`s this file), and the pure
//! decode half sits in the natively-compiled wire-contract module beside the golden that pins it.

/// Contract name of the Send+Sync teardown entry point. Class-R / source guards pin this literal
/// so a future edit cannot rename the cleanup fn without updating the proof.
#[allow(dead_code)] // read by `class_r_sse_abort_teardown_exists`; not a runtime path
pub const SSE_ABORT_CLEANUP_FN: &str = "abort_server_status_stream";

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

#[cfg(target_arch = "wasm32")]
use crate::auth::AuthStore;
#[cfg(target_arch = "wasm32")]
use crate::dto::{decode_server_status_frame, ServerStatusDto, SseFrame};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// Live `AbortController` for the Server Intel SSE fetch. Parked here instead of inside
    /// `on_cleanup` because the controller is `!Send` while cleanup is `Send + Sync`-bound.
    /// Reached only via [`abort_server_status_stream`] (a zero-capture fn item).
    static SSE_ABORT: RefCell<Option<web_sys::AbortController>> = const { RefCell::new(None) };
}

/// Abort the live `/servers/:id/status/stream` fetch, if any. Zero-capture (plain `fn` item:
/// `Send + Sync + 'static`) so it is `on_cleanup`-compatible. Idempotent.
///
/// Callers: `ServerIntelInner`'s `on_cleanup` (route-leave), and [`stream_server_status`] before
/// arming a replacement controller (re-subscribe / remount).
pub fn abort_server_status_stream() {
    #[cfg(target_arch = "wasm32")]
    {
        let taken = SSE_ABORT.with(|c| c.borrow_mut().take());
        if let Some(ctrl) = taken {
            ctrl.abort();
        }
    }
}

/// Subscribe to `/servers/:id/status/stream`; the latest status/connected/error land in the given
/// signals (the React hook's return triple). Pairs with [`abort_server_status_stream`] under the
/// host page's `on_cleanup`.
#[cfg(target_arch = "wasm32")]
pub fn stream_server_status(
    store: AuthStore,
    server_id: String,
    status: RwSignal<Option<ServerStatusDto>>,
    connected: RwSignal<bool>,
    error: RwSignal<Option<String>>,
) {
    // Replace any prior controller (remount / re-subscribe) before arming a new one.
    abort_server_status_stream();

    let Ok(controller) = web_sys::AbortController::new() else {
        error.set(Some("SSE abort controller failed".into()));
        return;
    };
    let signal = controller.signal();
    SSE_ABORT.with(|c| *c.borrow_mut() = Some(controller));

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
            // T-287 — abort signal: route-leave calls [`abort_server_status_stream`], which
            // rejects this fetch / errors the body reader so the loop exits.
            init.set_signal(Some(&signal));
            let req =
                web_sys::Request::new_with_str_and_init(&url, &init).map_err(|_| "request")?;
            let win = web_sys::window().ok_or("window")?;
            let resp: web_sys::Response =
                wasm_bindgen_futures::JsFuture::from(win.fetch_with_request(&req))
                    .await
                    .map_err(|_| if signal.aborted() { "aborted" } else { "fetch" })?
                    .dyn_into()
                    .map_err(|_| "response")?;
            if signal.aborted() {
                return Err("aborted");
            }
            if !resp.ok() {
                return Err("SSE connection failed");
            }
            let body = resp.body().ok_or("SSE connection failed")?;
            let reader: web_sys::ReadableStreamDefaultReader = body.get_reader().unchecked_into();
            connected.set(true);
            error.set(None);
            let mut buf: Vec<u8> = Vec::new();
            loop {
                if signal.aborted() {
                    return Err("aborted");
                }
                let chunk = wasm_bindgen_futures::JsFuture::from(reader.read())
                    .await
                    .map_err(|_| if signal.aborted() { "aborted" } else { "read" })?;
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
        match run.await {
            Err("aborted") => {
                // Route-leave / remount — not a user-visible failure.
                connected.set(false);
            }
            Err(e) => {
                error.set(Some(e.to_string()));
                connected.set(false);
            }
            Ok(()) => {
                connected.set(false);
            }
        }
        // Leave the TLS slot alone on natural EOF/error: a later `stream_server_status` or
        // `abort_server_status_stream` `take()`s it. Clearing here races a remount that already
        // parked a newer controller under the same slot.
    });
}

#[cfg(test)]
mod tests {
    /// T-287 Class-R — the SSE fetch must abort on dispose; a comment-only "fixed" is a fail.
    #[test]
    fn class_r_sse_abort_teardown_exists() {
        const SRC: &str = include_str!("sse.rs");
        const INTEL: &str = include_str!("server_intel.rs");
        // Exclude this tests module from the negative asserts — the banned phrases appear in the
        // assert! messages themselves and would false-red the guard. Split on the mod marker, not
        // a bare `#[cfg(test)]` (that substring also appears in the module docs above).
        let production = SRC
            .split("mod tests {")
            .next()
            .expect("tests module marker");

        assert_eq!(
            super::SSE_ABORT_CLEANUP_FN,
            "abort_server_status_stream",
            "cleanup fn name pin drifted from the const"
        );
        // Native no-op call — proves the Send+Sync seam is reachable outside wasm.
        super::abort_server_status_stream();
        assert!(
            production.contains("AbortController"),
            "sse.rs must create an AbortController for the fetch"
        );
        assert!(
            production.contains("init.set_signal(Some(&signal))"),
            "RequestInit must wire the AbortController signal via init.set_signal(Some(&signal))"
        );
        assert!(
            production.contains("fn abort_server_status_stream"),
            "zero-capture abort entry point must exist for on_cleanup"
        );
        assert!(
            production.contains("SSE_ABORT"),
            "AbortController must be parked in a thread_local (Send workaround)"
        );
        assert!(
            !production.contains("NOT torn down on SPA nav"),
            "stale leak documentation must not remain after T-287"
        );
        assert!(
            !production.contains("navigation leaks at most one"),
            "stale leak documentation must not remain after T-287"
        );
        assert!(
            INTEL.contains("on_cleanup(crate::sse::abort_server_status_stream)")
                || INTEL.contains("on_cleanup(abort_server_status_stream)"),
            "ServerIntelInner must register abort_server_status_stream under on_cleanup"
        );
    }
}
