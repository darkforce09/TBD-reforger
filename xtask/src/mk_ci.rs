//! `mk_ci` — the Makefile's CI / composite / map lane, as first-class xtask tasks (T-896).
//!
//! T-853 Phase 3. The root `Makefile` is being absorbed into `xtask`; T-897 deletes it. This
//! module owns the CI lane: `ci-local`, `ci-local-schema`, `schema-validate`, `schema-codegen`,
//! `verify-citations`, `verify-coding-standards`, `verify-doc-layout`, `verify-editorconfig`, the
//! three `map-*` composites, `lfs-dem`, `lfs-sat`, `help`, `test`, `build`.
//!
//! ── 1. WHY A TABLE AND NOT SIXTEEN FUNCTIONS ────────────────────────────────────────────────
//!
//! `ci-local`, `ci-local-schema` and `rust-ci` are *sequences of other targets*. Written as
//! sixteen independent functions, a composite would have to re-list what its parts do, and the
//! copy rots — that is exactly T-489's defect, where a make target reported success while its
//! recipe had been hollowed to `@true`. Here [`Step::Task`] names another row of [`TASKS`] and
//! the runner recurses into **the same** [`run_task`] the standalone command calls. A composite
//! therefore cannot drift from its parts or be hollowed independently of them: there is one
//! implementation of "what `verify-doc-layout` does", and both `cargo xtask ci verify-doc-layout`
//! and `cargo xtask ci ci-local` reach it through the identical call.
//!
//! The same table is the source for [`help`] (so a task cannot exist and be undiscoverable) and
//! for [`schema_list_gates`] (so `wave.sh`'s drift tripwire keeps an input after the Makefile
//! dies — see §4).
//!
//! ── 2. WHY SOME ROWS BELONG TO OTHER SLICES ─────────────────────────────────────────────────
//!
//! `ci-local` runs `rust-ci` and `ci-local-leptos`; `test` runs `rust-test`; `build` runs
//! `leptos-build`. Those targets are **T-895's** lane and `rust-test-it` is **T-894's**, and all
//! three slices are in flight on the same commit. Two options existed:
//!
//!   * stub them, and have `ci-local` report a green it did not earn — the defect this program
//!     exists to kill; or
//!   * carry the recipe here, marked [`Lane::Borrowed`], so the composite genuinely runs.
//!
//! The second, with a guard: `mk_ci_tests.rs` parses the Makefile and asserts every row's steps
//! reproduce that target's recipe **verbatim**, borrowed rows included. While the Makefile lives
//! they cannot drift; when it dies they are already proven equal, so the merge with T-894/T-895
//! is a deletion of duplicates, not a reconciliation of two guesses.
//!
//! [`Lane::Alias`] rows are different: `verify-no-python` and friends were *already* one-line
//! aliases for an existing `cargo xtask verify …`, so nothing is borrowed — the row just records
//! that the make name maps onto a command that has existed since T-165.
//!
//! ── 3. MAKEFILE ODDITIES PRESERVED ON PURPOSE ───────────────────────────────────────────────
//!
//! * **The PATH prepend** (`Makefile:7`) is load-bearing, not decoration: `editorconfig-checker`
//!   lives in `~/go/bin`, which is on no default PATH. [`apply_env`] reproduces the prepend for
//!   every child. Dropping it would turn `verify-editorconfig` into "command not found" on a
//!   correct machine.
//! * **`CARGO_TARGET_DIR ?=`** (`Makefile:16`, T-253) points every linked worktree at the primary
//!   checkout's warm `target/`, and `?=` lets an operator/wave export win. Not reproducing it
//!   would silently split the 52 GB cache per worktree. T-895 owns the *assertion* half
//!   (`verify-cargo-target`); this is the derivation half, and the two should become one helper
//!   when the lanes merge.
//! * **`-podman …`** in `rust-test-it` (`Makefile:205`) ignores failure; [`Step::Shell`] keeps the
//!   `ignore_err` flag rather than "fixing" a deliberate tolerance.
//!
//! **Fail-opens closed, and named:** `verify-doc-layout`'s recipe is
//! `! find … 2>/dev/null | grep -q .`. That has two holes — `2>/dev/null` hides an unreadable
//! directory, and a missing `grep` (exit 127) reads as "no match", i.e. as a pass. The Rust port
//! walks the tree with [`tbd_gate::scan::walk_files`], which returns [`NotRun`] for an unreadable
//! directory, and matches in-process so there is no `grep` to be absent. A tree it could not read
//! is reported, not swallowed.
//!
//! **make's own framing is NOT reproduced**, deliberately, and it is the one place where output
//! differs. GNU make prints `make[1]: Entering directory …` around every sub-make and collapses
//! **every** recipe failure to its own exit status 2 — the Makefile itself complains about that
//! flattening at `mod-compile-selftest` (`Makefile:286`), where a `case` had to be written to
//! recover the 1-vs-3 distinction make destroyed. This runner returns the **leaf's raw code**.
//! Acceptance diffs therefore normalise make's framing away and reconstruct the true rc from the
//! `make: *** [Makefile:NNN: t] Error N` line, which states it.
//!
//! ── 4. `xtask schema list-gates` ────────────────────────────────────────────────────────────
//!
//! `scripts/platform/wave.sh:1598` and its port `xtask/src/wave/schema.rs` **parse the Makefile's
//! `schema-validate` recipe** to cross-check `GATE_SCHEMA_VALIDATE_GATES`; that tripwire refuses
//! to report PASS when the parse comes back empty. Deleting the Makefile removes its input, so
//! the tripwire would go permanently red — or, worse, be quietly loosened. [`schema_list_gates`]
//! prints the set derived from the `schema-validate` row of [`TASKS`], i.e. from the code that
//! actually runs the gates, so the cross-check keeps a source that cannot be a stale second copy.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use tbd_gate::verdict::NotRun;

use crate::root::find_repo_root;

/// Who owns a row. Recorded because three slices are porting the Makefile at once and the answer
/// decides what a merge does with the row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lane {
    /// T-896 — this slice.
    Ci,
    /// The make target was already a 1:1 alias for a `cargo xtask` command that predates T-853.
    Alias,
    /// Another slice's lane, carried so this lane's composites genuinely run. See §2.
    Borrowed(&'static str),
}

/// One recipe line.
///
/// `silent` mirrors make's `@` prefix: without it make echoes the expanded line before running
/// it, and that echo is the operator's progress trace through an eleven-step gate.
pub enum Step {
    /// `$(MAKE) <target>` — recurse into the same [`TASKS`] row the standalone command runs.
    Task(&'static str),
    /// A child process. ONE datum: the make-expanded recipe line. It is both what gets echoed and
    /// what gets spawned ([`split_cmd`]), so the trace and the execution cannot disagree — a
    /// separate `argv` field is exactly the kind of second copy this module exists to avoid.
    /// No shell: none of this lane's own lines carry metacharacters or quoting, and
    /// `cmd_lines_are_shell_free` pins that.
    Cmd { line: &'static str, silent: bool },
    /// An **in-process** xtask leaf: the same function `cargo xtask <group> <cmd>` dispatches to.
    /// Nine subprocesses become nine calls, and the sub-gate list cannot drift from the CLI's.
    Xtask {
        echo: &'static str,
        silent: bool,
        run: fn() -> anyhow::Result<u8>,
    },
    /// A Rust port of a shell recipe (`verify-doc-layout`). Always silent: the only recipe with
    /// this shape is `@`-prefixed, and the parity test pins that.
    Native { run: fn() -> i32 },
    /// A recipe line handed to `/bin/sh -c` verbatim — **only** for [`Lane::Borrowed`] rows.
    /// Not a port and not claimed as one: it is the same shell make ran, kept byte-faithful
    /// (including `sh: podman: not found`) until the owning slice ports it properly.
    Shell {
        script: &'static str,
        silent: bool,
        ignore_err: bool,
    },
}

/// A recipe line split into `(cwd, argv)`. `cd <dir> && <cmd>` is make's idiom for "run this in
/// that directory" and is the only shell construct this lane's own lines use.
pub fn split_cmd(line: &str) -> (Option<&str>, Vec<&str>) {
    if let Some(rest) = line.strip_prefix("cd ")
        && let Some((dir, tail)) = rest.split_once(" && ")
    {
        return (Some(dir), tail.split_whitespace().collect());
    }
    (None, line.split_whitespace().collect())
}

pub struct Task {
    pub name: &'static str,
    /// The Makefile's `## ` text, verbatim — [`help`] reprints it in make's own column format.
    pub help: &'static str,
    pub group: &'static str,
    pub lane: Lane,
    pub steps: &'static [Step],
}

/// `verify-doc-layout`'s failure text, verbatim from `Makefile:332`. A `const` so the parity test
/// can pin it against the recipe rather than trusting two hand-copied sentences to agree.
pub const DOC_LAYOUT_MSG: &str =
    "FORBIDDEN: markdown under apps/**/docs/ or packages/**/docs/ — use docs/website/ instead";

// The table lives next door, split at the data/behaviour seam to keep both files inside SIZE-1.
// It is pure data: `run_task` below is its only interpreter.
#[path = "mk_ci_tasks.rs"]
mod tasks;
pub use tasks::TASKS;

/* ─────────────────────────────────── the runner ─────────────────────────────────── */

pub fn find(name: &str) -> Option<&'static Task> {
    TASKS.iter().find(|t| t.name == name)
}

/// The command line a step ECHOES, or `None` for the shapes that echo nothing.
///
/// T-897: `gate_t468` used to pin the Makefile's `verify-t456:` / `verify-t468:` recipe BODIES
/// against being hollowed to `@true`. Those recipes are gone; this table is their successor, and
/// this accessor is how that gate reads it. [`Step::Native`] is deliberately `None` — it carries
/// no command line to pin, which is exactly why no `verify-*` row uses that shape.
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

/// [`run_task`] against an explicit table. The indirection exists so the "a composite fails when
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

fn run_step(s: &Step, all: &[Task]) -> i32 {
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

fn echo(line: &str) {
    println!("{line}");
    flush();
}

/// stdout is BLOCK-buffered when piped to a file, so an un-flushed echo would surface *after* the
/// child it announces. Every acceptance capture in this program is `> file 2>&1`; without this the
/// ordering would differ from make's for reasons that have nothing to do with the port.
fn flush() {
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
}

/// Spawn with stdio INHERITED — make lets recipe children write straight to the terminal, and
/// capturing here would both hide a long build's progress and invent an interleaving.
fn spawn(cwd: Option<&str>, argv: &[&str]) -> i32 {
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
                    // A signal is not an exit code (tbd_gate::proc §1). Say so in the shared
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

/// Environment `cargo run` injects into this process and which must NOT reach a nested `cargo`.
///
/// MEASURED, this worktree, warm `target-ctr` (`/proc/<pid>/environ` of the inner cargo, make-side
/// vs xtask-side):
/// ```text
///   make test                            Compiling=0
///   cargo run -q -p xtask -- ci test     Compiling=13   <- hyper-rustls reqwest ring rustls
///   make test                            Compiling=13      rustls-platform-verifier rustls-webpki
///   ./target-ctr/debug/xtask ci test     Compiling=0       sqlx* tokio-rustls
///   make test                            Compiling=0
/// ```
/// The same binary launched directly costs nothing; launched through `cargo run` it makes the
/// backend's whole TLS stack thrash, and then `make` rebuilds it back — for as long as anyone
/// alternates the two. The env diff explains it: cargo exports `SSL_CERT_FILE`/`SSL_CERT_DIR`
/// (its own probed paths) and an `LD_LIBRARY_PATH` pointing into `target/debug`, and those are in
/// the build-script fingerprints of exactly that crate set.
///
/// This is the T-253/T-322 hazard in a new coat — a shared 52 GB target dir where two invocations
/// disagree — and it arrives with the Makefile's replacement, so it is closed here rather than
/// left to be rediscovered as "xtask is slow". The `CARGO_PKG_*` block is stripped for the same
/// reason plus an obvious one: `CARGO_PKG_NAME=xtask` is a lie to the child.
const CARGO_RUN_INJECTED: &[&str] = &[
    "CARGO",
    "CARGO_BIN_NAME",
    "CARGO_CRATE_NAME",
    "CARGO_MANIFEST_DIR",
    "CARGO_MANIFEST_PATH",
    "CARGO_PRIMARY_PACKAGE",
    "DYLD_FALLBACK_LIBRARY_PATH",
    "LD_LIBRARY_PATH",
    "SSL_CERT_DIR",
    "SSL_CERT_FILE",
];

/// The Makefile's two exported variables (`Makefile:7` PATH, `Makefile:16` CARGO_TARGET_DIR).
/// Both are load-bearing — see §3. `?=` semantics for the target dir: an existing export wins.
fn apply_env(c: &mut Command, root: &Path) {
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
fn shared_target_dir(root: &Path) -> PathBuf {
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

/* ───────────────────────────── verify-doc-layout (native) ───────────────────────────── */

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

fn verify_doc_layout() -> i32 {
    let root = match find_repo_root() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("xtask: {e:#}");
            return 1;
        }
    };
    let roots = [root.join("apps"), root.join("packages")];
    let refs: Vec<&Path> = roots.iter().map(|p| p.as_path()).collect();
    let hits = tbd_gate::scan::walk_files(&refs, |p| is_forbidden_doc(&p.to_string_lossy()));
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

/* ──────────────────────────────── help / list-gates ──────────────────────────────── */

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
    println!("TBD Reforger — `cargo xtask` task surface (T-853/T-897: the root Makefile is gone;");
    println!("this replaces `make help`. Run `cargo xtask ci <task>`).");
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
                Lane::Borrowed(who) => format!(" [{who}]"),
            };
            println!("  \x1b[36m{:<22}\x1b[0m {}{}", t.name, t.help, tag);
        }
    }
    println!("\n  [alias]  already a `cargo xtask verify …` command; the make name was a wrapper.");
    println!("  [T-89x]  that slice's lane — carried here so this lane's composites really run.");
    println!("\nThe other two lanes list themselves — they are not reprinted here, because a copy");
    println!("of a list is a list that rots:");
    println!(
        "  \x1b[36m{:<22}\x1b[0m build/dev-server lane: {}",
        "cargo xtask mk",
        crate::mk_build::TARGETS.join(" ")
    );
    println!(
        "  \x1b[36m{:<22}\x1b[0m database lane: {}",
        "cargo xtask db --help",
        crate::mk_db::LANE_COMMANDS.join(" ")
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

#[cfg(test)]
#[path = "mk_ci_tests.rs"]
mod tests;
