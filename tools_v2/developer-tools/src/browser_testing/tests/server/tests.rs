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
