//! T-889 — port of `scripts/platform/preflight.sh` → `cargo xtask platform preflight`.
//!
//! Unattended factory assertions; ANSI ✓ / ✗ BLOCK / ! WARN  + summary match bash.
//! Disk/memory lines are wall-clock noisy (T-853 §Non-reproducible). `hostrun cargo` is
//! obsolete (build-essential in-container); cargo/ticket/slice-collisions run direct. Host
//! bridge + API `ss`/`stat`/`date` still use distrobox-host-exec when containerised.
//! Fixture override: `TBD_PREFLIGHT_ROOT`.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};

use crate::root::find_repo_root;

struct Counters {
    block: u32,
    warn: u32,
}

fn ok(label: &str, detail: &str) {
    println!("  \x1b[32m✓\x1b[0m {label:<34} {detail}");
}

fn nope(c: &mut Counters, label: &str, detail: &str) {
    println!("  \x1b[31m✗ BLOCK\x1b[0m {label:<28} {detail}");
    c.block += 1;
}

fn soft(c: &mut Counters, label: &str, detail: &str) {
    println!("  \x1b[33m! WARN \x1b[0m {label:<28} {detail}");
    c.warn += 1;
}

fn in_container() -> bool {
    Path::new("/run/.containerenv").is_file()
        || Path::new("/.dockerenv").is_file()
        || env::var_os("container").is_some()
}

fn has_distrobox_host_exec() -> bool {
    Command::new("sh")
        .args(["-c", "command -v distrobox-host-exec >/dev/null 2>&1"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Wrap only when containerised AND distrobox-host-exec is on PATH.
fn use_host_bridge() -> bool {
    has_distrobox_host_exec() && in_container()
}

fn hostrun(args: &[&str]) -> Command {
    if use_host_bridge() {
        let mut c = Command::new("distrobox-host-exec");
        for a in args {
            c.arg(a);
        }
        c
    } else {
        let mut c = Command::new(args[0]);
        for a in &args[1..] {
            c.arg(a);
        }
        c
    }
}

fn capture_stdout(cmd: &mut Command) -> Option<String> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

fn status_ok(cmd: &mut Command) -> bool {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

fn resolve_root() -> Result<PathBuf> {
    if let Some(p) = env::var_os("TBD_PREFLIGHT_ROOT") {
        return Ok(PathBuf::from(p));
    }
    // Prefer $PWD (logical path) so dual-homed hosts (/home vs /var/home) match bash `cd … && pwd`.
    if let Some(pwd) = env::var_os("PWD") {
        let p = PathBuf::from(pwd);
        if p.join(".ai/tickets/ROOT").is_file() || p.join(".ai/tickets/registry.json").is_file() {
            return Ok(p);
        }
    }
    find_repo_root()
}

fn free_gb(root: &Path) -> Option<u64> {
    let out = Command::new("df")
        .args(["-BG", "--output=avail"])
        .arg(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let last = text.lines().last().unwrap_or("");
    let digits: String = last.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn orphan_cache_mb() -> u64 {
    // bash: du -sm /var/tmp/*target* /var/tmp/v2-* | awk sum
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = fs::read_dir("/var/tmp") {
        for ent in rd.flatten() {
            let name = ent.file_name();
            let s = name.to_string_lossy();
            if s.contains("target") || s.starts_with("v2-") {
                paths.push(ent.path());
            }
        }
    }
    if paths.is_empty() {
        return 0;
    }
    let mut cmd = Command::new("du");
    cmd.arg("-sm");
    for p in &paths {
        cmd.arg(p);
    }
    cmd.stderr(Stdio::null());
    let out = match cmd.output() {
        Ok(o) => o,
        Err(_) => return 0,
    };
    let mut sum: u64 = 0;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let Some(first) = line.split_whitespace().next() {
            if let Ok(n) = first.parse::<u64>() {
                sum += n;
            }
        }
    }
    sum
}

fn mem_available_mib() -> Option<u64> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb / 1024);
        }
    }
    None
}

fn swap_used_pct() -> Option<u64> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    let mut total: Option<u64> = None;
    let mut free: Option<u64> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("SwapTotal:") {
            total = rest.split_whitespace().next()?.parse().ok();
        } else if let Some(rest) = line.strip_prefix("SwapFree:") {
            free = rest.split_whitespace().next()?.parse().ok();
        }
    }
    let (t, f) = (total?, free?);
    if t == 0 {
        return None;
    }
    Some(((t - f) * 100) / t)
}

fn git_out(root: &Path, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(root);
    capture_stdout(&mut cmd)
}

fn git_status_porcelain(root: &Path) -> Option<String> {
    git_out(root, &["status", "--porcelain"])
}

fn count_pgrep(pattern: &str) -> u64 {
    // Count pgrep -f lines (not -fc: prints 0 and exits 1).
    let out = Command::new("pgrep")
        .args(["-f", pattern])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .count() as u64,
        Err(_) => 0,
    }
}

fn tcp_up(addr: &str) -> bool {
    let Ok(mut addrs) = addr.to_socket_addrs() else {
        return false;
    };
    let Some(sa) = addrs.next() else {
        return false;
    };
    TcpStream::connect_timeout(&sa, Duration::from_secs(1)).is_ok()
}

fn curl_http_code(url: &str) -> String {
    let out = Command::new("curl")
        .args([
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "-m",
            "4",
            url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => String::new(),
    }
}

/// Open-ticket count straight from the committed lock (waves 1+); `None` when the lock is
/// missing or unreadable — the caller BLOCKs on that via `wave check` anyway.
fn wave_lock_open_count(root: &Path) -> Option<(usize, usize)> {
    let lock = crate::wave_lock::load(root).ok()?;
    let open: usize = lock
        .waves
        .iter()
        .filter(|w| w.n > 0)
        .map(|w| w.tickets.len())
        .sum();
    let waves = lock.waves.iter().filter(|w| w.n > 0).count();
    Some((open, waves))
}

fn stray_worktree_targets(root: &Path) -> u64 {
    let base = root.join(".ai/artifacts/worktrees");
    let mut n = 0u64;
    let Ok(rd) = fs::read_dir(&base) else {
        return 0;
    };
    for ent in rd.flatten() {
        let target = ent.path().join("target");
        if target.is_dir() {
            n += 1;
        }
    }
    n
}

// ── T-300: THE RUN TARGET'S PROVENANCE ───────────────────────────────────────────────────────
//
// `stray_worktree_targets` above answers "did a worktree build into its own `target/`?" — a disk
// question. This answers the one that cost wave 1 a day: "is the binary a run lane is about to
// launch the code that is actually on main?" Cargo cannot answer it. Its `-C metadata` hash does
// not include the manifest path, so two checkouts of one package write the same artifact and the
// same uplifted `<profile>/<bin>`, and freshness is mtime-keyed, so the second build is satisfied
// by the first and prints `Finished` with no `Compiling` line. MEASURED 2026-09-06 (T-300):
// a worktree built `UNMERGED-SLICE-CODE`, the main checkout's `cargo run` then printed it.
//
// So `cargo xtask platform wave run` writes `tbd-built-from` (`<sha> <checkout>`) beside the
// binaries, and this reads it back. THREE answers, and only one of them is green: agreement,
// disagreement, and NO STAMP — because binaries whose provenance is unknown are exactly the
// case the wave-1 incident presented as, and treating unknown as fine is the signature defect.

/// Cargo's two profile directories, in the order this check reports them.
const RUN_PROFILE_DIRS: &[&str] = &["debug", "release"];

/// The executables in a profile directory, sorted — what this check NAMES when it blocks.
///
/// Regular files with an execute bit and no extension: cargo's uplifted-binary shape, which
/// excludes `.d` depfiles, the stamp, `.rlib`/`.rmeta` and the `deps/ build/ incremental/`
/// subdirectories without enumerating them. A sibling of [`stray_worktree_targets`] above rather
/// than of the writer in [`crate::wave`]: it is a probe of a directory, and the reader is the
/// only caller.
fn run_binaries(bin_dir: &Path) -> Vec<String> {
    use std::os::unix::fs::PermissionsExt;
    let mut out: Vec<String> = Vec::new();
    let Ok(rd) = fs::read_dir(bin_dir) else {
        return out;
    };
    for ent in rd.flatten() {
        let Ok(md) = ent.metadata() else { continue };
        if !md.is_file() || md.permissions().mode() & 0o111 == 0 {
            continue;
        }
        let name = ent.file_name().to_string_lossy().into_owned();
        if name.contains('.') || name == crate::wave::RUN_STAMP_FILE {
            continue;
        }
        out.push(name);
    }
    out.sort();
    out
}

/// What the run target's stamp says about the binaries sitting in it.
#[derive(Debug, PartialEq, Eq)]
enum RunTargetState {
    /// No run target on disk, or no binaries in it. Nothing can be stale.
    Empty,
    /// Stamped, and the stamp names this checkout at HEAD.
    Fresh { profile: String, bins: usize },
    /// Binaries with no readable `tbd-built-from` beside them.
    Unstamped { profile: String, bins: Vec<String> },
    /// Stamped, and the stamp disagrees with HEAD or with this checkout.
    Stale {
        profile: String,
        bins: Vec<String>,
        stamp: crate::wave::RunStamp,
    },
}

/// Read every profile directory in the run target and report the FIRST one that is not green.
///
/// First-bad rather than a summary: preflight's contract is one line per check, and the operator
/// needs the path to delete, not a census. `head` empty (git could not answer) is itself a
/// disagreement — a preflight that cannot resolve HEAD cannot certify anything.
fn run_target_state(run_dir: &Path, head: &str, this_checkout: &Path) -> RunTargetState {
    let mut fresh: Option<RunTargetState> = None;
    for profile in RUN_PROFILE_DIRS {
        let bin_dir = run_dir.join(profile);
        let bins = run_binaries(&bin_dir);
        if bins.is_empty() {
            continue;
        }
        match crate::wave::read_run_stamp(&bin_dir) {
            None => {
                return RunTargetState::Unstamped {
                    profile: (*profile).to_string(),
                    bins,
                };
            }
            Some(stamp) => {
                let agrees = !head.is_empty()
                    && stamp.sha == head
                    && Path::new(&stamp.checkout) == this_checkout;
                if !agrees {
                    return RunTargetState::Stale {
                        profile: (*profile).to_string(),
                        bins,
                        stamp,
                    };
                }
                if fresh.is_none() {
                    fresh = Some(RunTargetState::Fresh {
                        profile: (*profile).to_string(),
                        bins: bins.len(),
                    });
                }
            }
        }
    }
    fresh.unwrap_or(RunTargetState::Empty)
}

/// `(is_block, detail)` — the detail names the stale binary AND the checkout that built it AND
/// the command that clears it, because a preflight line the operator cannot act on is a warning
/// they will learn to scroll past.
fn run_target_detail(state: &RunTargetState, run_dir: &Path, head: &str) -> (bool, String) {
    let d = run_dir.display();
    match state {
        RunTargetState::Empty => (false, format!("{d} — no run binaries built yet")),
        RunTargetState::Fresh { profile, bins } => (
            false,
            format!(
                "{d}/{profile} — {bins} binary(ies) built from HEAD {}",
                short(head)
            ),
        ),
        RunTargetState::Unstamped { profile, bins } => (
            true,
            format!(
                "{d}/{profile}/{} has no {} — provenance unknown; cargo clean --target-dir {d}",
                bins.join(","),
                crate::wave::RUN_STAMP_FILE,
            ),
        ),
        RunTargetState::Stale {
            profile,
            bins,
            stamp,
        } => (
            true,
            format!(
                "{d}/{profile}/{} was built from {} by {} — HEAD is {}; cargo clean --target-dir {d}",
                bins.join(","),
                short(&stamp.sha),
                stamp.checkout,
                short(head),
            ),
        ),
    }
}

/// First 7 of a sha, or `(unresolved)` when git could not answer — never an empty string mid
/// sentence, which is how `wave::short` renders a failure and how a message loses its subject.
fn short(sha: &str) -> String {
    if sha.len() < 7 {
        return "(unresolved)".to_string();
    }
    sha[..7].to_string()
}

fn worktree_paths(root: &Path) -> Vec<PathBuf> {
    let out = Command::new("git")
        .args(["worktree", "list"])
        .current_dir(root)
        .output();
    let Ok(o) = out else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&o.stdout);
    text.lines()
        .skip(1) // drop primary
        .filter_map(|l| l.split_whitespace().next().map(PathBuf::from))
        .collect()
}

fn is_git_young(path: &Path, idle_min: u64) -> bool {
    // find .git -newermt "-Nm"
    let git = path.join(".git");
    let Ok(meta) = fs::metadata(&git) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    let Ok(age) = SystemTime::now().duration_since(modified) else {
        return true; // future mtime → young
    };
    age < Duration::from_secs(idle_min.saturating_mul(60))
}

fn format_hhmm_epoch(epoch: i64) -> String {
    let mut c = hostrun(&["date", "-d", &format!("@{epoch}"), "+%H:%M"]);
    capture_stdout(&mut c).unwrap_or_default()
}

fn api_listen_pid() -> Option<String> {
    let out = hostrun(&["ss", "-ltnp"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if !line.contains(":8080") {
            continue;
        }
        if let Some(idx) = line.find("pid=") {
            let rest = &line[idx + 4..];
            let pid: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !pid.is_empty() {
                return Some(pid);
            }
        }
    }
    None
}

fn proc_start_epoch(pid: &str) -> i64 {
    let mut c = hostrun(&["stat", "-c", "%Y", &format!("/proc/{pid}")]);
    capture_stdout(&mut c)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Entry for `xtask platform preflight [--warn]`.
pub fn run(warn_only: bool) -> Result<u8> {
    let root = resolve_root()?;
    env::set_current_dir(&root).with_context(|| format!("cd {}", root.display()))?;

    let mut c = Counters { block: 0, warn: 0 };
    println!("═══ platform factory preflight ═══");

    // 1. Host bridge
    if Path::new("/run/.containerenv").is_file() {
        if status_ok(&mut hostrun(&["true"])) {
            ok("host bridge", "distrobox-host-exec live");
        } else {
            nope(
                &mut c,
                "host bridge",
                "in a container and distrobox-host-exec is dead — every cargo gate will fail",
            );
        }
    } else {
        ok("host bridge", "not containerised");
    }

    // 2. cargo (direct — T-889)
    {
        let mut cmd = Command::new("cargo");
        cmd.arg("--version");
        match capture_stdout(&mut cmd) {
            Some(v) => ok("cargo", &v),
            None => nope(&mut c, "cargo", "cargo unusable via the bridge"),
        }
    }

    // 3. Disk
    match free_gb(&root) {
        Some(gb) if gb >= 40 => ok("disk", &format!("{gb}G free")),
        Some(gb) if gb >= 20 => soft(
            &mut c,
            "disk",
            &format!("{gb}G free — tight; make clean-targets first"),
        ),
        Some(gb) => nope(&mut c, "disk", &format!("{gb}G free — below the 20G floor")),
        None => nope(&mut c, "disk", "df failed — below the 20G floor"),
    }
    let orphan_mb = orphan_cache_mb();
    if orphan_mb > 4096 {
        soft(
            &mut c,
            "reclaimable",
            &format!(
                "{}G of build caches in /var/tmp — cargo xtask platform wave reclaim",
                orphan_mb / 1024
            ),
        );
    } else {
        ok(
            "reclaimable",
            &format!("{}G of stale build caches", orphan_mb / 1024),
        );
    }

    // 4. CARGO_TARGET_DIR + no per-worktree target/
    match env::var("CARGO_TARGET_DIR") {
        Ok(v) if !v.is_empty() => ok("CARGO_TARGET_DIR", &v),
        _ => soft(
            &mut c,
            "CARGO_TARGET_DIR",
            "unset in this shell — `cargo xtask platform wave` exports it, but a dispatcher must too",
        ),
    }
    let stray = stray_worktree_targets(&root);
    if stray == 0 {
        ok("no per-worktree target/", "");
    } else {
        nope(
            &mut c,
            "per-worktree target/",
            &format!("{stray} worktree(s) built into their own target — will exhaust disk"),
        );
    }
    // T-300. The shared cache is fine for check/test/clippy and fatal for a launched binary; this
    // is the check that says which one the run target currently holds.
    {
        let run_dir = PathBuf::from(crate::wave::resolve_run_target_dir(&root));
        let head = git_out(&root, &["rev-parse", "HEAD"]).unwrap_or_default();
        let state = run_target_state(&run_dir, &head, &root);
        let (block, detail) = run_target_detail(&state, &run_dir, &head);
        if block {
            nope(&mut c, "run target", &detail);
        } else {
            ok("run target", &detail);
        }
    }

    // 5. RAM + swap
    match mem_available_mib() {
        Some(mb) if mb >= 1024 => ok("memory", &format!("{mb}MiB available")),
        Some(mb) => nope(
            &mut c,
            "memory",
            &format!("{mb}MiB — gate-env floor is 1024"),
        ),
        None => nope(&mut c, "memory", "0MiB — gate-env floor is 1024"),
    }
    if let Some(sw_used) = swap_used_pct() {
        if sw_used < 70 {
            ok("swap", &format!("{sw_used}% used"));
        } else {
            soft(
                &mut c,
                "swap",
                &format!("{sw_used}% used — OOM risk over a long run"),
            );
        }
    }

    // 6. Clean tree + synced remote
    match git_status_porcelain(&root) {
        Some(s) if s.is_empty() => ok("working tree", "clean"),
        Some(_) => nope(&mut c, "working tree", "dirty — commit or stash first"),
        None => nope(&mut c, "working tree", "dirty — commit or stash first"),
    }
    match git_out(&root, &["rev-parse", "--abbrev-ref", "HEAD"]) {
        Some(b) if b == "main" => ok("branch", "main"),
        _ => nope(&mut c, "branch", "not on main"),
    }
    let ahead =
        git_out(&root, &["rev-list", "--count", "origin/main..HEAD"]).unwrap_or_else(|| "?".into());
    if ahead == "0" {
        ok("remote", "in sync");
    } else {
        soft(&mut c, "remote", &format!("{ahead} commit(s) unpushed"));
    }

    // 7. Stale worktrees
    let wts = worktree_paths(&root);
    let wt = wts.len() as u64;
    if wt == 0 {
        ok("worktrees", "none stale");
    } else {
        soft(
            &mut c,
            "worktrees",
            &format!("{wt} left over — cargo xtask platform wave land will reuse or trip on them"),
        );
    }

    // 7b. Idle undispatched worktrees
    let idle_min: u64 = env::var("TBD_IDLE_WORKTREE_MIN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let mut idle: Vec<String> = Vec::new();
    for w in &wts {
        let t = w
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ahead_w =
            git_out(w, &["rev-list", "--count", "main..HEAD"]).unwrap_or_else(|| "0".into());
        let dirty_n = git_status_porcelain(w)
            .map(|s| s.lines().filter(|l| !l.is_empty()).count())
            .unwrap_or(0);
        let young = is_git_young(w, idle_min);
        if ahead_w == "0" && dirty_n == 0 && !young {
            idle.push(t);
        }
    }
    if idle.is_empty() {
        ok(
            "worktrees busy",
            &format!("every worktree is working or newer than {idle_min}m"),
        );
    } else {
        soft(
            &mut c,
            "idle worktrees",
            &format!(
                "nothing written in {idle_min}m+ — {} — created and never dispatched?",
                idle.join(" ")
            ),
        );
    }

    // 8. ticket check
    {
        let mut cmd = Command::new("cargo");
        cmd.args(["run", "-q", "-p", "xtask", "--", "ticket", "check"])
            .current_dir(&root);
        if status_ok(&mut cmd) {
            ok("ticket check", "registry valid");
        } else {
            nope(
                &mut c,
                "ticket check",
                "registry INVALID — every wave gate will fail",
            );
        }
    }

    // 9. Wave lock (T-912.2 — this check pointed at the TSV until the lock replaced it).
    // `wave check` recomputes from the tickets and structurally compares; a missing lock is a
    // DidNotRun refusal inside it, so an absent plan can never read as green here.
    {
        let mut cmd = Command::new("cargo");
        cmd.args(["run", "-q", "-p", "xtask", "--", "wave", "check"])
            .current_dir(&root);
        if status_ok(&mut cmd) {
            let (n, w) = wave_lock_open_count(&root).unwrap_or((0, 0));
            ok(
                "wave lock",
                &format!("{n} open tickets in {w} waves, matches the ticket files"),
            );
        } else {
            nope(
                &mut c,
                "wave lock",
                "cargo xtask wave check failed — stale or missing .ai/tickets/wave.lock",
            );
        }
    }

    // 10. Optional env — postgres + API freshness
    if tcp_up("127.0.0.1:5434") {
        ok("postgres :5434", "up");
    } else {
        soft(
            &mut c,
            "postgres :5434",
            "down — API integration tests will skip (cargo xtask db up on the HOST)",
        );
    }

    let api_code = curl_http_code("http://127.0.0.1:8080/healthz");
    if api_code == "200" {
        let api_pid = api_listen_pid().unwrap_or_default();
        let started = if api_pid.is_empty() {
            0
        } else {
            proc_start_epoch(&api_pid)
        };
        let newest = git_out(
            &root,
            &[
                "log",
                "-1",
                "--format=%ct",
                "--",
                "apps/website/api",
                "crates/map-engine-core",
            ],
        )
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
        if started > 0 && newest > started {
            soft(
                &mut c,
                "api :8080",
                &format!(
                    "healthy but STALE — running since {}, API code changed {}. Restart it or verifications lie.",
                    format_hhmm_epoch(started),
                    format_hhmm_epoch(newest)
                ),
            );
        } else {
            ok("api :8080", "healthz 200, binary current");
        }
    } else if !api_code.is_empty() && api_code != "000" {
        soft(
            &mut c,
            "api :8080",
            &format!("listening but /healthz returned {api_code} — wedged or mid-restart"),
        );
    } else {
        soft(
            &mut c,
            "api :8080",
            "down — editor smokes would report gate-red for an env reason",
        );
    }

    // 11b. trunk serve (informational since T-396)
    let ts = count_pgrep("trunk serve");
    if ts == 0 {
        ok("trunk serve", "not running");
    } else {
        ok(
            "trunk serve",
            &format!("{ts} running — fine since T-396; the gate builds into private dist + target"),
        );
    }

    // 11. Stray chrome
    let ch = count_pgrep("chrome-linux64/chrome");
    if ch == 0 {
        ok("chrome", "none stray");
    } else {
        soft(
            &mut c,
            "chrome",
            &format!("{ch} process(es) alive — leptos-gates will refuse"),
        );
    }

    println!();
    let mut out = io::stdout();
    if c.block > 0 {
        writeln!(
            out,
            "PREFLIGHT: {} BLOCK, {} warn — DO NOT START",
            c.block, c.warn
        )?;
        if warn_only {
            return Ok(0);
        }
        return Ok(1);
    }
    writeln!(out, "PREFLIGHT: PASS ({} warn)", c.warn)?;
    Ok(0)
}

#[cfg(test)]
mod run_target_tests {
    use super::*;
    use crate::wave::{RunStamp, write_run_stamp};
    use std::os::unix::fs::PermissionsExt;

    const HEAD: &str = "4b2cca4a5880ee8a0e8fbcbbedd476534db5b0ac";
    const OTHER: &str = "1f486e5721f906619259768b8ae7b7ebfd625fb9";

    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Tmp {
            let p = env::temp_dir().join(format!(
                "tbd-t300-pf-{tag}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).expect("mkdir scratch");
            Tmp(p)
        }
        /// A run target holding `bins` executables in `profile`, and the stamp when given.
        fn run_target(&self, profile: &str, bins: &[&str], stamp: Option<RunStamp>) -> PathBuf {
            let run = self.0.join("run-main");
            let d = run.join(profile);
            fs::create_dir_all(&d).expect("mkdir profile");
            for b in bins {
                let p = d.join(b);
                fs::write(&p, b"elf").expect("write bin");
                fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).expect("chmod");
            }
            if let Some(s) = stamp {
                write_run_stamp(&d, &s).expect("stamp");
            }
            run
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn stamp(sha: &str, checkout: &Path) -> RunStamp {
        RunStamp {
            sha: sha.to_string(),
            checkout: checkout.display().to_string(),
        }
    }

    #[test]
    fn a_run_target_that_was_never_built_is_green() {
        let t = Tmp::new("empty");
        let run = t.0.join("run-main");
        let st = run_target_state(&run, HEAD, &t.0);
        assert_eq!(st, RunTargetState::Empty);
        assert!(!run_target_detail(&st, &run, HEAD).0);
    }

    #[test]
    fn binaries_built_from_head_in_this_checkout_are_green() {
        let t = Tmp::new("fresh");
        let run = t.run_target("debug", &["api"], Some(stamp(HEAD, &t.0)));
        let st = run_target_state(&run, HEAD, &t.0);
        assert_eq!(
            st,
            RunTargetState::Fresh {
                profile: "debug".into(),
                bins: 1
            }
        );
        let (block, detail) = run_target_detail(&st, &run, HEAD);
        assert!(!block, "{detail}");
        assert!(detail.contains("4b2cca4"), "{detail}");
    }

    /// THE TICKET'S OWN PERTURBATION, as a unit: a stamp whose sha is not HEAD must go red and
    /// the message must name the binary, the sha it came from and HEAD.
    #[test]
    fn a_stamp_that_disagrees_with_head_blocks_and_names_the_binary() {
        let t = Tmp::new("stale");
        let run = t.run_target("debug", &["api", "world"], Some(stamp(OTHER, &t.0)));
        let st = run_target_state(&run, HEAD, &t.0);
        let (block, detail) = run_target_detail(&st, &run, HEAD);
        assert!(block, "a wrong sha read as green: {detail}");
        assert!(detail.contains("api,world"), "{detail}");
        assert!(detail.contains("1f486e5"), "{detail}");
        assert!(detail.contains("4b2cca4"), "{detail}");
        assert!(detail.contains("cargo clean --target-dir"), "{detail}");
    }

    /// THE WAVE-1 INCIDENT, as a unit: right sha, wrong checkout. This is the case a sha-only
    /// comparison passes — a worktree sitting on the same commit as main still holds UNCOMMITTED
    /// slice code, which is precisely what `make api` served on :8080.
    #[test]
    fn a_stamp_from_a_worktree_at_the_same_sha_still_blocks_and_names_the_checkout() {
        let t = Tmp::new("foreign");
        let wt = t.0.join(".ai/artifacts/worktrees/T-300");
        let run = t.run_target("debug", &["api"], Some(stamp(HEAD, &wt)));
        let st = run_target_state(&run, HEAD, &t.0);
        let (block, detail) = run_target_detail(&st, &run, HEAD);
        assert!(block, "a foreign checkout read as green: {detail}");
        assert!(detail.contains("worktrees/T-300"), "{detail}");
    }

    /// Fail closed. Binaries with no stamp are the state every run target was in before this
    /// ticket, and reporting that as green would make the whole check decorative.
    #[test]
    fn binaries_with_no_stamp_block_rather_than_pass() {
        let t = Tmp::new("unstamped");
        let run = t.run_target("release", &["api"], None);
        let st = run_target_state(&run, HEAD, &t.0);
        let (block, detail) = run_target_detail(&st, &run, HEAD);
        assert!(block, "unstamped binaries read as green: {detail}");
        assert!(detail.contains("tbd-built-from"), "{detail}");
        assert!(detail.contains("release/api"), "{detail}");
    }

    /// A preflight that cannot resolve HEAD certifies nothing.
    #[test]
    fn an_unresolvable_head_blocks_rather_than_certifies() {
        let t = Tmp::new("nohead");
        let run = t.run_target("debug", &["api"], Some(stamp(HEAD, &t.0)));
        let st = run_target_state(&run, "", &t.0);
        let (block, detail) = run_target_detail(&st, &run, "");
        assert!(block, "empty HEAD read as green: {detail}");
        assert!(detail.contains("(unresolved)"), "{detail}");
    }

    /// Both profile directories are read: a release run binary must not hide behind an empty
    /// debug one.
    #[test]
    fn the_release_profile_is_checked_too() {
        let t = Tmp::new("release");
        let run = t.run_target("release", &["api"], Some(stamp(OTHER, &t.0)));
        assert!(run_target_detail(&run_target_state(&run, HEAD, &t.0), &run, HEAD).0);
    }

    /// The reader/writer contract, asserted from the READER's side: what
    /// [`crate::wave::write_run_stamp`] emits is exactly what this module accepts, and an absent
    /// stamp reads as unknown rather than as agreement.
    #[test]
    fn the_stamp_round_trips_and_an_absent_one_reads_as_unknown() {
        let t = Tmp::new("roundtrip");
        let d = t.0.join("debug");
        let s = stamp(HEAD, &t.0);
        assert_eq!(s.render(), format!("{HEAD} {}\n", t.0.display()));
        assert_eq!(
            crate::wave::read_run_stamp(&d),
            None,
            "absent read as agreement"
        );
        write_run_stamp(&d, &s).expect("write");
        assert_eq!(crate::wave::run_stamp_path(&d), d.join("tbd-built-from"));
        assert_eq!(crate::wave::read_run_stamp(&d), Some(s));
    }

    /// The check NAMES the binary, so this must find binaries and nothing else.
    #[test]
    fn run_binaries_lists_executables_and_skips_the_stamp_and_depfiles() {
        let t = Tmp::new("bins");
        let d = t.0.join("debug");
        fs::create_dir_all(d.join("deps")).expect("mkdir");
        for (name, mode) in [("api", 0o755), ("world", 0o755), ("notes", 0o644)] {
            let p = d.join(name);
            fs::write(&p, b"x").expect("write");
            fs::set_permissions(&p, fs::Permissions::from_mode(mode)).expect("chmod");
        }
        fs::write(d.join("api.d"), b"dep").expect("write");
        write_run_stamp(&d, &stamp(HEAD, &t.0)).expect("stamp");
        assert_eq!(
            run_binaries(&d),
            vec!["api".to_string(), "world".to_string()]
        );
    }
}
