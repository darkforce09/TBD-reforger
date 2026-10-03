//! Live Workbench path for spawn-determinism, split from [`super::spawn_determinism`] for SIZE-1.
//! Only reached after preflight succeeds.
//!
//! A run that cannot go on (Workbench never comes back, no console log, `wb_play` refused) prints
//! its `FATAL:` line to stderr and returns [`FATAL_EXIT`] as the command's exit status.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use regex::Regex;

use crate::Result;

use super::{
    assess_run, det_timeout, extract, keep_snapshots, normalize, port_open, tempfile_dir, wb_port,
};

/// The exit status of a live run that stopped on a `FATAL:` line.
const FATAL_EXIT: u8 = 2;

/// A live run stopped: its `FATAL:` line is printed, and the run exits [`FATAL_EXIT`].
struct FatalStop;

fn mcp(repo_root: &Path, tool: &str, args_json: &str) -> process_runner::Merged {
    // `cargo xtask mcp call`.
    // Callers inspect `.code` / `.text` (bash redirected most calls).
    match process_runner::Run::new("cargo")
        .args([
            "run", "-q", "-p", "xtask", "--", "mcp", "call", tool, args_json,
        ])
        .cwd(repo_root)
        .merged_output()
    {
        Ok(m) => m,
        Err(_) => process_runner::Merged {
            code: 127,
            text: String::new(),
            duration: Duration::ZERO,
        },
    }
}

fn latest_log() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let home = PathBuf::from(home);
    let dirs = [
        home.join(
            ".local/share/Steam/steamapps/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/logs",
        ),
        home.join("Documents/Games/ArmaReforgerWorkbench/logs"),
    ];
    for d in &dirs {
        if !d.is_dir() {
            continue;
        }
        // bash: `ls -td "$d"/logs_* 2>/dev/null | head -1` — mtime order.
        let mut entries: Vec<_> = fs::read_dir(d)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("logs_"))
            })
            .collect();
        entries.sort_by_key(|p| std::cmp::Reverse(fs::metadata(p).and_then(|m| m.modified()).ok()));
        if let Some(picked) = entries.into_iter().next() {
            return Some(picked.join("console.log"));
        }
    }
    None
}

fn restart_wb_once(repo_root: &Path, world: &str) -> std::result::Result<bool, FatalStop> {
    // Bracket trick preserved in spirit: kill WorkbenchSteamDiag without matching ourselves.
    let _ = process_runner::Run::new("pkill")
        .arg("-f")
        .arg("WorkbenchSteamD[i]ag")
        .merged_output();
    thread::sleep(Duration::from_secs(5));
    if !port_open() {
        // bash: `steam -applaunch 1874910 2>/dev/null || true` — intentional launch fail-open.
        let _ = process_runner::Run::new("steam")
            .arg("-applaunch")
            .arg("1874910")
            .merged_output();
    }
    let mut waited = 0u64;
    while !port_open() {
        thread::sleep(Duration::from_secs(5));
        waited += 5;
        if waited >= 300 {
            eprintln!("FATAL: Workbench did not come back on :{}", wb_port());
            return Err(FatalStop);
        }
    }
    thread::sleep(Duration::from_secs(15));
    if mcp(repo_root, "wb_connect", "{}").code != 0 {
        return Ok(false);
    }
    let open = mcp(
        repo_root,
        "wb_open_resource",
        &format!("{{\"path\":\"{world}\"}}"),
    );
    let ok = Regex::new(r"Resource Opened|Opened:")
        .expect("open probe")
        .is_match(&open.text);
    if !ok {
        return Ok(false);
    }
    thread::sleep(Duration::from_secs(15));
    Ok(true)
}

fn restart_wb(repo_root: &Path, world: &str) -> std::result::Result<(), FatalStop> {
    for try_n in 1..=3 {
        if restart_wb_once(repo_root, world)? {
            return Ok(());
        }
        println!("WARN: Workbench came up game-dead (try {try_n}) — cycling again");
    }
    eprintln!("FATAL: Workbench game-dead after 3 restart cycles");
    Err(FatalStop)
}

pub(crate) fn run_live(repo_root: &Path, runs: u32, world: &str) -> Result<u8> {
    let out_dir = tempfile_dir("tbd-spawn-det");
    let timeout = det_timeout();
    let mut fail: u8 = 0;
    let mut digests: Vec<String> = Vec::with_capacity(runs as usize);

    for i in 1..=runs {
        println!("── run {i}/{runs} ──");
        if restart_wb(repo_root, world).is_err() {
            return Ok(FATAL_EXIT);
        }
        let Some(log) = latest_log() else {
            eprintln!("FATAL: no console.log found");
            return Ok(FATAL_EXIT);
        };
        let mark = line_count(&log);

        if mcp(repo_root, "wb_play", "{}").code != 0 {
            eprintln!("FATAL: wb_play failed");
            return Ok(FATAL_EXIT);
        }

        let mut waited = 0u64;
        let mut done_flag = false;
        while waited < timeout {
            thread::sleep(Duration::from_secs(5));
            waited += 5;
            let new = tail_from(&log, mark);
            if new.contains("[TBD][Audit]") {
                done_flag = true;
                break;
            }
        }
        thread::sleep(Duration::from_secs(8));
        let new = tail_from(&log, mark);
        let _ = mcp(repo_root, "wb_stop", "{}");

        if !done_flag {
            println!("FAIL run {i}: sentinel not seen within {timeout}s");
            fail = 1;
        }

        let raw = out_dir.join(format!("run{i}.raw.log"));
        let norm = out_dir.join(format!("run{i}.norm.log"));
        fs::write(&raw, &new)?;
        // bash: `echo "$NEW" | extract /dev/stdin | normalize > "$NORM" || true`
        // FAIL-OPEN PIN: extract/normalize pipeline errors discarded; reproduced (infallible here).
        let extracted = extract(&new);
        let normalized = normalize(&extracted);
        fs::write(&norm, &normalized)?;

        if assess_run(&raw, &format!("{i}")) != 0 {
            fail = 1;
        }

        // The digest of the normalised snapshot, byte for byte the file just written.
        let digest = content_digest::sha256_hex(normalized.as_bytes());
        let lines = normalized.lines().count();
        println!(
            "run {i} digest {} ({lines} lines)",
            &digest[..12.min(digest.len())]
        );
        digests.push(digest);
    }

    for i in 1..runs as usize {
        if digests[i] != digests[0] {
            println!(
                "FAIL: run {} digest differs from run 1 — first divergence:",
                i + 1
            );
            let a = fs::read_to_string(out_dir.join("run1.norm.log")).unwrap_or_default();
            let b = fs::read_to_string(out_dir.join(format!("run{}.norm.log", i + 1)))
                .unwrap_or_default();
            // Approximate `diff | head -15` — enough for operator evidence.
            for (n, line) in difflines(&a, &b).into_iter().take(15).enumerate() {
                let _ = n;
                println!("{line}");
            }
            fail = 1;
        }
    }

    if fail == 0 {
        let d = &digests[0];
        println!(
            "DETERMINISM PASS: {runs}/{runs} identical (digest {})",
            &d[..12.min(d.len())]
        );
        if !keep_snapshots() {
            let _ = fs::remove_dir_all(&out_dir);
        }
    } else {
        println!("DETERMINISM FAIL — snapshots kept at {}", out_dir.display());
    }
    Ok(fail)
}

fn line_count(path: &Path) -> usize {
    fs::read_to_string(path)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

fn tail_from(path: &Path, mark: usize) -> String {
    match fs::read_to_string(path) {
        Ok(s) => s.lines().skip(mark).collect::<Vec<_>>().join("\n"),
        Err(_) => String::new(),
    }
}

fn difflines(a: &str, b: &str) -> Vec<String> {
    // Minimal unified-ish dump when digests diverge (bash uses `diff | head -15`).
    let mut out = Vec::new();
    let al: Vec<_> = a.lines().collect();
    let bl: Vec<_> = b.lines().collect();
    let n = al.len().max(bl.len());
    for i in 0..n {
        let left = al.get(i).copied().unwrap_or("");
        let right = bl.get(i).copied().unwrap_or("");
        if left != right {
            if !left.is_empty() {
                out.push(format!("< {left}"));
            }
            if !right.is_empty() {
                out.push(format!("> {right}"));
            }
        }
    }
    out
}
