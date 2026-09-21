use super::*;

/// A recipe line split into `(cwd, argv)`. `cd <dir> && <cmd>` means "run this in that
/// directory" and is the only shell construct this lane's own lines use.
pub fn split_cmd(line: &str) -> (Option<&str>, Vec<&str>) {
    if let Some(rest) = line.strip_prefix("cd ")
        && let Some((dir, tail)) = rest.split_once(" && ")
    {
        return (Some(dir), tail.split_whitespace().collect());
    }
    (None, line.split_whitespace().collect())
}

pub fn find(name: &str) -> Option<&'static Task> {
    TASKS.iter().find(|t| t.name == name)
}

/// The command line a step ECHOES, or `None` for the shapes that echo nothing.
///
/// `verify ci-schema-parity` pins the `verify-mission-rest-size-limits` and `ci-local` rows
/// against being hollowed, and this accessor is how it reads them. [`Step::Native`] is
/// deliberately `None` — it carries no command line to pin, which is exactly why no `verify-*`
/// row uses that shape.
pub fn step_echo(s: &Step) -> Option<&'static str> {
    match s {
        Step::Cmd { line, .. } => Some(line),
        Step::Xtask { echo, .. } => Some(echo),
        Step::Shell { script, .. } => Some(script),
        Step::Task(_) | Step::Native { .. } => None,
    }
}

/// The task names a row delegates to — the successor to a recipe's `$(MAKE) <target>` lines.
pub fn invoked_tasks(t: &Task) -> Vec<&'static str> {
    t.steps
        .iter()
        .filter_map(|s| match s {
            Step::Task(n) => Some(*n),
            _ => None,
        })
        .collect()
}

/// `cargo xtask ci [<target>]`. No target lists the lane, as `make` with no target would not.
pub fn run(target: Option<&str>) -> i32 {
    let Some(name) = target else {
        return help();
    };
    match find(name) {
        Some(t) => run_task(t),
        None => {
            eprintln!("xtask ci: no such task: {name}");
            eprintln!("  `cargo xtask help` lists every task this lane owns.");
            2
        }
    }
}

/// Run one task's steps, stopping at the first non-zero — make's fail-fast, one shell per line.
///
/// Returns the **leaf's** code, not make's flattened 2. See §3.
pub fn run_task(t: &Task) -> i32 {
    run_task_in(t, TASKS)
}

/// `run_task` against an explicit table. The indirection exists so the "a composite fails when
/// one of its leaves fails" property is provable on a synthetic table, without either shelling
/// out or perturbing the real tree — the recursion under test is the same code path.
pub fn run_task_in(t: &Task, all: &[Task]) -> i32 {
    for s in t.steps {
        let rc = run_step(s, all);
        if rc != 0 {
            return rc;
        }
    }
    0
}

pub(super) fn run_step(s: &Step, all: &[Task]) -> i32 {
    match s {
        Step::Task(name) => {
            let Some(t) = all.iter().find(|t| t.name == *name) else {
                // Unreachable while the parity test passes; a loud refusal rather than a silent
                // skip, because a composite that skips a step is the defect this module prevents.
                eprintln!("xtask ci: composite references unknown task `{name}` — refusing to");
                eprintln!("  report a result for a sequence with a missing step.");
                return 2;
            };
            echo(&format!("cargo xtask ci {name}"));
            run_task_in(t, all)
        }
        Step::Cmd { line, silent } => {
            if !silent {
                echo(line);
            }
            let (cwd, argv) = split_cmd(line);
            spawn(cwd, &argv)
        }
        Step::Xtask {
            echo: e,
            silent,
            run,
        } => {
            if !silent {
                echo(e);
            }
            // Reproduce main()'s error handler exactly: an `Err` there prints `xtask: {e:#}` and
            // exits 1, and that text is part of the observed output (`make schema-validate` on a
            // tree whose DEM is an LFS pointer prints `xtask: dem decode: …`).
            let rc = match run() {
                Ok(code) => code as i32,
                Err(e) => {
                    flush();
                    eprintln!("xtask: {e:#}");
                    1
                }
            };
            flush();
            rc
        }
        Step::Native { run } => {
            let rc = run();
            flush();
            rc
        }
        Step::Shell {
            silent,
            script,
            ignore_err,
        } => {
            if !silent {
                echo(script);
            }
            let rc = spawn(None, &["/bin/sh", "-c", script]);
            if *ignore_err { 0 } else { rc }
        }
    }
}

pub(super) fn echo(line: &str) {
    println!("{line}");
    flush();
}

/// stdout is BLOCK-buffered when piped to a file, so an un-flushed echo would surface *after* the
/// child it announces. Every acceptance capture in this program is `> file 2>&1`; without this the
/// ordering would differ from make's for reasons that have nothing to do with the port.
pub(super) fn flush() {
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
}

/// Spawn with stdio INHERITED — make lets recipe children write straight to the terminal, and
/// capturing here would both hide a long build's progress and invent an interleaving.
pub(super) fn spawn(cwd: Option<&str>, argv: &[&str]) -> i32 {
    flush();
    let root = find_repo_root().unwrap_or_else(|_| PathBuf::from("."));
    let mut c = Command::new(argv[0]);
    c.args(&argv[1..]);
    let dir = match cwd {
        Some(d) => root.join(d),
        None => root.clone(),
    };
    c.current_dir(&dir);
    // A shell updates `PWD` when it `cd`s; `Command::current_dir` does not, and the Makefile's
    // recipes are `cd $(WEB) && …` under sh. Left stale it would name the wrong directory to any
    // child that trusts it (`xtask fetch vanilla-api` in this very binary reads `$PWD`).
    c.env("PWD", &dir);
    apply_env(&mut c, &root);
    match c.status() {
        Ok(st) => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                if let Some(sig) = st.signal() {
                    // A signal is not an exit code (verification_core::proc §1). Say so in the shared
                    // library's words instead of letting 128+n read as an ordinary failure.
                    let nr = NotRun::Signalled {
                        tool: argv[0].to_string(),
                        signal: sig,
                    };
                    eprintln!("xtask ci: {nr:?} — the process died, it did not report.");
                    return 128 + sig;
                }
            }
            st.code().unwrap_or(127)
        }
        // `command not found` is 127 in a shell; make's recipes reach the same number through sh.
        Err(e) => {
            eprintln!("xtask ci: {}: {e}", argv[0]);
            127
        }
    }
}

/// The Makefile's two exported variables (`Makefile:7` PATH, `Makefile:16` CARGO_TARGET_DIR).
/// Both are load-bearing — see §3. `?=` semantics for the target dir: an existing export wins.
pub(super) fn apply_env(c: &mut Command, root: &Path) {
    for k in CARGO_RUN_INJECTED {
        c.env_remove(k);
    }
    for (k, _) in std::env::vars() {
        if k.starts_with("CARGO_PKG_") {
            c.env_remove(k);
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = home.to_string_lossy().into_owned();
        let inherited = std::env::var("PATH").unwrap_or_default();
        c.env(
            "PATH",
            format!("{home}/.cargo/bin:{home}/.local/go/bin:{home}/go/bin:{inherited}"),
        );
    }
    if std::env::var_os("CARGO_TARGET_DIR").is_none() {
        c.env("CARGO_TARGET_DIR", shared_target_dir(root));
    }
}

/// `$(TBD_REPO_ROOT)/target` — the PRIMARY checkout's target, shared by every linked worktree
/// (T-253). `git rev-parse --git-common-dir` is what makes a worktree resolve to its primary.
pub(super) fn shared_target_dir(root: &Path) -> PathBuf {
    let out = Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .current_dir(root)
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let p = PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string());
            match p.file_name().map(|f| f == ".git") {
                Some(true) => p.parent().unwrap_or(root).join("target"),
                _ => root.join("target"),
            }
        }
        _ => root.join("target"),
    }
}

/// DOCUMENTATION_STANDARDS §8.2 — `Makefile:331`:
/// `@! find apps packages -type f -path '*/docs/*.md' ! -path '*/node_modules/*' 2>/dev/null | grep -q . || (echo … && exit 1)`
///
/// Two fail-opens in that line, both closed here (see §3): `2>/dev/null` hid an unreadable
/// directory, and an absent `grep` exits 127, which `! …` turns into a PASS. The message text and
/// the stdout stream are preserved byte-for-byte — the `echo` runs inside `( … )`, so it is
/// stdout, not stderr.
/// `find … -type f -path '*/docs/*.md' ! -path '*/node_modules/*'`.
///
/// find's `*` crosses `/`, so `*/docs/*.md` is "any `.md` at any depth below any directory named
/// `docs`" — NOT just `apps/<x>/docs/<y>.md`. Kept as a predicate over the path string so the
/// glob semantics are testable without planting files in the real tree.
pub fn is_forbidden_doc(path: &str) -> bool {
    path.ends_with(".md") && path.contains("/docs/") && !path.contains("/node_modules/")
}

pub(super) fn verify_doc_layout() -> i32 {
    let root = match find_repo_root() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("xtask: {e:#}");
            return 1;
        }
    };
    let roots = [
        root.join("apps"),
        root.join("contracts_v2"),
        root.join("assets_v2"),
    ];
    let refs: Vec<&Path> = roots.iter().map(|p| p.as_path()).collect();
    let hits =
        verification_core::scan::walk_files(&refs, |p| is_forbidden_doc(&p.to_string_lossy()));
    match hits {
        Ok(found) if found.is_empty() => 0,
        Ok(_) => {
            println!("{DOC_LAYOUT_MSG}");
            1
        }
        Err(nr) => {
            eprintln!("verify-doc-layout: DID NOT RUN — {nr:?}");
            eprintln!("  A tree that could not be read is not a clean tree. (`2>/dev/null` in the");
            eprintln!("  Makefile recipe hid exactly this; T-896 closed it.)");
            2
        }
    }
}

/// `cargo xtask help` — the successor to `make help`.
///
/// Same column format as the awk one-liner it replaces (`  \033[36m%-22s\033[0m %s`), so the
/// visual contract survives the Makefile; grouped, because the flat 60-row list was ordered by
/// where a target happened to sit in the file. Rendered from [`TASKS`], so a task cannot be added
/// without appearing here.
///
/// T-897: this is now the ONLY task index — the Makefile it mirrored is gone. It cannot render
/// the other two lanes' rows (T-894's `db` is a clap enum, T-895's `mk` a `&[&str]`, neither
/// carrying help text), so it POINTS at them rather than transcribing a third copy that would
/// rot. `cargo xtask mk` and `cargo xtask db --help` each list their own.
pub fn help() -> i32 {
    println!("TBD Reforger — `cargo xtask` task surface. Run `cargo xtask ci <task>`.");
    for group in ["CI", "schema", "verify", "map", "build", "db"] {
        let rows: Vec<&Task> = TASKS.iter().filter(|t| t.group == group).collect();
        if rows.is_empty() {
            continue;
        }
        println!("\n{group}:");
        for t in rows {
            let tag = match t.lane {
                Lane::Ci => String::new(),
                Lane::Alias => " [alias]".to_string(),
                Lane::Borrowed => " [borrowed]".to_string(),
            };
            println!("  \x1b[36m{:<22}\x1b[0m {}{}", t.name, t.help, tag);
        }
    }
    println!("\n  [alias]     a one-line wrapper on an existing `cargo xtask verify …` command.");
    println!("  [borrowed]  the build or database lane — carried so the CI composites really run.");
    println!("\nThe other two lanes list themselves — they are not reprinted here, because a copy");
    println!("of a list is a list that rots:");
    println!(
        "  \x1b[36m{:<22}\x1b[0m build/dev-server lane: {}",
        "cargo xtask mk",
        crate::commands::build::recipes::TARGETS.join(" ")
    );
    println!(
        "  \x1b[36m{:<22}\x1b[0m database lane: {}",
        "cargo xtask db --help",
        crate::commands::db::operations::LANE_COMMANDS.join(" ")
    );
    println!("\n`cargo xtask --help` lists the full CLI (ticket, mcp, mod, deploy, schema, …).");
    0
}

/// `cargo xtask schema list-gates` — the input `wave.sh`'s drift tripwire loses with the Makefile.
///
/// `scripts/platform/wave.sh:1598` awks the `schema-validate` recipe and refuses to report PASS
/// when the parse comes back empty (T-420/T-422). This prints the same set, derived from the
/// `schema-validate` row of [`TASKS`] — the code that runs the gates — so the replacement input
/// is the executable list itself and not a third transcription of it.
pub fn schema_list_gates() -> i32 {
    let Some(t) = find("schema-validate") else {
        eprintln!("xtask schema list-gates: the schema-validate task is missing from TASKS.");
        return 1;
    };
    for g in validate_gate_names(t) {
        println!("{g}");
    }
    0
}

/// Sub-gate names from the `schema-validate` row: the last word of each step's echoed command.
pub fn validate_gate_names(t: &Task) -> Vec<String> {
    t.steps
        .iter()
        .filter_map(|s| match s {
            Step::Xtask { echo, .. } => echo.strip_prefix("cargo xtask schema "),
            _ => None,
        })
        .map(str::to_string)
        .collect()
}
