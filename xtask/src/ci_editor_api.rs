//! T-901 — boot website-api for editor-gates.yml and wait on `/healthz`.
//!
//! The workflow used to background `cargo run` with `|| true`, then `seq`/`curl` in a loop.
//! `|| true` is the forbidden shape: a spawn failure would read as "API is starting". This
//! wrapper fails if the child cannot start or if healthz is still down after 60 s, and it
//! `setsid`s so the GitHub Actions step teardown does not reap the server before later steps.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const LOG: &str = "/tmp/api.log";
const TRIES: u32 = 60;

pub fn run() -> i32 {
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
    let root = crate::root::find_repo_root().map_err(|e| e.to_string())?;
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    let st = Command::new("cargo")
        .args(["build", "-p", "website-api", "--bin", "api"])
        .current_dir(&root)
        .status()
        .map_err(|e| format!("cargo build: {e}"))?;
    if !st.success() {
        return Err(format!("cargo build exited {}", st.code().unwrap_or(1)));
    }

    let log = File::create(LOG).map_err(|e| format!("{LOG}: {e}"))?;
    let log_err = log.try_clone().map_err(|e| format!("{LOG} clone: {e}"))?;
    let mut child = Command::new("cargo");
    child
        .args(["run", "-q", "-p", "website-api", "--bin", "api"])
        .current_dir(&root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));
    // SAFETY: new session so the Actions step's process-group kill does not reap the API
    // before leptos-gates runs. The YAML used `(cmd &)` for the same reason.
    unsafe {
        child.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let child = child.spawn().map_err(|e| format!("cargo run api: {e}"))?;
    let pid = child.id();
    std::mem::forget(child);
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
    let mut s = match TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_secs(2),
    ) {
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

/// `git diff --exit-code` over the generated contract types — the contracts.yml stale-output pin.
pub fn verify_codegen_fresh() -> i32 {
    let root = match crate::root::find_repo_root() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("xtask: {e:#}");
            return 1;
        }
    };
    match tbd_gate::proc::Run::new("git")
        .args([
            "diff",
            "--exit-code",
            "--",
            "apps/website/api/src/contract/generated",
        ])
        .cwd(&root)
        .output()
    {
        Ok(out) if out.code == 0 => 0,
        Ok(out) => {
            print!("{}", out.stdout);
            eprint!("{}", out.stderr);
            eprintln!("verify-codegen-fresh: generated contract output is stale");
            1
        }
        Err(nr) => {
            eprintln!(
                "{}",
                tbd_gate::Verdict::did_not_run(
                    "verify-codegen-fresh",
                    tbd_gate::verdict::Kind::Pin,
                    nr,
                )
            );
            2
        }
    }
}

/// FMT-2: run editorconfig-checker from repo root, installing the pinned Go binary if needed.
///
/// CI no longer uses `actions/setup-go`. GitHub-hosted runners still ship `go`; locally the
/// binary already lives in `~/go/bin` (mk_ci PATH prepend). A missing tool is DidNotRun, never OK.
pub fn verify_editorconfig() -> i32 {
    let root = match crate::root::find_repo_root() {
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
                tbd_gate::Verdict::did_not_run(
                    "verify-editorconfig",
                    tbd_gate::verdict::Kind::Pin,
                    nr,
                )
            );
            return 2;
        }
    };
    match tbd_gate::proc::Run::new(&bin).cwd(&root).output() {
        Ok(out) => {
            print!("{}", out.stdout);
            eprint!("{}", out.stderr);
            if out.code == 0 { 0 } else { 1 }
        }
        Err(nr) => {
            eprintln!(
                "{}",
                tbd_gate::Verdict::did_not_run(
                    "verify-editorconfig",
                    tbd_gate::verdict::Kind::Pin,
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
    unsafe { std::env::set_var("PATH", extra) };
}

fn ensure_editorconfig_checker() -> Result<PathBuf, tbd_gate::NotRun> {
    if let Ok(p) = tbd_gate::proc::which("editorconfig-checker") {
        return Ok(p);
    }
    match tbd_gate::proc::Run::new("go")
        .args(["install", EDITORCONFIG_PIN])
        .status()
    {
        Ok(0) => tbd_gate::proc::which("editorconfig-checker"),
        Ok(code) => Err(tbd_gate::NotRun::ToolError {
            tool: "go install editorconfig-checker".into(),
            status: code,
            stderr: String::new(),
        }),
        Err(nr) => Err(nr),
    }
}
