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
//! The decode itself lives in [`crate::core::dto`] ([`decode_server_status_frame`]), not here, and that
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
use crate::core::auth::AuthStore;
#[cfg(target_arch = "wasm32")]
use crate::core::dto::{decode_server_status_frame, ServerStatusDto, SseFrame};
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
                            let msg = crate::core::dto::audit_rejected_frame(
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
    use crate::editor::arsenal::class_r_scrub::{live_code, only_body};

    /// T-287 Class-R — the SSE fetch must abort on dispose; a comment-only "fixed" is a fail.
    ///
    /// # Cure 2 (scrub-then-grep), and why not cure 1 (T-601)
    ///
    /// The invariant lives entirely inside `#[cfg(target_arch = "wasm32")]`
    /// [`stream_server_status`]: it is `web_sys::AbortController` + `RequestInit::set_signal` +
    /// a `thread_local` park. There is no runtime signature a native harness can observe without
    /// standing up a fake `web_sys` — cure 1 would be modelling the browser, and a pin that
    /// asserts against its own mock of `fetch` proves nothing about the real one. So this stays a
    /// source pin, scrubbed so that what it greps is provably shipped code.
    ///
    /// # What this pin was reporting on before T-601 — and it was its own prose
    ///
    /// `production` was `SRC.split("mod tests {").next()`: the raw file, **module documentation
    /// included**. Every positive needle below appears verbatim in the `//!` header above —
    /// "`AbortController` is `!Send`", "park the controller in a `thread_local`", "register the
    /// zero-capture [`abort_server_status_stream`]". Delete [`stream_server_status`] outright and
    /// this pin stayed green off the paragraph describing it. That is the signature defect exactly:
    /// a tool reporting success over an input it never examined.
    ///
    /// The positive asserts now run on [`live_code`] — comments and string literals blanked,
    /// `#[cfg(<false>)]` items and constant-false blocks removed — so a needle only counts as a
    /// call in code the build can reach. The **negative** asserts deliberately keep reading the raw
    /// file: they ban stale prose, and prose is precisely what a scrubber removes.
    #[test]
    fn class_r_sse_abort_teardown_exists() {
        const SRC: &str = include_str!("sse.rs");
        const INTEL: &str = include_str!("../pages/public/server_intel.rs");
        let production = live_code(SRC);
        // T-457 — the INTEL pin must ignore comments so commenting out the live
        // `on_cleanup(...abort_server_status_stream)` while leaving the string in a comment REDS.
        let intel_code = live_code(INTEL);

        assert_eq!(
            super::SSE_ABORT_CLEANUP_FN,
            "abort_server_status_stream",
            "cleanup fn name pin drifted from the const"
        );
        // Native no-op call — proves the Send+Sync seam is reachable outside wasm.
        super::abort_server_status_stream();

        // Scoped to the one function that owns the transport. A whole-file `contains` is satisfied
        // by a match ANYWHERE, including in a second, dead copy of the fetch — `only_body` refuses
        // two definitions rather than silently reading the first.
        let stream = only_body(&production, "pub fn stream_server_status(");
        assert!(
            stream.contains("AbortController"),
            "stream_server_status must create an AbortController for the fetch — on a live path, \
             not in the paragraph that explains why it needs one"
        );
        assert!(
            stream.contains("init.set_signal(Some(&signal))"),
            "RequestInit must wire the AbortController signal via init.set_signal(Some(&signal)); \
             an AbortController the fetch never receives aborts nothing"
        );
        assert!(
            stream.contains("SSE_ABORT.with("),
            "the controller must be parked in the thread_local (the !Send workaround), or \
             route-leave has nothing to take"
        );
        assert!(
            stream.contains("abort_server_status_stream()"),
            "a re-subscribe must abort the prior controller first, or a remount leaks a stream"
        );

        let abort = only_body(&production, "pub fn abort_server_status_stream()");
        assert!(
            abort.contains("SSE_ABORT.with(") && abort.contains(".abort()"),
            "the zero-capture entry point must actually take the parked controller and abort it"
        );

        // Negatives read the RAW production text on purpose: they ban stale *documentation*, and
        // documentation is the first thing the scrubber removes. The cut is the `mod tests {`
        // marker rather than a bare `#[cfg(test)]` (that substring also appears in the module docs
        // above) so this test's own assertion strings cannot satisfy the ban it is asserting.
        let prose = SRC
            .split("mod tests {")
            .next()
            .expect("tests module marker");
        assert!(
            !prose.contains("NOT torn down on SPA nav"),
            "stale leak documentation must not remain after T-287"
        );
        assert!(
            !prose.contains("navigation leaks at most one"),
            "stale leak documentation must not remain after T-287"
        );
        assert!(
            intel_code.contains("on_cleanup(crate::core::sse::abort_server_status_stream)")
                || intel_code.contains("on_cleanup(abort_server_status_stream)"),
            "ServerIntelInner must register abort_server_status_stream under on_cleanup \
             (live production line — comment-only string is not enough)"
        );
    }

    /// **The pin above, pinned.** Each attack is applied to a copy of this file's own source and
    /// the scrubbed result must no longer satisfy the needle — so a future edit that weakens the
    /// scrubbing shows up here rather than as a quiet green.
    ///
    /// This is the cheap generic form of the calibration `mission_title_prefer` gets for free by
    /// executing the code: prove the instrument can still say NO.
    #[test]
    fn the_teardown_pin_rejects_every_dead_code_wrapper() {
        let needle = "init.set_signal(Some(&signal))";
        let attacks: [(&str, String); 12] = [
            (
                "if true == false",
                format!("if true == false {{ {needle}; }}"),
            ),
            ("loop { break; … }", format!("loop {{ break; {needle}; }}")),
            (
                "#[cfg(any())]",
                format!("#[cfg(any())] fn d() {{ {needle}; }}"),
            ),
            ("while false", format!("while false {{ {needle}; }}")),
            ("if !true", format!("if !true {{ {needle}; }}")),
            ("if 1 > 2", format!("if 1 > 2 {{ {needle}; }}")),
            (
                "if std::hint::black_box(false)",
                format!("if std::hint::black_box(false) {{ {needle}; }}"),
            ),
            (
                "const C: bool = false; if C",
                format!("const C: bool = false;\nfn d() {{ if C {{ {needle}; }} }}"),
            ),
            ("return; above", format!("fn d() {{ return; {needle}; }}")),
            (
                "#[cfg(any())] mod shadow",
                format!("#[cfg(any())] mod shadow {{ fn d() {{ {needle}; }} }}"),
            ),
            (
                "match guard",
                format!("match () {{ _ if false => {{ {needle}; }} _ => {{}} }}"),
            ),
            ("comment", format!("// {needle}")),
        ];
        for (label, body) in attacks {
            let forged = format!("fn stream_server_status() {{\n    {body}\n}}\n#[cfg(test)]\n");
            assert!(
                !live_code(&forged).contains(needle),
                "{label}: the signal-wiring needle survived scrubbing, so this pin would report a \
                 live abort wire over code the build never runs"
            );
        }
        // The honest wiring still reads as present, or the assertions above pin nothing.
        let live = format!("fn stream_server_status() {{\n    {needle};\n}}\n#[cfg(test)]\n");
        assert!(live_code(&live).contains(needle));
    }
}
