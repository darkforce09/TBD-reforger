//! `cargo xtask mcp call` — one Workbench tool call, through the daemon when it is up.
//!
//! Exit codes, pinned by `cargo xtask mcp selftest`: 0 success · 1 usage or empty after every
//! retry · 2 initialize failed (no checkout to start the server from included) · 3 JSON-RPC tool error · 4 timeout. Internal 9 means "the daemon
//! is unavailable, run the call one-shot".
//!
//! Three failures deliberately do not stop the call, because none of them says the tool call
//! cannot succeed:
//! - a lock that cannot be taken within 65 s — the lock only serialises daemon startup;
//! - an absent or unreadable npx download cache — the pinned package usually answers first;
//! - a daemon `status` or `start` that fails — the one-shot path still reaches the server.
//!
//! One failure does stop the attempt: a missing `timeout(1)`. The alternative is a call that
//! hangs for the whole `MCP_CALL_TIMEOUT` wall clock behind a racy kill thread, so the attempt
//! fails closed instead and maps to the empty-result code.
//!
//! **Role:** runs one tool call: through the `mcpd` broker (`socket-send | consume`) when a
//! daemon is up or can be started, else one-shot (`timeout <server> | consume`) with retries.
//! **Position:** called by [`crate::dispatch`] for `call`; starts the broker through
//! [`crate::daemon`], resolves the one-shot server through [`crate::server_entrypoint`], and
//! spawns this executable's `mcp socket-send` and `mcp consume` through `process_runner`.
//! **Signals & state:** exports the Enfusion path defaults and `MCP_SOCK` into this process's
//! environment before it spawns any child; holds the socket's start lock while it starts a
//! broker.
//! **Invariants:** the exit codes above; each pipe runs as two children in turn, the first one's
//! whole stdout becoming the second one's stdin, so no child blocks on a full pipe.

use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use process_runner::Run;
use verification_core::lock::flock_exclusive;

use crate::server_entrypoint;

/// The usage line `cargo xtask mcp selftest` greps for.
const USAGE: &str = "usage: cargo xtask mcp call <tool> '<json-args>'";

/// Entry for `xtask mcp call [tool] [args-json]`.
// The environment writes below run before this call spawns any thread or child, which is what
// `std::env::set_var` requires.
#[allow(unsafe_code)]
pub(crate) fn run(tool: Option<String>, args_json: Option<String>) -> i32 {
    let Some(tool) = tool.filter(|t| !t.is_empty()) else {
        eprintln!("{USAGE}");
        return 1;
    };
    // An absent or empty argument object is the empty JSON object.
    let args = match args_json {
        None => "{}".to_string(),
        Some(s) if s.is_empty() => "{}".to_string(),
        Some(s) => s,
    };

    export_enfusion_defaults();
    let sock = crate::daemon::resolve_sock();
    // SAFETY: set before any thread is spawned. The daemon and the `socket-send` helper read
    // MCP_SOCK from the environment they inherit.
    unsafe { env::set_var("MCP_SOCK", &sock) };

    let mut rc = daemon_try(&tool, &args, &sock);
    if rc == 9 {
        rc = oneshot(&tool, &args);
    }
    rc
}

fn export_enfusion_defaults() {
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    if let Ok(root) = repository_root() {
        set_default(
            "ENFUSION_GAME_PATH",
            &root
                .join(repository_layout::ENFUSION_MCP_GAME_ROOT)
                .display()
                .to_string(),
        );
    }
    set_default(
        "ENFUSION_WORKBENCH_PATH",
        &format!("{home}/.local/share/Steam/steamapps/common/Arma Reforger Tools"),
    );
    set_default(
        "ENFUSION_PROJECT_PATH",
        &format!("{home}/Documents/Games/ArmaReforgerWorkbench/addons"),
    );
}

// Called from `run` before any thread or child exists, as `std::env::set_var` requires.
#[allow(unsafe_code)]
fn set_default(key: &str, val: &str) {
    if env::var_os(key).is_none() {
        unsafe { env::set_var(key, val) };
    }
}

fn xtask_bin() -> PathBuf {
    env::current_exe().unwrap_or_else(|_| PathBuf::from("xtask"))
}

fn dbg(msg: &str) {
    if env::var("MCP_DEBUG").ok().as_deref() == Some("1") {
        eprintln!("[mcp-call] {msg}");
    }
}

fn timeout_secs() -> u64 {
    env::var("MCP_CALL_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(180)
}

fn retries() -> u32 {
    env::var("MCP_CALL_RETRIES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

fn emit_requests(tool: &str, args: &str) -> String {
    let call = format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"{tool}","arguments":{args}}}}}"#
    );
    format!(
        "{}\n{}\n{}\n",
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"cc","version":"1.0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        call
    )
}

/// The argv that starts an `enfusion-mcp` server for the one-shot path.
fn resolve_runner() -> repository_root::Result<Vec<String>> {
    let command = server_entrypoint::resolve(&repository_root()?);
    dbg(&format!("runner={}", command.source.label()));
    Ok(command.argv())
}

/// The checkout to resolve the pinned server package against.
///
/// The cwd walk answers for the worktree the call is being made from. When it cannot (the call
/// was made from outside a checkout), the walk from the crate's own manifest folder locates the
/// repository that built this binary; when neither finds a root, the call cannot start a server.
fn repository_root() -> repository_root::Result<PathBuf> {
    repository_root::find_repository_root().or_else(|_| {
        repository_root::find_repository_root_from(Path::new(env!("CARGO_MANIFEST_DIR")))
    })
}

fn ensure_daemon(sock: &str) -> bool {
    if env::var("MCP_NO_DAEMON").ok().as_deref() == Some("1") {
        return false;
    }
    if crate::daemon::is_running_at(sock) {
        return true;
    }
    // One lock file per socket serialises the start, so two concurrent calls do not each spawn a
    // broker for the same socket.
    let lock_path = format!("{sock}.lock");
    // A lock that cannot be taken must not block the call: the daemon may already be starting.
    let _held = flock_exclusive(
        Path::new(&lock_path),
        Duration::from_secs(1),
        Duration::from_secs(65),
        |_| {},
    )
    .ok();
    if !crate::daemon::is_running_at(sock) {
        let _ = crate::daemon::start_at(sock, true);
    }
    drop(_held);
    crate::daemon::is_running_at(sock)
}

/// 0 success · 3 tool error · 9 fall-back-to-oneshot
fn daemon_try(tool: &str, args: &str, sock: &str) -> i32 {
    if !ensure_daemon(sock) {
        dbg("daemon unavailable");
        return 9;
    }
    dbg(&format!("daemon_try TOOL=[{tool}] ARGS=[{args}]"));
    let xtask = xtask_bin();

    // `socket-send` prints one response line and exits, so its whole stdout is what `consume`
    // reads: the two run in turn, as the pipe `socket-send | consume`. A child that could not
    // be started or died on a signal falls back to the one-shot path.
    let Ok(send) = Run::new(&xtask)
        .args(["mcp", "socket-send", sock, tool, args])
        .output()
    else {
        return 9;
    };
    let Ok(consume) = Run::new(&xtask)
        .args(["mcp", "consume"])
        .stdin(send.stdout)
        .output()
    else {
        return 9;
    };
    // `consume` writes a tool error to stderr; it reaches the caller as it would from a child
    // sharing this process's stderr.
    let _ = io::stderr().write_all(consume.stderr.as_bytes());
    let (send_rc, consume_rc) = (send.code, consume.code);
    dbg(&format!("daemon send_rc={send_rc} consume_rc={consume_rc}"));

    if send_rc == 0 && consume_rc == 0 {
        print_stdout(&consume.stdout);
        0
    } else if send_rc == 0 && consume_rc == 3 {
        3
    } else {
        if env::var("MCP_DEBUG").ok().as_deref() == Some("1") {
            let _ = io::stderr().write_all(send.stderr.as_bytes());
        }
        9
    }
}

fn oneshot(tool: &str, args: &str) -> i32 {
    let runner = match resolve_runner() {
        Ok(runner) => runner,
        Err(cause) => {
            eprintln!("mcp call: no checkout to start the enfusion-mcp server from: {cause}");
            return 2;
        }
    };
    let timeout = timeout_secs();
    let max_retries = retries();
    let mut attempt = 0u32;
    loop {
        let outcome = oneshot_pipe(&runner, tool, args, timeout);
        let (to_rc, consume_rc) = (outcome.timeout_code, outcome.consume_code);
        dbg(&format!(
            "oneshot attempt={attempt} to_rc={to_rc} consume_rc={consume_rc}"
        ));
        let code = if to_rc == 124 {
            4
        } else if consume_rc == 0 {
            print_stdout(&outcome.result);
            return 0;
        } else if consume_rc == 3 {
            return 3;
        } else if consume_rc == 2 {
            2
        } else {
            1
        };
        attempt += 1;
        if attempt > max_retries {
            let _ = io::stderr().write_all(outcome.server_stderr.as_bytes());
            return code;
        }
        dbg(&format!(
            "retry ({attempt}/{max_retries}) after code={code}"
        ));
    }
}

/// What one one-shot attempt produced.
struct OneshotOutcome {
    /// The exit code of `timeout(1)`: 124 when the server ran out of time.
    timeout_code: i32,
    /// The exit code of `consume`.
    consume_code: i32,
    /// What `consume` printed: the tool result on success.
    result: String,
    /// What the server wrote to stderr.
    server_stderr: String,
}

impl OneshotOutcome {
    /// An attempt that could not run; it maps to the empty-result code.
    fn not_run() -> OneshotOutcome {
        OneshotOutcome {
            timeout_code: 1,
            consume_code: 1,
            result: String::new(),
            server_stderr: String::new(),
        }
    }
}

/// `emit | timeout RUNNER | xtask mcp consume`, the server's stderr kept apart.
///
/// The server reads the requests to their end and exits, so its whole stdout is what `consume`
/// reads: the two run in turn. A server or `consume` that could not be started or died on a
/// signal makes the attempt one that did not run.
fn oneshot_pipe(runner: &[String], tool: &str, args: &str, timeout: u64) -> OneshotOutcome {
    // Without `timeout(1)` an unresponsive server hangs this attempt for the whole wall clock.
    if process_runner::which("timeout").is_err() {
        dbg("timeout(1) absent — oneshot attempt fails closed");
        return OneshotOutcome::not_run();
    }
    let reqs = emit_requests(tool, args);
    let Ok(server) = Run::new("timeout")
        .arg(timeout.to_string())
        .args(runner)
        .stdin(reqs)
        .output()
    else {
        return OneshotOutcome::not_run();
    };
    let Ok(consume) = Run::new(xtask_bin())
        .args(["mcp", "consume"])
        .stdin(server.stdout)
        .output()
    else {
        return OneshotOutcome::not_run();
    };
    // `consume` writes a tool error to stderr; it reaches the caller as it would from a child
    // sharing this process's stderr.
    let _ = io::stderr().write_all(consume.stderr.as_bytes());
    OneshotOutcome {
        timeout_code: server.code,
        consume_code: consume.code,
        result: consume.stdout,
        server_stderr: server.stderr,
    }
}

fn print_stdout(text: &str) {
    let mut stdout = io::stdout();
    let _ = stdout.write_all(text.as_bytes());
    let _ = stdout.flush();
}
