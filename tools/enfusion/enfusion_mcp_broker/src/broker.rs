//! The broker: one initialised `enfusion-mcp` server child behind a Unix socket.
//!
//! **Role:** spawns the server once, initialises it once (paying the ~35 s index load a single
//! time), and serves `tools/call` requests from the socket. Requests are serialised (the
//! Workbench NetAPI is single-stream); responses are matched by id and relabelled to `id == 2`
//! so `cargo xtask mcp consume` parses them unchanged. A child that exits is restarted.
//! **Position:** run by [`crate::command_entry::run`] on its tokio runtime; resolves the server
//! command through `enfusion_mcp::server_entrypoint`; `cargo xtask mcp socket-send` is its
//! client.
//! **Signals & state:** the [`Broker`] owns the child, its stdin, the pending-request table, the
//! call queue, the restart flag and the idle clock; the signal, idle and lifetime tasks and a
//! failed restart send stop requests to the accept loop, which alone ends the daemon.
//! **Invariants:** at most one request is in flight to the child; every answer has `id == 2`; a
//! stop kills the child and removes the socket and the pidfile before the exit code returns.
//!
//! ```text
//!   Env: ENFUSION_MCP_BIN (set by `cargo xtask mcp daemon` to the entry it resolved; see
//!        `enfusion_mcp::server_entrypoint`), MCP_DAEMON_IDLE (s, default 1800; 0=never),
//!        MCP_DAEMON_MAX_LIFE (s, default 14400; 0=disabled), MCP_CALL_TIMEOUT (s, default
//!        180), MCP_DEBUG=1.
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use enfusion_mcp::server_entrypoint;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, mpsc, oneshot};

use crate::error::{Error, Result};

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

/// The daemon's shared state, owned through an `Arc` by the accept loop and every task.
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
    /// Where a task asks the accept loop to stop the daemon, with the exit code.
    shutdown_requests: mpsc::UnboundedSender<u8>,
}

impl Broker {
    fn cleanup_files(&self) {
        for p in [&self.sock, &self.pidfile] {
            let _ = std::fs::remove_file(p);
        }
    }

    /// Asks the accept loop to stop the daemon with `code`; the loop kills the child, removes the
    /// socket and pidfile, and returns the code.
    fn request_shutdown(&self, code: u8) {
        let _ = self.shutdown_requests.send(code);
    }

    /// Kills the child, removes the socket and pidfile, and returns `code` as the exit code.
    async fn stop(&self, code: u8) -> ExitCode {
        if let Some(mut c) = self.child_proc.lock().await.take() {
            let _ = c.start_kill();
        }
        self.cleanup_files();
        ExitCode::from(code)
    }

    async fn reset_idle(&self) {
        *self.last_activity.lock().await = std::time::Instant::now();
    }

    /// Spawn + initialize the enfusion-mcp child; register the stdout reader.
    async fn start_child(self: &Arc<Self>) -> Result<()> {
        let runner = server_entrypoint::resolve(&repository_root::find_repository_root()?);
        let (prog, args) = (runner.program, runner.args);
        dlog!(
            "spawning {prog} {} ({})",
            args.join(" "),
            runner.source.label()
        );
        // tokio's process: the broker streams the server's stdio on its runtime for the daemon's
        // whole life, which the synchronous `process_runner` cannot host.
        let mut child = Command::new(&prog)
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or(Error::MissingPipe { stream: "stdin" })?;
        let stdout = child
            .stdout
            .take()
            .ok_or(Error::MissingPipe { stream: "stdout" })?;
        let stderr = child
            .stderr
            .take()
            .ok_or(Error::MissingPipe { stream: "stderr" })?;

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
            Ok(Err(_)) => Err(Error::ExitedDuringInit),
            Err(_) => {
                self.pending.lock().await.remove(&1);
                Err(Error::InitTimeout)
            }
        }
    }

    /// Boxed (`dyn`) future — `start_child` spawns a reader that re-enters this on child
    /// death, and the type-level cycle needs `dyn` erasure to stay finite.
    fn on_child_exit(self: &Arc<Self>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
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
                    me.request_shutdown(1);
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

/// Serves the socket `sock` until a stop request, and returns the daemon's exit code: 0 for a
/// signal or a reached limit, 1 for a socket failure or a failed restart, 2 for a failed first
/// start.
pub(crate) async fn run_broker(sock: PathBuf, pidfile: PathBuf) -> ExitCode {
    let idle_ms = env_secs("MCP_DAEMON_IDLE", 1800) * 1000;
    // Hard backstop: self-terminate after this even if "busy", so the daemon can never
    // linger/leak indefinitely. The next `mcp call` transparently restarts it. 0 = disabled.
    let max_life_ms = env_secs("MCP_DAEMON_MAX_LIFE", 14400) * 1000;
    let call_ms = env_secs("MCP_CALL_TIMEOUT", 180) * 1000;

    let (shutdown_requests, mut shutdown_received) = mpsc::unbounded_channel();
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
        shutdown_requests,
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
            me.request_shutdown(0);
        });
    }

    // The first start can take the whole call timeout; a stop request ends the daemon at once.
    let started = tokio::select! {
        started = broker.start_child() => started,
        code = shutdown_received.recv() => return broker.stop(code.unwrap_or(1)).await,
    };
    if let Err(e) = started {
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
            return broker.stop(1).await;
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
            me.request_shutdown(0);
        });
    }
    if idle_ms > 0 {
        let me = Arc::clone(&broker);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if me.last_activity.lock().await.elapsed().as_millis() as u64 >= idle_ms {
                    dlog!("idle timeout");
                    me.request_shutdown(0);
                    return;
                }
            }
        });
    }

    loop {
        tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((conn, _)) => {
                    tokio::spawn(Arc::clone(&broker).handle_conn(conn));
                }
                Err(e) => {
                    eprintln!("mcp-daemon: server error {e}");
                    return broker.stop(1).await;
                }
            },
            code = shutdown_received.recv() => return broker.stop(code.unwrap_or(1)).await,
        }
    }
}
