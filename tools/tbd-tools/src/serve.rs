//! T-165.5 — static SPA server (port of `driver/serve.mjs`).
//!
//! Serves a built SPA with the SAME cross-origin-isolation headers the app expects
//! (`crossOriginIsolated === true` for the wasm/SAB path). Any path without a file extension
//! falls back to index.html (client routing). Optional same-origin `/api/` proxy (the Trunk
//! `[[proxy]]` equivalent) and `/map-assets/` passthrough to the real packages/map-assets.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri, header};
use axum::response::Response;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

fn mime_for(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "webp" => "image/webp",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ico" => "image/x-icon",
        "map" => "application/json",
        _ => return None,
    })
}

/// Strip leading `..` components (the serve.mjs traversal guard).
fn sanitize_rel(p: &str) -> PathBuf {
    PathBuf::from(p)
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .collect()
}

pub struct ServeConfig {
    pub dir: PathBuf,
    pub api_proxy: Option<String>,
    pub map_assets_dir: Option<PathBuf>,
}

struct AppState {
    cfg: ServeConfig,
    client: reqwest::Client,
}

pub struct RunningServer {
    pub port: u16,
    shutdown: Option<oneshot::Sender<()>>,
    handle: tokio::task::JoinHandle<()>,
}

/// How long [`RunningServer::close`] lets in-flight connections drain before it stops asking
/// nicely. Generous enough for any ordinary finite response to finish writing, short enough that
/// a gate step can never sit on it.
const SHUTDOWN_GRACE: std::time::Duration = std::time::Duration::from_secs(3);

impl RunningServer {
    /// Signal shutdown and wait — but only for [`SHUTDOWN_GRACE`], then abort.
    ///
    /// T-361: `axum::serve(..).with_graceful_shutdown(..)` resolves only once every in-flight
    /// connection has drained. Before this ticket that was safe, because the `/api` proxy
    /// buffered with `bytes().await` and so could not produce an unbounded response. Now that it
    /// streams, a still-open SSE subscription is a connection that **never** drains, and a plain
    /// `self.handle.await` blocks forever.
    ///
    /// This is not hypothetical and it bit here first: it hung `sse_frames_arrive_incrementally_
    /// not_buffered` in-process, and because the stuck server is the one `start_server` spawned
    /// *inside* the test, killing any external `gate serve` did not unwind it. Left unbounded it
    /// would be the worse half of this ticket's own defect — a gate step that hangs produces no
    /// verdict at all, which is strictly worse than a red one.
    pub async fn close(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if tokio::time::timeout(SHUTDOWN_GRACE, &mut self.handle)
            .await
            .is_err()
        {
            // A stream that will not end on its own. Drop it rather than wait on it.
            self.handle.abort();
        }
    }
}

fn base_headers(res: &mut Response) {
    let h = res.headers_mut();
    h.insert("Cross-Origin-Opener-Policy", "same-origin".parse().unwrap());
    h.insert(
        "Cross-Origin-Embedder-Policy",
        "credentialless".parse().unwrap(),
    );
    h.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
}

/// T-361 — upstream response headers that must NOT be copied onto a *streamed* proxy response.
///
/// Two distinct reasons, both fatal if ignored:
///  - **Framing** (`content-length`, `transfer-encoding`): we re-frame the body ourselves. hyper
///    picks the framing for the downstream connection from the body it is handed — an inherited
///    `content-length` from the upstream's *decoded* body, or the upstream's own
///    `transfer-encoding: chunked` (which the real `/status/stream` endpoint does send), would
///    describe a body we are no longer sending byte-for-byte. The client then truncates the
///    stream at the stale length, or tries to de-chunk already-de-chunked bytes and stalls.
///  - **Hop-by-hop** (RFC 9110 §7.6.1): `connection`, `keep-alive`, `proxy-authenticate`,
///    `proxy-authorization`, `te`, `trailer`, `upgrade` describe *that* TCP hop, not this one.
///
/// Everything else is forwarded verbatim — which is what makes the streaming path honest.
/// `content-type: text/event-stream` is what tells the browser to open an `EventSource` reader
/// instead of buffering to a string, `x-accel-buffering: no` is the upstream's explicit
/// "do not buffer me" instruction to intermediaries, and `content-encoding` must survive because
/// reqwest is built here without any decompression feature (`default-features = false`), so a
/// compressed upstream body reaches us — and must reach the client — still compressed.
fn is_hop_by_hop(k: &HeaderName) -> bool {
    matches!(
        k.as_str(),
        "content-length"
            | "transfer-encoding"
            | "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "upgrade"
    )
}

fn respond(status: StatusCode, content_type: Option<&str>, body: Vec<u8>) -> Response {
    let mut res = Response::builder().status(status);
    if let Some(ct) = content_type {
        res = res.header(header::CONTENT_TYPE, ct);
    }
    let mut res = res.body(Body::from(body)).unwrap();
    base_headers(&mut res);
    res
}

/// Parse `bytes=START-END` (END optional). Returns inclusive byte range clamped to `file_len`.
fn parse_bytes_range(h: &HeaderMap, file_len: u64) -> Option<(u64, u64)> {
    if file_len == 0 {
        return None;
    }
    let raw = h.get(header::RANGE)?.to_str().ok()?;
    let spec = raw.strip_prefix("bytes=")?;
    // Single range only (sat preview / TBDS tiles).
    let spec = spec.split(',').next()?.trim();
    let (start_s, end_s) = spec.split_once('-')?;
    if start_s.is_empty() {
        // suffix form `bytes=-N`
        let n: u64 = end_s.parse().ok()?;
        if n == 0 {
            return None;
        }
        let start = file_len.saturating_sub(n);
        return Some((start, file_len - 1));
    }
    let start: u64 = start_s.parse().ok()?;
    if start >= file_len {
        return None;
    }
    let end = if end_s.is_empty() {
        file_len - 1
    } else {
        end_s.parse::<u64>().ok()?.min(file_len - 1)
    };
    if end < start {
        return None;
    }
    Some((start, end))
}

/// Serve one map-asset file with optional HTTP Range (T-166). Full GET still reads the file;
/// Range uses seek + exact-length read so a 152_713_114 B sat bundle is never fully buffered.
async fn serve_map_asset(file: &Path, headers: &HeaderMap) -> Response {
    let meta = match tokio::fs::metadata(file).await {
        Ok(m) if m.is_file() => m,
        _ => return respond(StatusCode::NOT_FOUND, None, b"map-asset not found".to_vec()),
    };
    let file_len = meta.len();
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
    let ct = mime_for(ext).unwrap_or("application/octet-stream");

    if let Some((start, end)) = parse_bytes_range(headers, file_len) {
        let len = end - start + 1;
        let mut f = match tokio::fs::File::open(file).await {
            Ok(f) => f,
            Err(_) => {
                return respond(StatusCode::NOT_FOUND, None, b"map-asset not found".to_vec());
            }
        };
        if f.seek(std::io::SeekFrom::Start(start)).await.is_err() {
            return respond(
                StatusCode::INTERNAL_SERVER_ERROR,
                None,
                b"seek failed".to_vec(),
            );
        }
        let mut buf = vec![0u8; len as usize];
        if f.read_exact(&mut buf).await.is_err() {
            return respond(
                StatusCode::INTERNAL_SERVER_ERROR,
                None,
                b"range read failed".to_vec(),
            );
        }
        let mut res = Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_TYPE, ct)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(
                header::CONTENT_RANGE,
                format!("bytes {start}-{end}/{file_len}"),
            )
            .header(header::CONTENT_LENGTH, len)
            .body(Body::from(buf))
            .unwrap();
        base_headers(&mut res);
        return res;
    }

    match tokio::fs::read(file).await {
        Ok(buf) => {
            let mut res = respond(StatusCode::OK, Some(ct), buf);
            res.headers_mut()
                .insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());
            res
        }
        Err(_) => respond(StatusCode::NOT_FOUND, None, b"map-asset not found".to_vec()),
    }
}

async fn handler(
    State(state): State<Arc<AppState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let path = uri.path().to_string();

    // /map-assets/ passthrough (T-159.28). T-166: honor Range with seek+partial read → 206
    // so CI sat preview never loads the full 152_713_114 B `.tbd-sat` into RAM.
    if let Some(assets) = &state.cfg.map_assets_dir
        && let Some(rest) = path.strip_prefix("/map-assets/")
    {
        let decoded = percent_decode(rest);
        let file = assets.join(sanitize_rel(&decoded));
        return serve_map_asset(&file, &headers).await;
    }

    // Same-origin API proxy (T-159.25 equivalent).
    if let Some(proxy) = &state.cfg.api_proxy
        && path.starts_with("/api/")
    {
        let target = format!(
            "{proxy}{}",
            uri.path_and_query().map(|pq| pq.as_str()).unwrap_or(&path)
        );
        let mut req = state.client.request(method.clone(), &target);
        for (k, v) in headers.iter() {
            if k != header::HOST {
                req = req.header(k, v);
            }
        }
        if !matches!(method, Method::GET | Method::HEAD) {
            req = req.body(body.to_vec());
        }
        return match req.send().await {
            Ok(upstream) => {
                let status = StatusCode::from_u16(upstream.status().as_u16())
                    .unwrap_or(StatusCode::BAD_GATEWAY);
                let mut res = Response::builder().status(status);
                if let Some(h) = res.headers_mut() {
                    for (k, v) in upstream.headers() {
                        if !is_hop_by_hop(k) {
                            // `append`, not `insert`: repeated headers (set-cookie) must all survive.
                            h.append(k, v.clone());
                        }
                    }
                    // Preserve the pre-T-361 default for an upstream that sends no content-type.
                    if !h.contains_key(header::CONTENT_TYPE) {
                        h.insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("application/json"),
                        );
                    }
                }
                // T-361 — STREAM the body; do not `upstream.bytes().await`.
                //
                // `bytes()` resolves only at end-of-body, so an endless response (Server-Sent
                // Events, long-poll) parked this handler forever and the downstream request
                // never completed: the gate could not browser-test any SSE-driven page, and
                // T-306 had to hand-roll a shim that replayed captured frames plus a close.
                // Yielding each chunk as it lands makes arrival incremental — the first frame
                // reaches the client while the upstream stream is still open — which is the
                // whole point; a finite body still arrives in full, just without the
                // read-it-all-first hop.
                //
                // reqwest's `bytes_stream()` is gated behind its `stream` feature, which this
                // crate does not enable; `chunk()` is ungated and gives the same semantics, so
                // this needs no Cargo.toml change. `None` = clean end-of-body. On a transport
                // error we surface it once and then stop: the `Option` state is taken so the
                // stream terminates instead of re-polling a broken response forever.
                let body = Body::from_stream(futures_util::stream::unfold(
                    Some(upstream),
                    |state| async move {
                        let mut u = state?;
                        match u.chunk().await {
                            Ok(Some(chunk)) => Some((Ok(chunk), Some(u))),
                            Ok(None) => None,
                            Err(e) => Some((Err(e), None)),
                        }
                    },
                ));
                let mut res = res.body(body).unwrap();
                base_headers(&mut res);
                res
            }
            Err(e) => respond(
                StatusCode::BAD_GATEWAY,
                None,
                format!("proxy error: {e}").into_bytes(),
            ),
        };
    }

    // Static file with SPA fallback: no extension (a client route) → index.html.
    let decoded = percent_decode(&path);
    let rel = sanitize_rel(decoded.trim_start_matches('/'));
    let mut file = state.cfg.dir.join(&rel);
    let mut ext = file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    if ext.is_empty() || tokio::fs::metadata(&file).await.is_err() {
        file = state.cfg.dir.join("index.html");
        ext = "html".to_string();
    }
    match tokio::fs::read(&file).await {
        Ok(buf) => respond(
            StatusCode::OK,
            Some(mime_for(&ext).unwrap_or("application/octet-stream")),
            buf,
        ),
        Err(e) => respond(
            StatusCode::INTERNAL_SERVER_ERROR,
            None,
            format!("serve error: {e}").into_bytes(),
        ),
    }
}

fn percent_decode(s: &str) -> String {
    // Minimal %XX decoder (serve.mjs uses decodeURIComponent; asset paths here are ASCII).
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Bind and serve. `port = 0` picks an ephemeral port (returned in `RunningServer.port`).
pub async fn start_server(cfg: ServeConfig, port: u16) -> Result<RunningServer> {
    let state = Arc::new(AppState {
        cfg,
        client: reqwest::Client::new(),
    });
    let app = axum::Router::new().fallback(handler).with_state(state);
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let actual = listener.local_addr()?.port();
    let (tx, rx) = oneshot::channel::<()>();
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = rx.await;
            })
            .await;
    });
    Ok(RunningServer {
        port: actual,
        shutdown: Some(tx),
        handle,
    })
}

/// Resolve the repo root from CARGO_MANIFEST_DIR (tools/tbd-tools → ../..) or cwd.
pub fn repo_root() -> PathBuf {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// T-361 — the `/api` proxy must **stream**, not buffer.
///
/// These tests exist because the gate could not see something. `gate serve` used to do
/// `upstream.bytes().await`, which resolves only at end-of-body, so an endless response (SSE,
/// long-poll) parked the handler forever and the downstream request never completed. Measured
/// against the live `GET /api/v1/servers/{id}/status/stream`: 0 bytes delivered in 12 s through
/// the old proxy while the upstream emitted 4 lines in the same window.
///
/// The trap these tests are built to avoid: **a test that only checks the final assembled body
/// would pass under the old buffering code too, and would therefore prove nothing.** So
/// [`sse_frames_arrive_incrementally_not_buffered`] keys on ORDERING — it shows frame 1 is in
/// hand while frame 2 does not exist yet — which buffering cannot fake, because buffering has
/// exactly one delivery moment and it is at end-of-body.
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Gap between the fake upstream's two SSE frames. The incrementality assertion waits less
    /// than this for frame 2 and requires it *not* to show up.
    const FRAME_GAP_MS: u64 = 2000;
    /// Budget for frame 1. Streaming delivers it in single-digit ms; buffering never delivers it
    /// at all, so the exact value only has to sit comfortably below `FRAME_GAP_MS`.
    const FIRST_FRAME_BUDGET_MS: u64 = 1200;

    /// Hard in-test deadline. **Every** test below runs its whole body inside this.
    ///
    /// This is load-bearing, not belt-and-braces. A `timeout` on the shell command around
    /// `cargo test` is not enough: it kills the harness without producing a verdict, and a hung
    /// test binary does not necessarily die with it — which is the very failure this ticket is
    /// about, a tool that returns nothing about an input it never finished examining. A gate step
    /// that hangs is strictly worse than one that fails, because nobody gets a red; they get
    /// silence. So a block here must surface as a **FAILED test**, in-process, on its own.
    ///
    /// Sized well clear of the slowest legitimate path in this module — `FRAME_GAP_MS * 3` (6 s)
    /// in the incrementality test, `SHUTDOWN_GRACE * 3` (9 s) in the shutdown test.
    const TEST_DEADLINE: Duration = Duration::from_secs(25);

    /// Run `body` under [`TEST_DEADLINE`]. A panic inside `body` propagates normally, so ordinary
    /// assertion failures still read as ordinary assertion failures; only a *block* is reported
    /// as one.
    async fn with_deadline(name: &str, body: impl std::future::Future<Output = ()>) {
        assert!(
            tokio::time::timeout(TEST_DEADLINE, body).await.is_ok(),
            "{name} BLOCKED: exceeded its {TEST_DEADLINE:?} internal deadline. Something in this \
             process is not making progress — read it as the streaming/shutdown regression it is, \
             not as a slow machine."
        );
    }

    /// Fake upstream. `/api/stream` mimics the real `status/stream` handler's shape: one frame at
    /// once, a long gap, a second frame, then **never closes**. `/api/finite` is an ordinary
    /// finite JSON response carrying a non-200 status and a custom header.
    async fn start_fake_upstream() -> u16 {
        async fn stream_h() -> Response {
            let body = Body::from_stream(futures_util::stream::unfold(0u8, |step| async move {
                match step {
                    0 => Some((
                        Ok::<_, std::io::Error>(axum::body::Bytes::from_static(b"data: one\n\n")),
                        1,
                    )),
                    1 => {
                        tokio::time::sleep(Duration::from_millis(FRAME_GAP_MS)).await;
                        Some((Ok(axum::body::Bytes::from_static(b"data: two\n\n")), 2))
                    }
                    // Park forever: the body never ends, exactly like a live SSE subscription.
                    _ => {
                        std::future::pending::<()>().await;
                        None
                    }
                }
            }));
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/event-stream")
                .header("x-accel-buffering", "no")
                .body(body)
                .unwrap()
        }
        async fn finite_h() -> Response {
            Response::builder()
                .status(StatusCode::CREATED)
                .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
                .header("x-custom-upstream", "kept")
                .body(Body::from(r#"{"ok":true,"n":42}"#))
                .unwrap()
        }
        let app = axum::Router::new()
            .route("/api/stream", axum::routing::get(stream_h))
            .route("/api/finite", axum::routing::get(finite_h));
        let l = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move {
            let _ = axum::serve(l, app).await;
        });
        port
    }

    async fn start_proxy(upstream_port: u16) -> RunningServer {
        start_server(
            ServeConfig {
                dir: std::env::temp_dir(),
                api_proxy: Some(format!("http://127.0.0.1:{upstream_port}")),
                map_assets_dir: None,
            },
            0,
        )
        .await
        .unwrap()
    }

    /// Read from `sock` until `needle` appears or `budget` elapses. Returns whether it appeared,
    /// plus whether the peer closed the connection (EOF) during the wait.
    async fn read_until(
        sock: &mut tokio::net::TcpStream,
        buf: &mut Vec<u8>,
        needle: &str,
        budget: Duration,
    ) -> (bool, bool) {
        let deadline = tokio::time::Instant::now() + budget;
        loop {
            if String::from_utf8_lossy(buf).contains(needle) {
                return (true, false);
            }
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                return (String::from_utf8_lossy(buf).contains(needle), false);
            }
            let mut tmp = [0u8; 4096];
            match tokio::time::timeout(left, sock.read(&mut tmp)).await {
                Ok(Ok(0)) => return (String::from_utf8_lossy(buf).contains(needle), true), // EOF
                Ok(Ok(n)) => buf.extend_from_slice(&tmp[..n]),
                Ok(Err(_)) => return (String::from_utf8_lossy(buf).contains(needle), true),
                Err(_) => return (String::from_utf8_lossy(buf).contains(needle), false), // budget
            }
        }
    }

    /// THE regression guard. Reverting the handler to `upstream.bytes().await` fails this at the
    /// first assertion: with buffering, frame 1 never arrives at all, because the only delivery
    /// moment is end-of-body and this body has none.
    #[tokio::test]
    async fn sse_frames_arrive_incrementally_not_buffered() {
        with_deadline(
            "sse_frames_arrive_incrementally_not_buffered",
            sse_frames_arrive_incrementally_not_buffered_body(),
        )
        .await;
    }

    async fn sse_frames_arrive_incrementally_not_buffered_body() {
        let up = start_fake_upstream().await;
        let proxy = start_proxy(up).await;

        let mut sock = tokio::net::TcpStream::connect(("127.0.0.1", proxy.port))
            .await
            .unwrap();
        sock.write_all(b"GET /api/stream HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut buf = Vec::new();

        // (1) Frame 1 lands well before the upstream has produced frame 2.
        let t0 = tokio::time::Instant::now();
        let (got1, eof1) = read_until(
            &mut sock,
            &mut buf,
            "data: one",
            Duration::from_millis(FIRST_FRAME_BUDGET_MS),
        )
        .await;
        let frame1_ms = t0.elapsed().as_millis();
        assert!(
            got1,
            "frame 1 did not arrive within {FIRST_FRAME_BUDGET_MS}ms — the proxy is buffering the \
             whole body before responding, so an endless stream never completes. Got {} bytes: {:?}",
            buf.len(),
            String::from_utf8_lossy(&buf)
        );
        assert!(!eof1, "upstream stream closed early; it must stay open");

        // (2) THE DISCRIMINATOR — frame 1 is in hand while frame 2 does not exist yet. A
        //     buffering proxy has a single delivery moment, so it could never produce this state.
        assert!(
            !String::from_utf8_lossy(&buf).contains("data: two"),
            "frame 2 arrived together with frame 1 — that is a buffered whole-body delivery, not \
             a stream"
        );
        // The measurement behind the claim, so the evidence is a number and not just a green
        // check. Visible with `--nocapture`; under the old buffering handler this line is
        // unreachable, because the assertion above it never gets a byte to inspect.
        eprintln!(
            "[T-361] frame 1 observed {frame1_ms} ms after request ({} bytes in hand); stream \
             still open; frame 2 absent (upstream emits it only after {FRAME_GAP_MS} ms)",
            buf.len()
        );

        // (3) Headers prove it is a live event stream and that we did not inherit stale framing.
        let head = String::from_utf8_lossy(&buf).to_lowercase();
        assert!(
            head.contains("content-type: text/event-stream"),
            "upstream content-type must survive the proxy: {head}"
        );
        assert!(
            head.contains("x-accel-buffering: no"),
            "upstream's do-not-buffer instruction must survive the proxy: {head}"
        );
        assert!(
            !head.contains("content-length:"),
            "a streamed body must not carry a content-length: {head}"
        );

        // (4) The same still-open connection goes on to deliver frame 2.
        let (got2, eof2) = read_until(
            &mut sock,
            &mut buf,
            "data: two",
            Duration::from_millis(FRAME_GAP_MS * 3),
        )
        .await;
        assert!(got2, "frame 2 never arrived on the open stream");
        assert!(!eof2, "stream must still be open after frame 2");

        drop(sock);
        proxy.close().await;
    }

    /// The other half of the hang, guarded on its own: shutting the server down while an endless
    /// stream is **still open** must not block.
    ///
    /// This is the failure that actually bit — graceful shutdown waits for in-flight connections
    /// to drain, and a live SSE subscription never drains. The deliberately-leaked socket here is
    /// the point: nothing closes the stream, so an unbounded `handle.await` in
    /// [`RunningServer::close`] parks forever and this test hangs instead of failing. The outer
    /// `timeout` turns that into a red.
    #[tokio::test]
    async fn close_does_not_hang_on_a_still_open_stream() {
        with_deadline(
            "close_does_not_hang_on_a_still_open_stream",
            close_does_not_hang_on_a_still_open_stream_body(),
        )
        .await;
    }

    async fn close_does_not_hang_on_a_still_open_stream_body() {
        let up = start_fake_upstream().await;
        let proxy = start_proxy(up).await;

        let mut sock = tokio::net::TcpStream::connect(("127.0.0.1", proxy.port))
            .await
            .unwrap();
        sock.write_all(b"GET /api/stream HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut buf = Vec::new();
        let (got1, _) = read_until(
            &mut sock,
            &mut buf,
            "data: one",
            Duration::from_millis(FIRST_FRAME_BUDGET_MS),
        )
        .await;
        assert!(
            got1,
            "precondition: the stream must be live before we close"
        );

        // Socket intentionally NOT dropped — the connection is still in flight.
        let closed = tokio::time::timeout(SHUTDOWN_GRACE * 3, proxy.close()).await;
        assert!(
            closed.is_ok(),
            "RunningServer::close hung on an open SSE stream — graceful shutdown was waiting for \
             a connection that never drains"
        );
        drop(sock);
    }

    /// Non-regression for ordinary finite responses: status, body bytes and forwarded headers are
    /// unchanged by the switch to a streamed body.
    #[tokio::test]
    async fn finite_responses_are_unchanged_by_streaming() {
        with_deadline(
            "finite_responses_are_unchanged_by_streaming",
            finite_responses_are_unchanged_by_streaming_body(),
        )
        .await;
    }

    async fn finite_responses_are_unchanged_by_streaming_body() {
        let up = start_fake_upstream().await;
        let proxy = start_proxy(up).await;

        let res = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{}/api/finite", proxy.port))
            .send()
            .await
            .unwrap();

        assert_eq!(
            res.status().as_u16(),
            201,
            "upstream status must pass through"
        );
        assert_eq!(
            res.headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("application/json; charset=utf-8"),
        );
        assert_eq!(
            res.headers()
                .get("x-custom-upstream")
                .and_then(|v| v.to_str().ok()),
            Some("kept"),
        );
        // The gate's own cross-origin-isolation + no-store headers still apply.
        assert_eq!(
            res.headers()
                .get("cross-origin-opener-policy")
                .and_then(|v| v.to_str().ok()),
            Some("same-origin"),
        );
        assert_eq!(
            res.text().await.unwrap(),
            r#"{"ok":true,"n":42}"#,
            "finite body must be byte-identical through the streamed path"
        );

        proxy.close().await;
    }
}
