//! Chromium launch, page setup and the polling helpers.
//!
//! **Role:** [`launch`], [`launch_with_gpu`], [`new_page`], [`wait_http`], [`sleep_ms`], the output
//! drain and the event-field merge.
//! **Position:** a child module of `browser_session.rs`, so it can build the handles' private
//! fields; the crate root re-exports its public functions.
//! **Signals & state:** each launch spawns two drain tasks that keep the browser's last output
//! lines in the `Browser`'s shared tail.
//! **Invariants:** both pipes are drained from spawn; the viewport and init scripts precede the
//! first navigation; every launch gets a fresh profile folder and the gate font cache.

use super::*;
use crate::chromium_discovery::{find_chromium, is_headless_shell};
use crate::gate_font_cache::gate_font_cache_dir;

/// Sleeps `ms` milliseconds on the tokio timer.
pub async fn sleep_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

/// Poll a URL until it answers (ok or 404 = server up). 60 tries × 250 ms.
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

/// Consume one of chrome's output pipes until EOF, keeping the last `LOG_TAIL_LINES` lines.
///
/// # Why this exists: the undrained-pipe deadlock
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
/// Font-abort signature (`cdp: ws call timed out (Runtime.evaluate)`, which `gate doctor`
/// then reports as "the headless browser process DIED"), so the diagnosis points at Skia while the
/// browser sits alive and blocked in `write`. A hand-rolled harness was lost to this twice;
/// Lost five sessions to the same signature from a genuinely different cause. Draining
/// removes the failure mode rather than detecting it: chrome cannot block on a pipe someone is
/// always reading.
///
/// The tail is the other half. Chrome's stderr is the **only** copy of its own abort reason
/// (runbook P2) and [`launch`] used to discard it, which is why the debug recipe had to re-launch
/// chromium by hand with `--enable-logging=stderr` and hope an intermittent fault reproduced.
pub(super) fn drain_pipe<R>(pipe: R, tail: Arc<StdMutex<VecDeque<String>>>)
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

/// Spawn a headless chromium with SwiftShader WebGL2 + lavapipe WebGPU (the gate-harness default).
///
/// For the editor-capture harness — which needs ANGLE/Vulkan on the real device to boot the live
/// WebGPU map engine — call [`launch_with_gpu`] with [`GpuBackend::Vulkan`].
pub async fn launch(debug_port: u16, extra_args: &[String]) -> Result<Browser> {
    launch_with_gpu(debug_port, GpuBackend::Swiftshader, extra_args).await
}

/// Spawn a headless chromium with a caller-chosen [`GpuBackend`]. See [`launch`] for the shared
/// setup (font cache, per-launch profile, pipe draining, process-group teardown) — the backend is
/// the only thing this varies.
pub async fn launch_with_gpu(
    debug_port: u16,
    gpu: GpuBackend,
    extra_args: &[String],
) -> Result<Browser> {
    let chromium = find_chromium().ok_or_else(|| {
        refusal!(
            "cdp: no chromium (set CHROME_HEADLESS_SHELL or PLAYWRIGHT_BROWSERS_PATH, or install \
             playwright)"
        )
    })?;
    // Every CDP caller (smokes Harness, vsuite, doctor liveness) gets the gate-owned
    // fontconfig cache. Pin `XDG_CACHE_HOME` on the *child* Command rather than relying
    // on process-wide `set_var` after tokio is running: that closes vsuite's separate-chromium
    // gap and avoids relocating an `unsafe` env write into a multi-threaded window.
    let font_cache = gate_font_cache_dir();
    std::fs::create_dir_all(font_cache)
        .with_context(|| format!("create gate font cache {}", font_cache.display()))?;
    // Unique profile dir per launch (harness pid + debug port — no Date/rand, deterministic
    // within a run). Removed first in case a crashed prior run left a stale copy with a lock.
    let user_data_dir =
        std::env::temp_dir().join(format!("tbd-cdp-{}-{debug_port}", std::process::id()));
    let _ = std::fs::remove_dir_all(&user_data_dir);
    let mut args: Vec<String> = Vec::new();
    // The full `chrome` build must be told to run headless (the shell is always headless and
    // ignores this). Without it the full binary tries to open a window and aborts. See `find_chromium`.
    if !is_headless_shell(&chromium) {
        args.push("--headless=new".into());
    }
    args.push("--no-sandbox".into());
    args.push("--disable-gpu-sandbox".into());
    args.push(format!("--remote-debugging-port={debug_port}"));
    args.push(format!("--user-data-dir={}", user_data_dir.display()));
    // Backend-specific GPU flags (SwiftShader for the gate, ANGLE/Vulkan for capture).
    args.extend(gpu.flags().iter().map(|s| (*s).to_string()));
    args.push("--enable-unsafe-webgpu".into());
    args.push("--hide-scrollbars".into());
    args.push("--force-device-scale-factor=1".into());
    args.push("about:blank".into());
    args.extend(extra_args.iter().cloned());
    // tokio's process: the browser lives for the whole async session and its pipes drain on the
    // runtime, which the synchronous `process_runner` cannot host.
    let mut child = Command::new(&chromium)
        .args(&args)
        .env("XDG_CACHE_HOME", font_cache)
        // Own process group (leader pid == child pid) so shutdown can signal the whole chrome
        // tree — renderer/gpu children included — without touching the harness.
        .process_group(0)
        .stdin(std::process::Stdio::null())
        // Both pipes are DRAINED below — piping either one without reading it deadlocks chrome at
        // 64 KiB. See [`drain_pipe`].
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn {}", chromium.display()))?;
    // Start draining BEFORE the first `/json/version` poll below. Chrome writes past the
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
        .ok_or_else(|| refusal!("cdp: no webSocketDebuggerUrl"))?;
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

pub(super) fn merge(base: &mut Value, extra: Value) {
    if let (Value::Object(b), Value::Object(e)) = (base, extra) {
        for (k, v) in e {
            b.insert(k, v);
        }
    }
}
