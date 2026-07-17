//! `mcpd` — the persistent enfusion-mcp broker + offline stub (T-165.7).
//!
//! Broker (port of `scripts/mod/lib/mcp-daemon.mjs`): spawns ONE enfusion-mcp child,
//! initializes it once (paying the ~35 s index load a single time), and serves tools/call
//! requests over an AF_UNIX socket. Requests are serialized (the Workbench NetAPI is
//! single-stream); responses are matched by id and relabelled to id==2 so
//! `cargo xtask mcp consume` parses them unchanged.
//!
//!   mcpd --socket <path> [--pidfile <path>]
//!   Env: ENFUSION_MCP_BIN (resolved by mcp-daemon.sh; .js/.mjs entries run under node,
//!        anything else execs directly), MCP_DAEMON_IDLE (s, default 1800; 0=never),
//!        MCP_DAEMON_MAX_LIFE (s, default 14400; 0=disabled), MCP_CALL_TIMEOUT (s, default
//!        180), MCP_DEBUG=1.
//!
//! Stub (port of `scripts/mod/fixtures/stub-runner.mjs`): emulates the enfusion-mcp server's
//! newline-JSON stdout without a Workbench, for the offline selftest. Selected by `--stub`
//! or env `MCP_STUB=1` (the env form lets `ENFUSION_MCP_BIN=<this binary>` resolve to the
//! stub with no argv plumbing; the real lane never sets it — and it also makes accidental
//! broker-under-broker recursion impossible).
//!
//!   STUB_MODE    success | error | empty | initfail   (default success)
//!   STUB_DAEMON  1 → request/response server (replies to each id); else one-shot + linger
//!   STUB_LINGER  seconds to stay alive in one-shot mode (default 1)

use std::collections::HashMap;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, oneshot};

fn env_secs(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn debug_on() -> bool {
    std::env::var("MCP_DEBUG").as_deref() == Ok("1")
}

macro_rules! dlog {
    ($($arg:tt)*) => {
        if debug_on() {
            eprintln!("[mcp-daemon] {}", format!($($arg)*));
        }
    };
}

/* ───────────────────────────── stub ───────────────────────────── */

fn run_stub() -> ExitCode {
    let mode = std::env::var("STUB_MODE").unwrap_or_else(|_| "success".into());
    let linger = std::env::var("STUB_LINGER")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(1.0)
        .max(0.0);
    let emit = |obj: Value| {
        let mut out = std::io::stdout().lock();
        // A closed pipe (the early-exit consumer SIGPIPE lane) ends the stub cleanly.
        if writeln!(out, "{obj}").is_err() {
            std::process::exit(0);
        }
        let _ = out.flush();
    };
    eprintln!("STUB-STDERR-MARKER mode={mode}");

    if std::env::var("STUB_DAEMON").as_deref() == Ok("1") {
        // Request/response server: reply to each request with a matching id.
        let stdin = std::io::stdin();
        for line in std::io::BufRead::lines(stdin.lock()) {
            let Ok(line) = line else { break };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(o) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            match o["method"].as_str() {
                Some("initialize") => emit(json!({
                    "jsonrpc": "2.0", "id": o["id"],
                    "result": { "protocolVersion": "2024-11-05", "capabilities": {},
                                "serverInfo": { "name": "stub", "version": "0" } }
                })),
                Some("tools/call") => {
                    let name = o["params"]["name"].as_str().unwrap_or_default().to_string();
                    let args = o["params"]["arguments"].clone();
                    let args = if args.is_null() { json!({}) } else { args };
                    if mode == "error" {
                        emit(json!({
                            "jsonrpc": "2.0", "id": o["id"],
                            "error": { "code": -32601, "message": format!("Stub error: {name}") }
                        }));
                    } else {
                        emit(json!({
                            "jsonrpc": "2.0", "id": o["id"],
                            "result": { "content": [{ "type": "text",
                                "text": format!("STUB-DAEMON-OK {name} args={args}") }] }
                        }));
                    }
                }
                _ => {} // notifications/* ignored
            }
        }
        ExitCode::SUCCESS
    } else {
        // One-shot mode: drain stdin in the background, emit once, linger like the real server.
        std::thread::spawn(|| {
            let mut sink = Vec::new();
            let _ = std::io::Read::read_to_end(&mut std::io::stdin().lock(), &mut sink);
        });
        if mode != "initfail" {
            emit(json!({
                "jsonrpc": "2.0", "id": 1,
                "result": { "protocolVersion": "2024-11-05", "capabilities": {},
                            "serverInfo": { "name": "stub", "version": "0" } }
            }));
        }
        if mode == "success" {
            emit(json!({
                "jsonrpc": "2.0", "id": 2,
                "result": { "content": [{ "type": "text", "text": "STUB-OK wb_state edit 123" }] }
            }));
        } else if mode == "error" {
            emit(json!({
                "jsonrpc": "2.0", "id": 2,
                "error": { "code": -32601, "message": "Stub error: unknown tool" }
            }));
        }
        std::thread::sleep(Duration::from_secs_f64(linger));
        ExitCode::SUCCESS
    }
}

/* ───────────────────────────── broker ───────────────────────────── */

/// `.js`/`.mjs` entries run under node; anything else execs directly (T-165.7 — the Node
/// driver used to hardcode `node <entry>`, which broke native runners like the mcpd stub).
fn resolve_runner() -> (String, Vec<String>) {
    if let Ok(bin) = std::env::var("ENFUSION_MCP_BIN")
        && !bin.is_empty()
        && PathBuf::from(&bin).exists()
    {
        return if bin.ends_with(".js") || bin.ends_with(".mjs") {
            ("node".into(), vec![bin])
        } else {
            (bin, vec![])
        };
    }
    // Pinned node_modules entry relative to scripts/mod/ (the .mjs used import.meta.url;
    // the binary resolves from the repo root instead).
    let pinned =
        tbd_tools::serve::repo_root().join("scripts/mod/node_modules/enfusion-mcp/dist/index.js");
    if pinned.exists() {
        return ("node".into(), vec![pinned.to_string_lossy().into_owned()]);
    }
    ("npx".into(), vec!["-y".into(), "enfusion-mcp".into()])
}

struct Broker {
    sock: PathBuf,
    pidfile: PathBuf,
    call_ms: u64,
    child_stdin: Mutex<Option<ChildStdin>>,
    child_proc: Mutex<Option<Child>>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Value>>>,
    next_id: AtomicU64,
    call_queue: Mutex<()>,
    restarting: AtomicBool,
    last_activity: Mutex<std::time::Instant>,
}

impl Broker {
    fn cleanup_files(&self) {
        for p in [&self.sock, &self.pidfile] {
            let _ = std::fs::remove_file(p);
        }
    }

    async fn shutdown(&self, code: i32) -> ! {
        if let Some(mut c) = self.child_proc.lock().await.take() {
            let _ = c.start_kill();
        }
        self.cleanup_files();
        std::process::exit(code);
    }

    async fn reset_idle(&self) {
        *self.last_activity.lock().await = std::time::Instant::now();
    }

    /// Spawn + initialize the enfusion-mcp child; register the stdout reader.
    async fn start_child(self: &Arc<Self>) -> anyhow::Result<()> {
        let (prog, args) = resolve_runner();
        dlog!("spawning {prog} {}", args.join(" "));
        let mut child = Command::new(&prog)
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        let mut stdin = child.stdin.take().expect("child stdin");
        let stdout = child.stdout.take().expect("child stdout");
        let stderr = child.stderr.take().expect("child stderr");

        // stderr → debug log.
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                dlog!("child stderr: {}", l.trim());
            }
        });

        // stdout reader: match ids to pending waiters; on stream end → child exit handling.
        let me = Arc::clone(self);
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let Ok(o) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                if let Some(id) = o["id"].as_u64()
                    && let Some(tx) = me.pending.lock().await.remove(&id)
                {
                    let _ = tx.send(o);
                }
            }
            dlog!("child exited");
            me.on_child_exit().await;
        });

        // initialize (id 1) + initialized notification, then await the init reply.
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(1, tx);
        let init = json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2024-11-05", "capabilities": {},
                        "clientInfo": { "name": "tbd-daemon", "version": "1.0" } } });
        let inited = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        stdin
            .write_all(format!("{init}\n{inited}\n").as_bytes())
            .await?;
        *self.child_stdin.lock().await = Some(stdin);
        *self.child_proc.lock().await = Some(child);

        match tokio::time::timeout(Duration::from_millis(self.call_ms), rx).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(_)) => anyhow::bail!("child exited during init"),
            Err(_) => {
                self.pending.lock().await.remove(&1);
                anyhow::bail!("init timeout")
            }
        }
    }

    /// Boxed (`dyn`) future — `start_child` spawns a reader that re-enters this on child
    /// death, and the type-level cycle needs `dyn` erasure to stay finite.
    fn on_child_exit(self: &Arc<Self>) -> futures_util::future::BoxFuture<'static, ()> {
        let me = Arc::clone(self);
        Box::pin(async move {
            let waiters: Vec<_> = me.pending.lock().await.drain().collect();
            for (_, tx) in waiters {
                let _ = tx.send(json!({ "jsonrpc": "2.0", "error": { "code": -32000, "message": "child error" } }));
            }
            *me.child_stdin.lock().await = None;
            *me.child_proc.lock().await = None;
            if me.restarting.swap(true, Ordering::SeqCst) {
                return;
            }
            match me.start_child().await {
                Ok(()) => {
                    me.restarting.store(false, Ordering::SeqCst);
                    dlog!("child restarted");
                }
                Err(e) => {
                    dlog!("restart failed {e}");
                    me.shutdown(1).await;
                }
            }
        })
    }

    /// Serialized tool call. Always resolves to a JSON-RPC object with id==2.
    async fn call_tool(self: &Arc<Self>, tool: &str, args: &Value) -> Value {
        let _queued = self.call_queue.lock().await;
        let finish = |mut obj: Value| {
            obj["id"] = json!(2);
            obj
        };
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        dlog!("→child {id} name={tool} args={args}");
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        {
            let mut stdin_slot = self.child_stdin.lock().await;
            let Some(stdin) = stdin_slot.as_mut() else {
                self.pending.lock().await.remove(&id);
                return finish(json!({ "jsonrpc": "2.0",
                    "error": { "code": -32000, "message": "daemon child unavailable" } }));
            };
            let req = json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call",
                "params": { "name": tool, "arguments": args } });
            if stdin
                .write_all(format!("{req}\n").as_bytes())
                .await
                .is_err()
            {
                self.pending.lock().await.remove(&id);
                return finish(json!({ "jsonrpc": "2.0",
                    "error": { "code": -32000, "message": "child error" } }));
            }
        }
        match tokio::time::timeout(Duration::from_millis(self.call_ms), rx).await {
            Ok(Ok(obj)) => finish(obj),
            Ok(Err(_)) => finish(json!({ "jsonrpc": "2.0",
                "error": { "code": -32000, "message": "child error" } })),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                finish(json!({ "jsonrpc": "2.0",
                    "error": { "code": -32001, "message": "tool timeout" } }))
            }
        }
    }

    async fn handle_conn(self: Arc<Self>, conn: UnixStream) {
        self.reset_idle().await;
        let (read, mut write) = conn.into_split();
        let mut lines = BufReader::new(read).lines();
        let Ok(Some(line)) = lines.next_line().await else {
            return;
        };
        let Ok(req) = serde_json::from_str::<Value>(&line) else {
            return; // malformed → close (the .mjs conn.end())
        };
        let tool = req["tool"].as_str().unwrap_or_default().to_string();
        let args = if req["args"].is_null() {
            json!({})
        } else {
            req["args"].clone()
        };
        let resp = self.call_tool(&tool, &args).await;
        let _ = write.write_all(format!("{resp}\n").as_bytes()).await;
        let _ = write.shutdown().await;
        self.reset_idle().await;
    }
}

async fn run_broker(sock: PathBuf, pidfile: PathBuf) -> ExitCode {
    let idle_ms = env_secs("MCP_DAEMON_IDLE", 1800) * 1000;
    // Hard backstop: self-terminate after this even if "busy", so the daemon can never
    // linger/leak indefinitely. The next mcp-call.sh transparently restarts it. 0 = disabled.
    let max_life_ms = env_secs("MCP_DAEMON_MAX_LIFE", 14400) * 1000;
    let call_ms = env_secs("MCP_CALL_TIMEOUT", 180) * 1000;

    let broker = Arc::new(Broker {
        sock: sock.clone(),
        pidfile: pidfile.clone(),
        call_ms,
        child_stdin: Mutex::new(None),
        child_proc: Mutex::new(None),
        pending: Mutex::new(HashMap::new()),
        next_id: AtomicU64::new(100),
        call_queue: Mutex::new(()),
        restarting: AtomicBool::new(false),
        last_activity: Mutex::new(std::time::Instant::now()),
    });

    // SIGTERM / SIGINT → clean shutdown (kill child, unlink socket+pidfile).
    for signum in [
        tokio::signal::unix::SignalKind::terminate(),
        tokio::signal::unix::SignalKind::interrupt(),
    ] {
        let me = Arc::clone(&broker);
        let mut sig = tokio::signal::unix::signal(signum).expect("signal handler");
        tokio::spawn(async move {
            sig.recv().await;
            me.shutdown(0).await;
        });
    }

    if let Err(e) = broker.start_child().await {
        eprintln!("mcp-daemon: child init failed: {e}");
        broker.cleanup_files();
        return ExitCode::from(2);
    }

    if sock.exists() {
        let _ = std::fs::remove_file(&sock); // stale
    }
    let listener = match UnixListener::bind(&sock) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("mcp-daemon: server error {e}");
            broker.shutdown(1).await;
        }
    };
    dlog!("listening on {}", sock.display());
    let _ = std::fs::write(&pidfile, std::process::id().to_string());
    broker.reset_idle().await;

    if max_life_ms > 0 {
        let me = Arc::clone(&broker);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(max_life_ms)).await;
            dlog!("max lifetime reached");
            me.shutdown(0).await;
        });
    }
    if idle_ms > 0 {
        let me = Arc::clone(&broker);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if me.last_activity.lock().await.elapsed().as_millis() as u64 >= idle_ms {
                    dlog!("idle timeout");
                    me.shutdown(0).await;
                }
            }
        });
    }

    loop {
        match listener.accept().await {
            Ok((conn, _)) => {
                tokio::spawn(Arc::clone(&broker).handle_conn(conn));
            }
            Err(e) => {
                eprintln!("mcp-daemon: server error {e}");
                broker.shutdown(1).await;
            }
        }
    }
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    // Broker mode wins whenever --socket is present: the selftest launches the broker with
    // MCP_STUB=1 exported (so the broker's CHILD — spawned argv-less from ENFUSION_MCP_BIN —
    // resolves to the stub), and the flag must not capture the broker itself.
    let has_socket = argv.iter().any(|a| a == "--socket");
    if !has_socket
        && (std::env::var("MCP_STUB").as_deref() == Ok("1")
            || argv.first().map(String::as_str) == Some("--stub"))
    {
        return run_stub();
    }
    let get = |k: &str| {
        argv.iter()
            .position(|a| a == k)
            .and_then(|i| argv.get(i + 1))
            .cloned()
    };
    let Some(sock) = get("--socket").or_else(|| std::env::var("MCP_SOCK").ok()) else {
        eprintln!("mcp-daemon: --socket required");
        return ExitCode::from(2);
    };
    let pidfile = get("--pidfile").unwrap_or_else(|| format!("{sock}.pid"));
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(run_broker(PathBuf::from(sock), PathBuf::from(pidfile)))
}
