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

pub async fn sleep_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

/// `CHROME_HEADLESS_SHELL` env → `~/.cache/ms-playwright` scan.
///
/// T-177 — prefer the **full `chrome` build over `chrome-headless-shell`**. The minimal headless
/// shell ships a stubbed `SkFontMgr_FontConfigInterface` whose `onMatchFamilyStyleCharacter`
/// (per-character font fallback) is a `FATAL: … "Not implemented"` (`SkFontMgr_FontConfigInterface.cpp:163`):
/// the moment a page needs a fallback glyph — which the editor chrome does, env-dependently — the
/// renderer aborts (SIGTRAP/SIGABRT), and the harness sees only a 130 s `Runtime.evaluate` hang
/// (the process is dead, the WS never answers). The full `chrome --headless=new` (see [`launch`])
/// has the complete font backend and does not crash. Fallback to the shell only if no full build
/// exists (the shell still works for pages that never trigger fallback). Also note the playwright
/// full-chrome path is `chrome-linux64/chrome` (not the old `chrome-linux/chrome`).
pub fn find_chromium() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CHROME_HEADLESS_SHELL") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }
    let cache = dirs_home()?.join(".cache/ms-playwright");
    if !cache.exists() {
        return None;
    }
    for prefix in ["chromium-", "chromium_headless_shell-"] {
        let mut dirs: Vec<String> = std::fs::read_dir(&cache)
            .ok()?
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|d| d.starts_with(prefix))
            .collect();
        dirs.sort();
        dirs.reverse();
        for d in dirs {
            for rel in [
                "chrome-linux64/chrome",
                "chrome-headless-shell-linux64/chrome-headless-shell",
            ] {
                let bin = cache.join(&d).join(rel);
                if bin.exists() {
                    return Some(bin);
                }
            }
        }
    }
    None
}

/// True when the resolved chromium is the minimal `chrome-headless-shell` (which is always headless
/// and ignores `--headless`); the full `chrome` build needs an explicit `--headless=new` ([`launch`]).
pub fn is_headless_shell(bin: &std::path::Path) -> bool {
    bin.to_string_lossy().contains("chrome-headless-shell")
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Poll a URL until it answers (ok or 404 = server up). 60 tries × 250 ms, as in cdp.mjs.
pub async fn wait_http(client: &reqwest::Client, url: &str, tries: u32) -> bool {
    for _ in 0..tries {
        if let Ok(res) = client.get(url).send().await
            && (res.status().is_success() || res.status().as_u16() == 404)
        {
            return true;
        }
        sleep_ms(250).await;
    }
    false
}

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
    /// Chrome's own last [`LOG_TAIL_LINES`] output lines, oldest first (T-354).
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

/// Consume one of chrome's output pipes until EOF, keeping the last [`LOG_TAIL_LINES`] lines.
///
/// # Why this exists (T-354 — the undrained-pipe deadlock)
///
/// [`launch`] hands chrome piped stdout+stderr. A pipe holds **64 KiB**; once it fills and nobody
/// reads, the next `write(2)` **blocks the thread that issued it** — indefinitely, because a pipe
/// nobody drains never drains. Chromium is chatty. MEASURED on this box with
/// `--enable-logging=stderr --v=1`, writing to a file so nothing throttled it: **87,583 bytes of
/// stderr inside the first second** on `about:blank` alone, 109,581 by t+6 s. That is 1.34× the
/// buffer before a page exists at all.
///
/// So the buffer fills, and *which* chrome thread happens to own the write that crosses the
/// threshold decides the outcome: a `ThreadPoolForeground` thread merely parks, but the **browser
/// main thread** parking stops the DevTools endpoint dead. MEASURED mid-hang: main thread in
/// `syscall=1 (write)` on `wchan=anon_pipe_write` with `fd/2 -> pipe:[…]`, and `/json/version`
/// accepting the TCP connection then answering **nothing** (`curl(52) Empty reply`, as against
/// `curl(7) Connection refused` from a browser that is genuinely gone). Every `Runtime.evaluate`
/// after that times out.
///
/// Two things make it expensive. It is **intermittent** — thread-scheduling roulette, so it passes
/// often enough to look like flake. And it is **self-disguising**: the harness sees exactly the
/// T-320 font-abort signature (`cdp: ws call timed out (Runtime.evaluate)`, which `gate doctor`
/// then reports as "the headless browser process DIED"), so the diagnosis points at Skia while the
/// browser sits alive and blocked in `write`. T-232 lost its second hand-rolled harness to this;
/// T-320 lost five sessions to the same signature from a genuinely different cause. Draining
/// removes the failure mode rather than detecting it: chrome cannot block on a pipe someone is
/// always reading.
///
/// The tail is the other half. Chrome's stderr is the **only** copy of its own abort reason
/// (runbook P2) and [`launch`] used to discard it, which is why the debug recipe had to re-launch
/// chromium by hand with `--enable-logging=stderr` and hope an intermittent fault reproduced.
fn drain_pipe<R>(pipe: R, tail: Arc<StdMutex<VecDeque<String>>>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut pipe = tokio::io::BufReader::new(pipe);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            // `read_until` + lossy decode, NOT `lines()`: `lines()` returns `Err` on invalid UTF-8,
            // and a drain that can stop early is a drain that can re-open the deadlock. Chrome's
            // logs carry page-controlled text (console output), so that is reachable, not theory.
            match pipe.read_until(b'\n', &mut buf).await {
                // EOF (chrome and every fd-inheriting child are gone) or a broken pipe: either way
                // there is nothing left that could block on a write.
                Ok(0) | Err(_) => return,
                Ok(_) => {}
            }
            let line = String::from_utf8_lossy(&buf).trim_end().to_string();
            // Locked only for the push, never across an await — hence the std mutex, so the
            // accessor can stay sync and be callable from an error path.
            if let Ok(mut t) = tail.lock() {
                if t.len() == LOG_TAIL_LINES {
                    t.pop_front();
                }
                t.push_back(line);
            }
        }
    });
}

/// Spawn a headless chromium with SwiftShader WebGL2 + lavapipe WebGPU.
pub async fn launch(debug_port: u16, extra_args: &[String]) -> Result<Browser> {
    let chromium = find_chromium().ok_or_else(|| {
        anyhow!("cdp: no chromium (set CHROME_HEADLESS_SHELL or install playwright)")
    })?;
    // Unique profile dir per launch (harness pid + debug port — no Date/rand, deterministic
    // within a run). Removed first in case a crashed prior run left a stale copy with a lock.
    let user_data_dir =
        std::env::temp_dir().join(format!("tbd-cdp-{}-{debug_port}", std::process::id()));
    let _ = std::fs::remove_dir_all(&user_data_dir);
    let mut args: Vec<String> = Vec::new();
    // T-177 — the full `chrome` build must be told to run headless (the shell is always headless and
    // ignores this). Without it the full binary tries to open a window and aborts. See `find_chromium`.
    if !is_headless_shell(&chromium) {
        args.push("--headless=new".into());
    }
    args.push("--no-sandbox".into());
    args.push("--disable-gpu-sandbox".into());
    args.push(format!("--remote-debugging-port={debug_port}"));
    args.push(format!("--user-data-dir={}", user_data_dir.display()));
    args.push("--use-angle=swiftshader".into());
    args.push("--enable-unsafe-swiftshader".into());
    args.push("--enable-unsafe-webgpu".into());
    args.push("--hide-scrollbars".into());
    args.push("--force-device-scale-factor=1".into());
    args.push("about:blank".into());
    args.extend(extra_args.iter().cloned());
    let mut child = Command::new(&chromium)
        .args(&args)
        // Own process group (leader pid == child pid) so shutdown can signal the whole chrome
        // tree — renderer/gpu children included — without touching the harness (T-166).
        .process_group(0)
        .stdin(std::process::Stdio::null())
        // Both pipes are DRAINED below — piping either one without reading it deadlocks chrome at
        // 64 KiB. See [`drain_pipe`] (T-354).
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn {}", chromium.display()))?;
    // T-354 — start draining BEFORE the first `/json/version` poll below. Chrome writes past the
    // 64 KiB buffer inside its first second, so a drain started any later races the very deadlock
    // it exists to prevent. Both fds: stdout is "usually empty", and that assumption is exactly the
    // kind that turns into an intermittent hang.
    let log_tail = Arc::new(StdMutex::new(VecDeque::with_capacity(LOG_TAIL_LINES)));
    if let Some(out) = child.stdout.take() {
        drain_pipe(out, Arc::clone(&log_tail));
    }
    if let Some(err) = child.stderr.take() {
        drain_pipe(err, Arc::clone(&log_tail));
    }
    let http = reqwest::Client::new();
    for _ in 0..80 {
        if let Ok(r) = http
            .get(format!("http://127.0.0.1:{debug_port}/json/version"))
            .send()
            .await
            && r.status().is_success()
        {
            break;
        }
        sleep_ms(125).await;
    }
    Ok(Browser {
        child,
        debug_port,
        http,
        user_data_dir,
        log_tail,
    })
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

/// Open a fresh page, apply init scripts + viewport BEFORE navigation, then (optionally)
/// navigate and wait for load.
pub async fn new_page(browser: &Browser, url: Option<&str>, init_scripts: &[&str]) -> Result<Page> {
    let target: Value = browser
        .http
        .put(format!(
            "http://127.0.0.1:{}/json/new?about:blank",
            browser.debug_port
        ))
        .send()
        .await?
        .json()
        .await
        .context("cdp: /json/new")?;
    let ws_url = target["webSocketDebuggerUrl"]
        .as_str()
        .ok_or_else(|| anyhow!("cdp: no webSocketDebuggerUrl"))?;
    let target_id = target["id"].as_str().unwrap_or_default().to_string();
    let (ws, _) = tokio_tungstenite::connect_async(ws_url)
        .await
        .context("cdp: ws connect")?;
    let (sink, mut stream) = ws.split();

    let shared = Arc::new(PageShared {
        sink: Mutex::new(sink),
        pending: Mutex::new(HashMap::new()),
        event_waiters: Mutex::new(HashMap::new()),
        persistent: Mutex::new(HashMap::new()),
        next_id: Mutex::new(0),
    });

    let reader_shared = Arc::clone(&shared);
    let reader = tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            let Message::Text(text) = msg else { continue };
            let Ok(m) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            if let Some(id) = m["id"].as_u64() {
                if let Some(tx) = reader_shared.pending.lock().await.remove(&id) {
                    let _ = tx.send(m);
                }
                continue;
            }
            let Some(method) = m["method"].as_str() else {
                continue;
            };
            let params = m["params"].clone();
            {
                let mut ph = reader_shared.persistent.lock().await;
                if let Some(subs) = ph.get_mut(method) {
                    subs.retain(|tx| tx.send(params.clone()).is_ok());
                }
            }
            let mut ew = reader_shared.event_waiters.lock().await;
            if let Some(waiters) = ew.remove(method) {
                for tx in waiters {
                    let _ = tx.send(params.clone());
                }
            }
        }
    });

    let page = Page {
        shared,
        target_id,
        debug_port: browser.debug_port,
        http: browser.http.clone(),
        _reader: reader,
    };

    page.send("Page.enable", json!({})).await?;
    page.send("Runtime.enable", json!({})).await?;
    page.set_viewport(VIEWPORT.0, VIEWPORT.1).await?;
    for s in init_scripts {
        page.send(
            "Page.addScriptToEvaluateOnNewDocument",
            json!({ "source": s }),
        )
        .await?;
    }
    if let Some(u) = url {
        page.navigate(u).await?;
    }
    Ok(page)
}

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

    /// Fulfill an intercepted request with a JSON body (the harness's fixture reply).
    pub async fn fulfill_json(
        &self,
        request_id: &str,
        status: u16,
        body_json: &Value,
    ) -> Result<()> {
        use base64::Engine as _;
        let body =
            base64::engine::general_purpose::STANDARD.encode(serde_json::to_string(body_json)?);
        self.send(
            "Fetch.fulfillRequest",
            json!({
                "requestId": request_id, "responseCode": status,
                "responseHeaders": [{ "name": "content-type", "value": "application/json" }],
                "body": body,
            }),
        )
        .await?;
        Ok(())
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

fn merge(base: &mut Value, extra: Value) {
    if let (Value::Object(b), Value::Object(e)) = (base, extra) {
        for (k, v) in e {
            b.insert(k, v);
        }
    }
}
