//! The live telemetry stream: one authenticated connection per page, torn down on leave.
//!
//! **Role:** opens the server-status event stream, splits it into frames, hands each frame to the
//! decoder, and pushes the result into the caller's signals.
//! **Position:** driven by the server intel page; nothing else opens a stream.
//! **Signals & state:** writes the caller's status, connected and error signals. Parks the live
//! abort handle in a thread-local so a `Send`-bound cleanup callback can reach it.
//! **Invariants:** the transport is a bearer-authenticated fetch over a readable stream rather than
//! the browser's own event-source object, which cannot carry an authorization header. Chunks
//! accumulate in a byte buffer and each frame is decoded on its own, so a multi-byte character
//! split across two reads can only land inside a frame, never across the blank-line boundary the
//! splitter keys on. One page means one live stream: arming a new controller aborts the previous
//! one, and route-leave aborts the current one.
//!
//! A frame the decoder cannot read is audited rather than swallowed. Dropping it silently is how a
//! type mismatch survived unnoticed for a month while the page showed a dead server and the backend
//! was sending healthy frames. One bad frame must not tear down a live feed, so the handling stays
//! best effort — but it leaves a console trail and raises the error signal.
//!
//! Decoding lives in the wire-type module rather than here, deliberately: the transport body below
//! is browser-only, so a test module behind that gate would never be compiled by the native test
//! run. The decoder sits beside the captured frame that pins it, where the native suite can reach
//! both.

/// Name of the teardown entry point, as a literal.
///
/// The guard test below pins this string, so the cleanup function cannot be renamed without the
/// proof being updated alongside it.
#[allow(dead_code)]
pub const SSE_ABORT_CLEANUP_FN: &str = "abort_server_status_stream";

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

#[cfg(target_arch = "wasm32")]
use crate::v2::core::api::dto::{decode_server_status_frame, ServerStatusDto, SseFrame};
#[cfg(target_arch = "wasm32")]
use crate::v2::core::auth::AuthStore;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// The live abort handle for the open stream.
    ///
    /// Parked here rather than captured by the page's cleanup callback because the handle is not
    /// `Send` while the cleanup slot requires `Send + Sync`. Reached only through
    /// [`abort_server_status_stream`], which captures nothing.
    static SSE_ABORT: RefCell<Option<web_sys::AbortController>> = const { RefCell::new(None) };
}

/// Abort the open stream, if there is one. Idempotent, and safe to call on a build with no
/// browser at all.
///
/// A plain function item that captures nothing, which is what makes it `Send + Sync + 'static` and
/// therefore usable as a page's cleanup callback. Called on route-leave, and by
/// [`stream_server_status`] before it arms a replacement.
pub fn abort_server_status_stream() {
    #[cfg(target_arch = "wasm32")]
    {
        let taken = SSE_ABORT.with(|c| c.borrow_mut().take());
        if let Some(ctrl) = taken {
            ctrl.abort();
        }
    }
}

/// Open the status stream for one server and drive the given signals from it.
///
/// The latest decoded status, whether the connection is up, and the most recent decode failure
/// each land in their signal. Pair this with [`abort_server_status_stream`] under the host page's
/// cleanup, or the connection outlives the page.
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
            // The abort signal: route-leave calls [`abort_server_status_stream`], which
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
                // Split off the complete frames; the partial tail stays buffered.
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
                            let msg = crate::v2::core::api::dto::audit_rejected_frame(
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
        // Leave the thread-local slot alone on a natural end of stream or error: a later call or
        // `abort_server_status_stream` `take`s it. Clearing here races a remount that already
        // parked a newer controller under the same slot.
    });
}

#[cfg(test)]
#[path = "tests/sse.rs"]
mod tests;
