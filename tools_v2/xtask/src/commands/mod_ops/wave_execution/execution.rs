use super::*;

/// Entry for `xtask mod wave [args…]`.
pub fn run(args: &[String]) -> Result<u8> {
    let root = find_repo_root()?;
    Ok(run_with_root(&root, args))
}

/// Testable entry that does not walk for the repo root.
pub fn run_with_root(root: &Path, args: &[String]) -> u8 {
    let cmd = args.first().map(String::as_str).unwrap_or("status");
    match cmd {
        "status" => cmd_status(root),
        "push" => cmd_push(root),
        "gate" => cmd_gate(root),
        "land" => cmd_land(root),
        "prep" => {
            let w = args.get(1).map(String::as_str).unwrap_or("");
            cmd_prep(root, w)
        }
        _ => {
            print!("{UNKNOWN_HELP}");
            let _ = io::stdout().flush();
            2
        }
    }
}

/// Is this id one of the mod program's? The driver is the T-181 driver — it says so on the tin —
/// and `shipped_slices` reads the T-181 slice plan exclusively, so any other id in the shared
/// lock is another program's row.
pub(super) fn mod_slice_id(id: &str) -> bool {
    id.starts_with("T-181.")
}

/// The lock's `(wave, slice)` pairs for this program. A missing/unreadable lock is `None` —
/// callers refuse loudly instead of shrugging into "ALL PLANNED WAVES SHIPPED".
pub(super) fn lock_mod_rows(root: &Path) -> Option<Vec<(u32, String)>> {
    let lock = match ticket_engine::wave_lock::load(root) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("mod wave: {e:#}");
            return None;
        }
    };
    Some(
        lock.waves
            .iter()
            .flat_map(|w| {
                w.tickets
                    .iter()
                    .filter(|t| mod_slice_id(t))
                    .map(move |t| (w.n, t.clone()))
            })
            .collect(),
    )
}

pub(super) fn wave_slices(root: &Path, w: &str) -> Vec<String> {
    let Some(rows) = lock_mod_rows(root) else {
        return Vec::new();
    };
    let Ok(n) = w.parse::<u32>() else {
        return Vec::new();
    };
    rows.into_iter()
        .filter(|(wn, _)| *wn == n)
        .map(|(_, s)| s)
        .collect()
}

pub(super) fn slice_title(root: &Path, s: &str) -> String {
    ticket_engine::registry::ticket_titles::read_ticket_title(root, s)
}

/// Open lock waves (n > 0) that hold at least one mod slice, ascending.
pub(super) fn unique_sorted_waves(root: &Path) -> Vec<String> {
    let Some(rows) = lock_mod_rows(root) else {
        return Vec::new();
    };
    let mut waves: Vec<u32> = rows
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, _)| n)
        .collect();
    waves.sort_unstable();
    waves.dedup();
    waves.into_iter().map(|n| n.to_string()).collect()
}

/// Shipped slice ids for T-181 (python3 one-liner → serde). On any error → empty (2>/dev/null).
pub(super) fn shipped_slices(root: &Path) -> Vec<String> {
    let v: Value = match ticket_engine::registry::load_registry(root) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let tickets = match v.get("tickets").and_then(|t| t.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };
    let t181 = match tickets
        .iter()
        .find(|t| t.get("id").and_then(|i| i.as_str()) == Some("T-181"))
    {
        Some(t) => t,
        None => return Vec::new(),
    };
    let plan = match t181.get("slice_plan").and_then(|p| p.as_object()) {
        Some(p) => p,
        None => return Vec::new(),
    };
    plan.iter()
        .filter(|(_, v)| v.get("status").and_then(|s| s.as_str()) == Some("shipped"))
        .map(|(k, _)| k.clone())
        .collect()
}

/// The first open lock wave whose mod slices are not all shipped. `None` = the lock itself is
/// missing or unreadable (already reported by [`lock_mod_rows`]) — a refusal, not "done".
pub(super) fn current_wave(root: &Path) -> Option<String> {
    lock_mod_rows(root)?;
    let shipped = shipped_slices(root);
    for w in unique_sorted_waves(root) {
        let mut done_all = true;
        for s in wave_slices(root, &w) {
            if !shipped.iter().any(|x| x == &s) {
                done_all = false;
                break;
            }
        }
        if !done_all {
            return Some(w);
        }
    }
    Some("done".to_string())
}

/// `sed -E 's/^(T-[0-9]+\.[0-9]+).*/\1/'` — sub-slices share the parent's worktree.
pub(super) fn parent_slice(s: &str) -> String {
    let re = Regex::new(r"^(T-[0-9]+\.[0-9]+)").expect("parent_slice regex");
    match re.find(s) {
        Some(m) => m.as_str().to_string(),
        None => s.to_string(),
    }
}

pub(super) fn tree_state(root: &Path, slice: &str) -> TreeState {
    let d = root.join(BASE).join(parent_slice(slice));
    if !d.is_dir() {
        return TreeState::Absent;
    }
    // bash: stdout only; stderr discarded; non-git dir → empty → committed
    let out = Command::new("git")
        .args(["-C", d.to_str().unwrap_or(""), "status", "--porcelain"])
        .output();
    match out {
        Ok(o) if !String::from_utf8_lossy(&o.stdout).trim().is_empty() => TreeState::Dirty,
        _ => TreeState::Committed,
    }
}

pub(super) fn has_work(root: &Path, slice: &str) -> bool {
    let b = format!("slice/{}", parent_slice(slice));
    let ok = Command::new("git")
        .current_dir(root)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{b}"),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        return false;
    }
    let out = Command::new("git")
        .current_dir(root)
        .args(["log", "--oneline", &format!("main..{b}")])
        .output();
    match out {
        Ok(o) => !String::from_utf8_lossy(&o.stdout).trim().is_empty(),
        Err(_) => false,
    }
}

pub(super) fn cmd_status(root: &Path) -> u8 {
    let Some(w) = current_wave(root) else {
        return 2;
    };
    println!("═══ mod wave status ═══");
    if w == "done" {
        println!(
            "ALL PLANNED WAVES SHIPPED. Next: queue mod tickets and `cargo xtask wave repack`, or close the program."
        );
        return 0;
    }
    println!("current wave: {w}");
    let mut ready = 0usize;
    let mut total = 0usize;
    for s in wave_slices(root, &w) {
        let st = tree_state(root, &s);
        total += 1;
        let mark = if st == TreeState::Committed && has_work(root, &s) {
            ready += 1;
            "READY".to_string()
        } else if st == TreeState::Committed {
            "empty (no commits yet)".to_string()
        } else if st == TreeState::Dirty {
            "DIRTY — agent must commit in its worktree".to_string()
        } else {
            format!("no worktree — run: cargo run -q -p xtask -- mod wave prep {w}")
        };
        println!("  {:<12} {:<9} {}", s, st.as_str(), mark);
        println!("               {}", slice_title(root, &s));
    }
    println!();
    println!("ready to merge: {ready}/{total}");
    if ready == total && total > 0 {
        println!("ACTION: cargo run -q -p xtask -- mod wave land");
    } else {
        println!("ACTION: wait for slice agents, then re-run status");
    }
    0
}

pub(super) fn cmd_prep(root: &Path, wave_arg: &str) -> u8 {
    let w = if wave_arg.is_empty() {
        match current_wave(root) {
            Some(w) => w,
            None => return 2,
        }
    } else {
        wave_arg.to_string()
    };
    if w == "done" {
        println!("nothing to prep");
        return 0;
    }
    // T-853: was `bash scripts/mod/slice-worktree.sh new <slice>`. Called IN-PROCESS now rather
    // than re-spawning cargo — the script is gone, and a nested `cargo run` here would pay a second
    // resolution and could pick a different target dir than the one this process was launched with.
    for s in wave_slices(root, &w) {
        // bash ran without -e here: a failed `new` does not stop the other slices from prepping.
        let _ = crate::commands::platform::slice_worktree::run_at(
            root,
            &["new".to_string(), s.clone()],
        );
    }
    0
}

pub(super) fn cmd_gate(root: &Path) -> u8 {
    println!("═══ wave gate ═══");
    let mut fail = 0u8;

    let mut run = |label: &str, program: &str, args: &[&str]| {
        print!("  {label:<26} ");
        let _ = io::stdout().flush();
        match Run::new(program).args(args).cwd(root).merged_output() {
            Ok(m) if m.code == 0 => {
                println!("PASS");
            }
            Ok(m) => {
                println!("FAIL");
                let lines: Vec<&str> = m.text.lines().collect();
                let start = lines.len().saturating_sub(12);
                for line in &lines[start..] {
                    println!("      {line}");
                }
                fail = 1;
            }
            Err(nr) => {
                // Tool absent / signalled — still a FAIL arm (bash would fail similarly).
                println!("FAIL");
                println!("      {nr:?}");
                fail = 1;
            }
        }
    };

    run(
        "compile",
        "cargo",
        &["run", "-q", "-p", "xtask", "--", "mod", "compile"],
    );
    // T-897: was `distrobox-host-exec cargo xtask mod compile-selftest`. That Makefile recipe carried the
    // rc classification (only exit 1 — a real rejection of broken source — is a pass); it now
    // lives in `crate::commands::mod_ops::compile::run_selftest`. The host bridge is dropped for the same reason
    // the `compile` arm above does not need it: the gate crosses it itself.
    run(
        "compile-selftest",
        "cargo",
        &["run", "-q", "-p", "xtask", "--", "mod", "compile-selftest"],
    );
    run(
        "world boot",
        "cargo",
        &["run", "-q", "-p", "xtask", "--", "mod", "world-boot"],
    );
    run(
        "world-boot selftest",
        "cargo",
        &[
            "run",
            "-q",
            "-p",
            "xtask",
            "--",
            "mod",
            "world-boot",
            "--selftest",
        ],
    );
    run(
        "world boot +mission",
        "cargo",
        &[
            "run",
            "-q",
            "-p",
            "xtask",
            "--",
            "mod",
            "world-boot",
            "--mission=bridgehead-at-levie",
        ],
    );
    run(
        "ui layouts",
        "cargo",
        &["run", "-q", "-p", "xtask", "--", "verify", "ui-layouts"],
    );
    run(
        "schema validate",
        "distrobox-host-exec",
        &["make", "schema-validate"],
    );
    run(
        "capability",
        "distrobox-host-exec",
        &["make", "verify-capability"],
    );
    run(
        "oracle citations",
        "distrobox-host-exec",
        &["make", "verify-oracle"],
    );
    run(
        "no-crf-leak",
        "distrobox-host-exec",
        &["make", "verify-no-crf-leak"],
    );
    run(
        "ticket registry",
        "distrobox-host-exec",
        &["cargo", "run", "-q", "-p", "xtask", "--", "ticket", "check"],
    );
    run(
        "enf unit tests",
        "distrobox-host-exec",
        &[
            "cargo",
            "test",
            "-q",
            "-p",
            "developer-tools",
            "--lib",
            "enf::",
        ],
    );

    println!();
    if fail != 0 {
        println!("GATE: FAIL");
        return 1;
    }
    println!("GATE: PASS");
    0
}
