//! The mod wave driver's dispatch, readers, `status`, `prep` and `gate`.
//!
//! **Role:** reads the mod programme's share of the wave plan from the central ticket manager,
//! reports the current wave, prepares its worktrees and runs the wave gate.
//! **Position:** under [`crate::wave_execution`]; uses `platform_execution::slice_worktree` and
//! `ticket_manager_client`.
//! **Signals & state:** the mod plan, read once per process (`ttm show T-181`, `ttm wave show`);
//! each run reads the live git and worktree state.
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

/// The programme ticket whose children are the game-mod slices, by its legacy number; the
/// central ticket manager resolves it to the programme's slug.
pub(super) const MOD_PROGRAMME: &str = "T-181";

/// One mod slice of an open wave.
struct ModSliceRow {
    wave: u32,
    slug: String,
    title: String,
}

/// The mod programme's share of the wave plan: the open-wave rows of its children and which
/// children have shipped, read once per process from the central ticket manager.
struct ModPlan {
    rows: Vec<ModSliceRow>,
    shipped: Vec<String>,
}

/// The mod plan, or `None` when the ticket manager cannot name the programme or give the wave
/// plan (reported on stderr) — callers refuse loudly instead of shrugging into "ALL PLANNED WAVES
/// SHIPPED".
fn mod_plan() -> Option<&'static ModPlan> {
    static PLAN: OnceLock<Option<ModPlan>> = OnceLock::new();
    PLAN.get_or_init(load_mod_plan).as_ref()
}

fn load_mod_plan() -> Option<ModPlan> {
    let ticket_manager = TicketManager::from_env();
    let programme = match ticket_manager.show(MOD_PROGRAMME) {
        Ok(programme) => programme,
        Err(error) => {
            eprintln!("mod wave: {error}");
            return None;
        }
    };
    let plan = match ticket_manager.wave_show() {
        Ok(plan) => plan,
        Err(error) => {
            eprintln!("mod wave: {error}");
            return None;
        }
    };
    let is_member = |slug: &str| programme.children.iter().any(|child| child.slug == slug);
    let rows = plan
        .waves
        .iter()
        .flat_map(|wave| {
            wave.tickets
                .iter()
                .filter(|row| is_member(row.slug.as_str()))
                .map(move |row| ModSliceRow {
                    wave: wave.n,
                    slug: row.slug.to_string(),
                    title: row.title.clone(),
                })
        })
        .collect();
    // The programme's children carry their statuses; a slice counts once it has shipped.
    let shipped = programme
        .children
        .iter()
        .filter(|child| child.status == "shipped")
        .map(|child| child.slug.to_string())
        .collect();
    Some(ModPlan { rows, shipped })
}

pub(super) fn wave_slices(_root: &Path, w: &str) -> Vec<String> {
    let (Some(plan), Ok(n)) = (mod_plan(), w.parse::<u32>()) else {
        return Vec::new();
    };
    plan.rows
        .iter()
        .filter(|row| row.wave == n)
        .map(|row| row.slug.clone())
        .collect()
}

pub(super) fn slice_title(_root: &Path, s: &str) -> String {
    mod_plan()
        .and_then(|plan| plan.rows.iter().find(|row| row.slug == s))
        .map(|row| row.title.clone())
        .unwrap_or_default()
}

/// Open waves (n > 0) that hold at least one mod slice, ascending.
pub(super) fn unique_sorted_waves(_root: &Path) -> Vec<String> {
    let Some(plan) = mod_plan() else {
        return Vec::new();
    };
    let mut waves: Vec<u32> = plan
        .rows
        .iter()
        .filter(|row| row.wave > 0)
        .map(|row| row.wave)
        .collect();
    waves.sort_unstable();
    waves.dedup();
    waves.into_iter().map(|n| n.to_string()).collect()
}

/// The first open wave whose mod slices are not all shipped. `None` = the plan itself could not
/// be read (already reported) — a refusal, not "done".
pub(super) fn current_wave(root: &Path) -> Option<String> {
    let plan = mod_plan()?;
    for w in unique_sorted_waves(root) {
        let done_all = wave_slices(root, &w)
            .iter()
            .all(|s| plan.shipped.iter().any(|x| x == s));
        if !done_all {
            return Some(w);
        }
    }
    Some("done".to_string())
}

/// Sub-slices share the parent's worktree: a reference of three or more dot segments resolves
/// to its first two ([`ticket_manager_client::parent_slice`]).
pub(super) fn parent_slice(s: &str) -> String {
    ticket_manager_client::parent_slice(s).to_string()
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
            "ALL PLANNED WAVES SHIPPED. Next: queue mod tickets and `ttm --project reforger wave repack`, or close the program."
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
