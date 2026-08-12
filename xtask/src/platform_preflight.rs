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
        if p.join(".ai/tickets/registry.json").is_file() {
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

fn wave_plan_ticket_count(root: &Path) -> Option<usize> {
    let text = fs::read_to_string(root.join("docs/platform/wave_plan.tsv")).ok()?;
    // bash grep -vc '^#' (includes blanks).
    Some(text.lines().filter(|l| !l.starts_with('#')).count())
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
                "{}G of build caches in /var/tmp — bash scripts/platform/wave.sh reclaim",
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
            "unset in this shell — wave.sh exports it, but a dispatcher must too",
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
            &format!("{wt} left over — wave.sh land will reuse or trip on them"),
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

    // 9. Wave plan / slice-collisions
    {
        let mut cmd = Command::new("cargo");
        cmd.args(["run", "-q", "-p", "xtask", "--", "slice-collisions"])
            .current_dir(&root);
        if status_ok(&mut cmd) {
            let n = wave_plan_ticket_count(&root).unwrap_or(0);
            ok("wave plan", &format!("{n} tickets, dispatch set computes"));
        } else {
            nope(&mut c, "wave plan", "cargo xtask slice-collisions failed");
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
