use super::*;

pub(super) fn ok(label: &str, detail: &str) {
    println!("  \x1b[32m✓\x1b[0m {label:<34} {detail}");
}

pub(super) fn nope(c: &mut Counters, label: &str, detail: &str) {
    println!("  \x1b[31m✗ BLOCK\x1b[0m {label:<28} {detail}");
    c.block += 1;
}

pub(super) fn soft(c: &mut Counters, label: &str, detail: &str) {
    println!("  \x1b[33m! WARN \x1b[0m {label:<28} {detail}");
    c.warn += 1;
}

pub(super) fn in_container() -> bool {
    Path::new("/run/.containerenv").is_file()
        || Path::new("/.dockerenv").is_file()
        || env::var_os("container").is_some()
}

pub(super) fn has_distrobox_host_exec() -> bool {
    Command::new("sh")
        .args(["-c", "command -v distrobox-host-exec >/dev/null 2>&1"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Wrap only when containerised AND distrobox-host-exec is on PATH.
pub(super) fn use_host_bridge() -> bool {
    has_distrobox_host_exec() && in_container()
}

pub(super) fn hostrun(args: &[&str]) -> Command {
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

pub(super) fn capture_stdout(cmd: &mut Command) -> Option<String> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

pub(super) fn status_ok(cmd: &mut Command) -> bool {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

pub(super) fn resolve_root() -> Result<PathBuf> {
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

pub(super) fn free_gb(root: &Path) -> Option<u64> {
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

pub(super) fn orphan_cache_mb() -> u64 {
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

pub(super) fn mem_available_mib() -> Option<u64> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb / 1024);
        }
    }
    None
}

pub(super) fn swap_used_pct() -> Option<u64> {
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

pub(super) fn git_out(root: &Path, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(root);
    capture_stdout(&mut cmd)
}

pub(super) fn git_status_porcelain(root: &Path) -> Option<String> {
    git_out(root, &["status", "--porcelain"])
}

pub(super) fn count_pgrep(pattern: &str) -> u64 {
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

pub(super) fn tcp_up(addr: &str) -> bool {
    let Ok(mut addrs) = addr.to_socket_addrs() else {
        return false;
    };
    let Some(sa) = addrs.next() else {
        return false;
    };
    TcpStream::connect_timeout(&sa, Duration::from_secs(1)).is_ok()
}

pub(super) fn curl_http_code(url: &str) -> String {
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
pub(super) fn wave_lock_open_count(root: &Path) -> Option<(usize, usize)> {
    let lock = ticket_engine::wave_lock::load(root).ok()?;
    let open: usize = lock
        .waves
        .iter()
        .filter(|w| w.n > 0)
        .map(|w| w.tickets.len())
        .sum();
    let waves = lock.waves.iter().filter(|w| w.n > 0).count();
    Some((open, waves))
}

pub(super) fn stray_worktree_targets(root: &Path) -> u64 {
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

/// The executables in a profile directory, sorted — what this check NAMES when it blocks.
///
/// Regular files with an execute bit and no extension: cargo's uplifted-binary shape, which
/// excludes `.d` depfiles, the stamp, `.rlib`/`.rmeta` and the `deps/ build/ incremental/`
/// subdirectories without enumerating them. A sibling of [`stray_worktree_targets`] above rather
/// than of the writer in [`crate::commands::platform::wave_execution`]: it is a probe of a directory, and the reader is the
/// only caller.
pub(super) fn run_binaries(bin_dir: &Path) -> Vec<String> {
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
        if name.contains('.') || name == crate::commands::platform::wave_execution::RUN_STAMP_FILE {
            continue;
        }
        out.push(name);
    }
    out.sort();
    out
}

/// Read every profile directory in the run target and report the FIRST one that is not green.
///
/// First-bad rather than a summary: preflight's contract is one line per check, and the operator
/// needs the path to delete, not a census. `head` empty (git could not answer) is itself a
/// disagreement — a preflight that cannot resolve HEAD cannot certify anything.
pub(super) fn run_target_state(run_dir: &Path, head: &str, this_checkout: &Path) -> RunTargetState {
    let mut fresh: Option<RunTargetState> = None;
    for profile in RUN_PROFILE_DIRS {
        let bin_dir = run_dir.join(profile);
        let bins = run_binaries(&bin_dir);
        if bins.is_empty() {
            continue;
        }
        match crate::commands::platform::wave_execution::read_run_stamp(&bin_dir) {
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
pub(super) fn run_target_detail(
    state: &RunTargetState,
    run_dir: &Path,
    head: &str,
) -> (bool, String) {
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
                crate::commands::platform::wave_execution::RUN_STAMP_FILE,
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
/// sentence, which is how `crate::commands::platform::wave_execution::short` renders a failure and how a message loses its subject.
pub(super) fn short(sha: &str) -> String {
    if sha.len() < 7 {
        return "(unresolved)".to_string();
    }
    sha[..7].to_string()
}

pub(super) fn worktree_paths(root: &Path) -> Vec<PathBuf> {
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

pub(super) fn is_git_young(path: &Path, idle_min: u64) -> bool {
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

pub(super) fn format_hhmm_epoch(epoch: i64) -> String {
    let mut c = hostrun(&["date", "-d", &format!("@{epoch}"), "+%H:%M"]);
    capture_stdout(&mut c).unwrap_or_default()
}

pub(super) fn api_listen_pid() -> Option<String> {
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

pub(super) fn proc_start_epoch(pid: &str) -> i64 {
    let mut c = hostrun(&["stat", "-c", "%Y", &format!("/proc/{pid}")]);
    capture_stdout(&mut c)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}
