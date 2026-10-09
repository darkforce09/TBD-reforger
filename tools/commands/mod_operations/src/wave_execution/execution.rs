//! The mod wave driver's dispatch, readers, `status`, `prep` and `gate`.
//!
//! **Role:** reads the wave lock and the registry, reports the current wave, prepares its worktrees
//! and runs the wave gate.
//! **Position:** under [`crate::wave_execution`]; uses `platform_execution::slice_worktree` and the
//! ticket crates.
//! **Signals & state:** none; each run reads the live lock, git and worktree state.
//! **Invariants:** a gate step that could not run is a failure, never a pass.

use super::*;

/// Entry for `xtask mod wave [args…]`.
pub(crate) fn run(args: &[String]) -> Result<u8> {
    let root = find_repository_root()?;
    Ok(run_with_root(&root, args))
}

/// Testable entry that does not walk for the repo root.
pub(super) fn run_with_root(root: &Path, args: &[String]) -> u8 {
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
            print!("{}", unknown_help());
            let _ = io::stdout().flush();
            2
        }
    }
}

/// The programme whose dotted children this driver owns, from the corpus pins. A missing or
/// malformed pin file is a refusal: filtering the shared lock against an empty programme id
/// would silently claim every other programme's rows.
pub(super) fn mod_programme(root: &Path) -> Option<String> {
    match ticket_registry::corpus_pins::load(root) {
        Ok(pins) => Some(pins.game_mod_programme_ticket.into()),
        Err(error) => {
            eprintln!("mod wave: {}", ticket_model::error_chain_text(&error));
            None
        }
    }
}

/// Is this id one of the mod programme's? Its dotted children are its slices, and
/// `shipped_slices` reads that programme's slice plan exclusively, so any other id in the
/// shared lock is another programme's row.
pub(super) fn mod_slice_id(programme: &str, id: &str) -> bool {
    id.starts_with(&format!("{programme}."))
}

/// The lock's `(wave, slice)` pairs for this programme. A missing/unreadable lock or pin file
/// is `None` — callers refuse loudly instead of shrugging into "ALL PLANNED WAVES SHIPPED".
pub(super) fn lock_mod_rows(root: &Path) -> Option<Vec<(u32, String)>> {
    let programme = mod_programme(root)?;
    let lock = match ticket_wave_lock::load(root) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("mod wave: {}", ticket_model::error_chain_text(&e));
            return None;
        }
    };
    Some(
        lock.waves
            .iter()
            .flat_map(|w| {
                w.tickets
                    .iter()
                    .filter(|t| mod_slice_id(&programme, t.as_str()))
                    .map(move |t| (w.n, t.to_string()))
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
    ticket_registry::registry::ticket_titles::read_ticket_title(root, s)
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

/// Shipped slice ids for the mod programme. On any error → empty.
pub(super) fn shipped_slices(root: &Path, programme: &str) -> Vec<String> {
    let v: Value = match ticket_registry::registry::load_registry(root) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let tickets = match v.get("tickets").and_then(|t| t.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };
    let t181 = match tickets
        .iter()
        .find(|t| t.get("id").and_then(|i| i.as_str()) == Some(programme))
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
    let shipped = shipped_slices(root, &mod_programme(root)?);
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
    let d = root.join(WORKTREES_DIR).join(parent_slice(slice));
    if !d.is_dir() {
        return TreeState::Absent;
    }
    // bash: stdout only; stderr discarded; non-git dir → empty → committed
    let out = Run::new("git")
        .args(["-C", d.to_str().unwrap_or(""), "status", "--porcelain"])
        .output();
    match out {
        Ok(o) if !o.stdout.trim().is_empty() => TreeState::Dirty,
        _ => TreeState::Committed,
    }
}

pub(super) fn has_work(root: &Path, slice: &str) -> bool {
    let b = format!("slice/{}", parent_slice(slice));
    let ok = Run::new("git")
        .cwd(root)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{b}"),
        ])
        .status()
        .is_ok_and(|code| code == 0);
    if !ok {
        return false;
    }
    let out = Run::new("git")
        .cwd(root)
        .args(["log", "--oneline", &format!("main..{b}")])
        .output();
    match out {
        Ok(o) => !o.stdout.trim().is_empty(),
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
    // The slice-worktree `new` verb, called IN-PROCESS rather
    // than re-spawning cargo — the script is gone, and a nested `cargo run` here would pay a second
    // resolution and could pick a different target dir than the one this process was launched with.
    for s in wave_slices(root, &w) {
        // bash ran without -e here: a failed `new` does not stop the other slices from prepping.
        let _ = platform_execution::slice_worktree::run_at(root, &["new".to_string(), s.clone()]);
    }
    0
}

/// One wave gate step: the label printed beside its verdict and the command it runs.
pub(super) struct GateStep {
    pub(super) label: &'static str,
    pub(super) program: &'static str,
    pub(super) args: &'static [&'static str],
}

/// The wave gate's steps, in run order. `capability` and `oracle citations` are the `enf` checks
/// of the upstream-framework capability verdicts and of the `@idx` citations in the docs; `schema
/// validate` is the contract sub-gate set of `cargo xtask ci schema-validate`.
pub(super) const GATE_STEPS: &[GateStep] = &[
    GateStep {
        label: "compile",
        program: "cargo",
        args: &["run", "-q", "-p", "xtask", "--", "mod", "compile"],
    },
    GateStep {
        label: "world boot",
        program: "cargo",
        args: &["run", "-q", "-p", "xtask", "--", "mod", "world-boot"],
    },
    GateStep {
        label: "world boot +mission",
        program: "cargo",
        args: &[
            "run",
            "-q",
            "-p",
            "xtask",
            "--",
            "mod",
            "world-boot",
            "--mission=bridgehead-at-levie",
        ],
    },
    GateStep {
        label: "schema validate",
        program: "cargo",
        args: &["run", "-q", "-p", "xtask", "--", "ci", "schema-validate"],
    },
    GateStep {
        label: "capability",
        program: "cargo",
        args: &[
            "run",
            "-q",
            "-p",
            "developer_tools",
            "--bin",
            "enf",
            "--",
            "capability",
        ],
    },
    GateStep {
        label: "oracle citations",
        program: "cargo",
        args: &[
            "run",
            "-q",
            "-p",
            "developer_tools",
            "--bin",
            "enf",
            "--",
            "citations",
        ],
    },
    GateStep {
        label: "ticket registry",
        program: "distrobox-host-exec",
        args: &["cargo", "run", "-q", "-p", "xtask", "--", "ticket", "check"],
    },
    GateStep {
        label: "enf unit tests",
        program: "distrobox-host-exec",
        args: &[
            "cargo",
            "test",
            "-q",
            "-p",
            "enfusion_script_index",
            "--lib",
        ],
    },
];

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

    for step in GATE_STEPS {
        run(step.label, step.program, step.args);
    }

    println!();
    if fail != 0 {
        println!("GATE: FAIL");
        return 1;
    }
    println!("GATE: PASS");
    0
}
