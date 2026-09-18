//! T-165.5 — Chrome DevTools Protocol client (port of `driver/cdp.mjs`).
//!
//! Same wire behavior as the Node harness: raw CDP over one WebSocket per page, chromium
//! resolved from `CHROME_HEADLESS_SHELL` or the playwright cache, SwiftShader WebGL2 +
//! lavapipe WebGPU flags, fixed 1440×900 dsf=1 viewport applied BEFORE navigation, init
//! scripts on document-start.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use futures_util::SinkExt;
use futures_util::stream::{SplitSink, StreamExt};
use serde_json::{Value, json};
use tokio::io::AsyncBufReadExt;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub struct Browser {
    child: Child,
    pub debug_port: u16,
    pub http: reqwest::Client,
    /// Per-launch chromium profile dir (T-166 hygiene). Every smoke gets its OWN profile so OPFS
    /// + IndexedDB (large persisted world/mission state) never bleed across smokes in a suite run.
    user_data_dir: PathBuf,
    /// Chrome's own recent stdout+stderr, filled by the [`drain_pipe`] tasks (T-354).
    log_tail: Arc<StdMutex<VecDeque<String>>>,
}

impl Browser {
    /// Chrome's own last `LOG_TAIL_LINES` output lines, oldest first (T-354).
    ///
    /// This is the browser's account of its own death — what runbook P2 goes hunting for by
    /// re-launching chromium by hand. Worth printing on any wedge/timeout path: a `FATAL` /
    /// `SK_ABORT` / `Received signal` line here names the cause, and its **absence** is itself
    /// informative (the browser never got as far as complaining).
    pub fn recent_output(&self) -> Vec<String> {
        self.log_tail
            .lock()
            .map(|t| t.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// SIGTERM the whole chrome PROCESS GROUP (T-166). Chrome forks renderer/gpu/zygote children;
    /// signalling only the parent pid (the old behavior) orphaned those children, which kept
    /// pegging every core under SwiftShader software GL → the *next* smoke's page starved of CPU
    /// and its `Runtime.evaluate` wedged (the suite "hang"). `launch` puts chrome in its own group
    /// (`process_group(0)`, leader pid == child pid) so `kill(-pid, …)` targets the tree, not us.
    pub fn kill(&mut self) {
        self.signal_group(libc::SIGTERM);
    }

    fn signal_group(&self, sig: libc::c_int) {
        if let Some(pid) = self.child.id() {
            unsafe {
                // Negative pid = "the process group" (pgid == leader pid).
                libc::kill(-(pid as i32), sig);
            }
        }
    }

    /// SIGTERM the group → reap (bounded) → SIGKILL the group → remove the profile dir. Reaping
    /// BEFORE the next smoke launches frees the debug port + CPU and drops all renderer children.
    pub async fn shutdown(mut self) {
        self.signal_group(libc::SIGTERM);
        if tokio::time::timeout(Duration::from_secs(5), self.child.wait())
            .await
            .is_err()
        {
            // Still alive after SIGTERM → SIGKILL the whole group and reap so nothing lingers.
            self.signal_group(libc::SIGKILL);
            let _ = self.child.wait().await;
        }
        let _ = tokio::fs::remove_dir_all(&self.user_data_dir).await;
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        // Best-effort profile cleanup for the `kill()`-then-drop path (e.g. vsuite), which does
        // not go through the async `shutdown`. `shutdown` already removed it → this is a no-op.
        let _ = std::fs::remove_dir_all(&self.user_data_dir);
    }
}

/// How many of chrome's most recent output lines a [`Browser`] keeps ([`Browser::recent_output`]).
const LOG_TAIL_LINES: usize = 200;

/// The GPU backend chromium is launched with — the one knob that separates the gate harness from
/// the editor-capture harness (T-165.5 vs T-661 capture port).
///
/// # Why this is a choice and not a constant
///
/// The **gate** smokes render the SPA chrome and a `wgpu` canvas that only needs WebGL2, and they
/// must run **in CI on a box with no GPU** — so they take [`GpuBackend::Swiftshader`] (ANGLE's
/// software rasterizer, `--enable-unsafe-swiftshader`), which renders deterministically everywhere.
///
/// The **capture** harness (`capture shot` / `capture zoomsweep`) photographs the *live* Mission
/// Creator, whose map is a real `wgpu`/WebGPU engine. Under SwiftShader that engine cannot create
/// its WebGPU buffers — `createBuffer failed, size (32) too large` → wasm abort → the editor hangs
/// on the boot overlay forever (measured; see `docs/tools/editor_capture.md` §2). It needs
/// [`GpuBackend::Vulkan`]: ANGLE/Vulkan on the host's real device, which is the only mode that
/// boots the engine. That is why the capture path exists as its own launch flavour rather than
/// reusing the gate's flags — everything else (profile hygiene, pipe draining, the font cache, the
/// process-group teardown) is shared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuBackend {
    /// ANGLE SwiftShader — software WebGL2, no GPU required. The gate harness default.
    Swiftshader,
    /// ANGLE/Vulkan on the real device — required to boot the live WebGPU map engine. The
    /// editor-capture harness default. The flag set mirrors `run_shot_gpu.sh` GPU_MODE=vulkan
    /// (`--use-angle=vulkan --enable-features=Vulkan --use-vulkan --ignore-gpu-blocklist`).
    Vulkan,
}

impl GpuBackend {
    /// The chromium GPU flags for this backend, appended after the shared base flags in [`launch`].
    fn flags(self) -> &'static [&'static str] {
        match self {
            // T-165.5 baseline: ANGLE software GL for CI determinism.
            GpuBackend::Swiftshader => &["--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
            // run_shot_gpu.sh vulkan branch, verbatim — the only mode the live wgpu engine boots on.
            GpuBackend::Vulkan => &[
                "--use-angle=vulkan",
                "--enable-features=Vulkan",
                "--use-vulkan",
                "--ignore-gpu-blocklist",
            ],
        }
    }
}

type WsSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

struct PageShared {
    sink: Mutex<WsSink>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Value>>>,
    event_waiters: Mutex<HashMap<String, Vec<oneshot::Sender<Value>>>>,
    persistent: Mutex<HashMap<String, Vec<mpsc::UnboundedSender<Value>>>>,
    next_id: Mutex<u64>,
}

pub struct Page {
    shared: Arc<PageShared>,
    target_id: String,
    debug_port: u16,
    http: reqwest::Client,
    _reader: tokio::task::JoinHandle<()>,
}

/// Fixed 1440×900 dsf=1 viewport (the harness default).
pub const VIEWPORT: (u32, u32) = (1440, 900);

impl Page {
    /// The suite default: [`Self::send_with_timeout`] at 130 s (see there).
    pub async fn send(&self, method: &str, params: Value) -> Result<Value> {
        self.send_with_timeout(method, params, Duration::from_secs(130))
            .await
    }

    /// A CDP call with an explicit per-call WS timeout (T-177). The suite default (130 s) sits just
    /// past the 120 s server-side `Runtime.evaluate` timeout so a real slow eval still completes but
    /// a wedged page main thread fails the smoke loudly instead of hanging `wait_for` — and the whole
    /// suite — forever (T-166 safety net). The fail-fast `gate doctor` liveness probe passes a SHORT
    /// timeout (via [`Self::evaluate_with_timeout`]) so a wedge surfaces in seconds with a diagnosis.
    pub async fn send_with_timeout(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value> {
        let id = {
            let mut n = self.shared.next_id.lock().await;
            *n += 1;
            *n
        };
        let (tx, rx) = oneshot::channel();
        self.shared.pending.lock().await.insert(id, tx);
        let frame = json!({ "id": id, "method": method, "params": params }).to_string();
        self.shared
            .sink
            .lock()
            .await
            .send(Message::text(frame))
            .await
            .context("cdp: ws send")?;
        let m = tokio::time::timeout(timeout, rx)
            .await
            .map_err(|_| anyhow!("cdp: ws call timed out ({method})"))?
            .context("cdp: ws closed mid-call")?;
        if !m["error"].is_null() {
            return Err(anyhow!("{method}: {}", m["error"]));
        }
        Ok(m["result"].clone())
    }

    /// One-shot event waiter (Node's `waitEvent`).
    pub async fn wait_event(&self, method: &str, timeout_ms: u64) -> Result<Value> {
        let (tx, rx) = oneshot::channel();
        self.shared
            .event_waiters
            .lock()
            .await
            .entry(method.to_string())
            .or_default()
            .push(tx);
        tokio::time::timeout(Duration::from_millis(timeout_ms), rx)
            .await
            .map_err(|_| anyhow!("cdp: timeout waiting for {method}"))?
            .map_err(|_| anyhow!("cdp: waiter dropped for {method}"))
    }

    /// Persistent event stream (Node's `onEvent`) — fired for EVERY matching event.
    pub async fn on_event(&self, method: &str) -> mpsc::UnboundedReceiver<Value> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.shared
            .persistent
            .lock()
            .await
            .entry(method.to_string())
            .or_default()
            .push(tx);
        rx
    }

    pub async fn set_viewport(&self, width: u32, height: u32) -> Result<()> {
        self.send(
            "Emulation.setDeviceMetricsOverride",
            json!({
                "width": width, "height": height, "deviceScaleFactor": 1, "mobile": false,
                "screenWidth": width, "screenHeight": height,
            }),
        )
        .await?;
        Ok(())
    }

    pub async fn navigate(&self, to: &str) -> Result<()> {
        // Register the load waiter BEFORE navigating (same ordering as the Node harness).
        let (tx, rx) = oneshot::channel();
        self.shared
            .event_waiters
            .lock()
            .await
            .entry("Page.loadEventFired".to_string())
            .or_default()
            .push(tx);
        self.send("Page.navigate", json!({ "url": to })).await?;
        tokio::time::timeout(Duration::from_millis(30000), rx)
            .await
            .map_err(|_| anyhow!("cdp: timeout waiting for Page.loadEventFired"))?
            .map_err(|_| anyhow!("cdp: load waiter dropped"))?;
        Ok(())
    }

    /// `Runtime.evaluate` with `returnByValue` (Node's `evaluate`) — the suite default (130 s WS /
    /// 120 s server).
    pub async fn evaluate(&self, expression: &str, await_promise: bool) -> Result<Value> {
        self.evaluate_with_timeout(expression, await_promise, Duration::from_secs(130))
            .await
    }

    /// `evaluate` with an explicit WS timeout (T-177). The server-side `Runtime.evaluate` `timeout`
    /// is set to match (clamped to `1000..=120000` ms) so the browser gives up in lockstep with the
    /// client — used by the fail-fast `gate doctor` liveness probe (short timeout → a wedge surfaces
    /// in seconds, not 130 s).
    pub async fn evaluate_with_timeout(
        &self,
        expression: &str,
        await_promise: bool,
        timeout: Duration,
    ) -> Result<Value> {
        let server_ms = u64::try_from(timeout.as_millis())
            .unwrap_or(120_000)
            .clamp(1_000, 120_000);
        let r = self
            .send_with_timeout(
                "Runtime.evaluate",
                json!({
                    "expression": expression, "awaitPromise": await_promise,
                    "returnByValue": true, "timeout": server_ms,
                }),
                timeout,
            )
            .await?;
        if !r["exceptionDetails"].is_null() {
            return Err(anyhow!(
                "{}",
                r["exceptionDetails"]["text"]
                    .as_str()
                    .unwrap_or("cdp: evaluate failed")
            ));
        }
        Ok(r["result"]["value"].clone())
    }

    /// Poll a boolean expression until true (app-ready, engine-ready, …).
    pub async fn wait_for(&self, expr: &str, tries: u32, interval_ms: u64) -> Result<bool> {
        for _ in 0..tries {
            if self.evaluate(expr, false).await?.as_bool() == Some(true) {
                return Ok(true);
            }
            sleep_ms(interval_ms).await;
        }
        Ok(false)
    }

    /// Full-viewport PNG (`Page.captureScreenshot`).
    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        use base64::Engine as _;
        let r = self
            .send(
                "Page.captureScreenshot",
                json!({ "format": "png", "captureBeyondViewport": false }),
            )
            .await?;
        let data = r["data"].as_str().unwrap_or_default();
        Ok(base64::engine::general_purpose::STANDARD.decode(data)?)
    }

    pub async fn dispatch_mouse(&self, ev_type: &str, x: f64, y: f64, extra: Value) -> Result<()> {
        let mut params =
            json!({ "type": ev_type, "x": x, "y": y, "button": "left", "clickCount": 1 });
        merge(&mut params, extra);
        self.send("Input.dispatchMouseEvent", params).await?;
        Ok(())
    }

    pub async fn dispatch_key(&self, ev_type: &str, key: &str, extra: Value) -> Result<()> {
        let mut params = json!({ "type": ev_type, "key": key });
        merge(&mut params, extra);
        self.send("Input.dispatchKeyEvent", params).await?;
        Ok(())
    }

    /// Fulfill an intercepted request with a body handed over byte for byte.
    ///
    /// The byte path matters for `text/event-stream`: a Server-Sent Events frame is delimited by a
    /// literal `\n\n`, and routing that body through `serde_json` would re-escape the delimiter into
    /// the two-character sequence `\\n\\n`, so the subscriber's frame splitter never sees a boundary
    /// and no frame ever decodes. Anything that is not JSON reaches the page through here.
    pub async fn fulfill_raw(
        &self,
        request_id: &str,
        status: u16,
        content_type: &str,
        body_bytes: &[u8],
    ) -> Result<()> {
        use base64::Engine as _;
        let body = base64::engine::general_purpose::STANDARD.encode(body_bytes);
        self.send(
            "Fetch.fulfillRequest",
            json!({
                "requestId": request_id, "responseCode": status,
                "responseHeaders": [{ "name": "content-type", "value": content_type }],
                "body": body,
            }),
        )
        .await?;
        Ok(())
    }

    /// Fulfill an intercepted request with a JSON body (the harness's fixture reply).
    ///
    /// Serialization and the `application/json` content type are the only things this adds over
    /// [`Self::fulfill_raw`], so every existing fixture reply keeps the bytes it always had.
    pub async fn fulfill_json(
        &self,
        request_id: &str,
        status: u16,
        body_json: &Value,
    ) -> Result<()> {
        let body = serde_json::to_string(body_json)?;
        self.fulfill_raw(request_id, status, "application/json", body.as_bytes())
            .await
    }

    pub async fn continue_request(&self, request_id: &str) -> Result<()> {
        self.send("Fetch.continueRequest", json!({ "requestId": request_id }))
            .await?;
        Ok(())
    }

    /// Close the tab via the browser HTTP endpoint (same as Node's `close`).
    pub async fn close(&self) {
        let _ = self
            .http
            .get(format!(
                "http://127.0.0.1:{}/json/close/{}",
                self.debug_port, self.target_id
            ))
            .send()
            .await;
    }
}

#[path = "cdp/sleep_ms.rs"]
mod sleep_ms;
pub use sleep_ms::find_chromium;
pub use sleep_ms::is_headless_shell;
pub use sleep_ms::launch;
pub use sleep_ms::launch_with_gpu;
use sleep_ms::merge;
pub use sleep_ms::new_page;
pub use sleep_ms::sleep_ms;
pub use sleep_ms::wait_http;
