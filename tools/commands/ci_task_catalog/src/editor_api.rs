//! Boot the API for editor-gates.yml and wait on `/healthz`; the editorconfig and codegen
//! freshness steps.
//!
//! **Role:** the `editor-api-boot`, `verify-editorconfig` and `verify-codegen-fresh` rows' steps.
//! **Position:** [`crate::task_runner::Step::Native`] steps of [`crate::task_runner::TASKS`];
//! over `schema_tooling` for the freshness check and `process_runner` for the build, the
//! detached server and the checker run.
//! **Signals & state:** the API boot leaves a detached server running and writes its log to
//! `/tmp/api.log`; the editorconfig step prepends the Go bin folders to this process's `PATH`.
//! **Invariants:** the boot fails when the child cannot start or `/healthz` is still down after
//! 60 s, never reading a spawn failure as "starting"; a missing editorconfig checker is a
//! did-not-run, never a pass.
//!
//! The boot starts the server detached, in a new session, so the GitHub Actions step teardown
//! does not reap it before later steps.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use process_runner::Run;
use verification_core::NotRun;

const LOG: &str = "/tmp/api.log";
const TRIES: u32 = 60;

/// `cargo xtask ci editor-api-boot`: 0 once `/healthz` answers, 1 with the API log's tail.
pub(crate) fn run() -> i32 {
    match run_inner() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("editor-api-boot: {e}");
            if let Ok(text) = fs::read_to_string(LOG) {
                let lines: Vec<&str> = text.lines().collect();
                let start = lines.len().saturating_sub(40);
                for l in &lines[start..] {
                    eprintln!("{l}");
                }
            }
            1
        }
    }
}

fn run_inner() -> Result<(), String> {
    let root = repository_root::find_repository_root().map_err(|e| e.to_string())?;
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    // On the inherited terminal: the build's progress streams as it happens.
    let code = match Run::new("cargo")
        .args(["build", "-p", "api_server", "--bin", "api-server"])
        .cwd(&root)
        .terminal()
    {
        Ok(code) => code,
        // A signal is a failed build with no exit code, as `ExitStatus::code` reads it.
        Err(NotRun::Signalled { .. }) => 1,
        Err(e) => return Err(format!("cargo build: {e}")),
    };
    if code != 0 {
        return Err(format!("cargo build exited {code}"));
    }

    let log = File::create(LOG).map_err(|e| format!("{LOG}: {e}"))?;
    let log_err = log.try_clone().map_err(|e| format!("{LOG} clone: {e}"))?;
    // A detached child in a new session, so the Actions step's process-group kill does not reap
    // the API before leptos-gates runs. The YAML used `(cmd &)` for the same reason.
    let pid = Run::new("cargo")
        .args(["run", "-q", "-p", "api_server", "--bin", "api-server"])
        .cwd(&root)
        .spawn_detached_to_files(log, log_err)
        .map_err(|e| format!("cargo run api-server: {e}"))?;
    println!("editor-api-boot: spawned pid {pid}, waiting on :{port}/healthz");

    let deadline = Instant::now() + Duration::from_secs(TRIES as u64);
    while Instant::now() < deadline {
        if healthz(port) {
            println!("api up");
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    Err("API never came up".into())
}

fn healthz(port: u16) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut s = match TcpStream::connect_timeout(&address, Duration::from_secs(2)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = s.set_write_timeout(Some(Duration::from_secs(2)));
    if s.write_all(b"GET /healthz HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut buf = [0u8; 256];
    let n = s.read(&mut buf).unwrap_or(0);
    let resp = String::from_utf8_lossy(&buf[..n]);
    resp.contains("200")
}

/// Regenerate in memory and compare every contract output, including untracked files.
pub(crate) fn verify_codegen_fresh() -> i32 {
    let result = repository_root::find_repository_root()
        .map_err(crate::Error::from)
        .and_then(|root| Ok(schema_tooling::verify_fresh(&root)?));
    match result {
        Ok(()) => {
            println!("verify-codegen-fresh: PASS");
            0
        }
        Err(error) => {
            eprintln!("verify-codegen-fresh: {}", crate::cause_chain(&error));
            1
        }
    }
}

/// FMT-2: run editorconfig-checker from repo root, installing the pinned Go binary if needed.
///
/// CI no longer uses `actions/setup-go`. GitHub-hosted runners still ship `go`; locally the
/// binary already lives in `~/go/bin` (mk_ci PATH prepend). A missing tool is DidNotRun, never OK.
pub(crate) fn verify_editorconfig() -> i32 {
    let root = match repository_root::find_repository_root() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("xtask: {e:#}");
            return 1;
        }
    };
    prepend_go_bins();
    let bin = match ensure_editorconfig_checker() {
        Ok(p) => p,
        Err(nr) => {
            eprintln!(
                "{}",
                verification_core::Verdict::did_not_run(
                    "verify-editorconfig",
                    verification_core::verdict::Kind::Pin,
                    nr,
                )
            );
            return 2;
        }
    };
    match process_runner::Run::new(&bin).cwd(&root).output() {
        Ok(out) => {
            print!("{}", out.stdout);
            eprint!("{}", out.stderr);
            if out.code == 0 { 0 } else { 1 }
        }
        Err(nr) => {
            eprintln!(
                "{}",
                verification_core::Verdict::did_not_run(
                    "verify-editorconfig",
                    verification_core::verdict::Kind::Pin,
                    nr,
                )
            );
            2
        }
    }
}

const EDITORCONFIG_PIN: &str =
    "github.com/editorconfig-checker/editorconfig-checker/v3/cmd/editorconfig-checker@v3.4.0";

fn prepend_go_bins() {
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let home = PathBuf::from(home);
    let inherited = std::env::var("PATH").unwrap_or_default();
    let extra = format!(
        "{}:{}:{inherited}",
        home.join("go/bin").display(),
        home.join(".local/go/bin").display()
    );
    // SAFETY: the editorconfig step runs on the runner's one thread before it spawns anything,
    // so no other thread reads the environment meanwhile, which `set_var` requires.
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("PATH", extra)
    };
}

fn ensure_editorconfig_checker() -> Result<PathBuf, verification_core::NotRun> {
    if let Ok(p) = process_runner::which("editorconfig-checker") {
        return Ok(p);
    }
    match process_runner::Run::new("go")
        .args(["install", EDITORCONFIG_PIN])
        .status()
    {
        Ok(0) => process_runner::which("editorconfig-checker"),
        Ok(code) => Err(verification_core::NotRun::ToolError {
            tool: "go install editorconfig-checker".into(),
            status: code,
            stderr: String::new(),
        }),
        Err(nr) => Err(nr),
    }
}
