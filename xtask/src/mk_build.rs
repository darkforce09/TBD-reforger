//! `cargo xtask mk <target>` — the Makefile's **build/test lane**, in Rust.
//!
//! T-853 Phase 3, slice T-895. Sixteen `make` targets move here byte-for-byte:
//! `rust-api rust-build rust-test rust-fmt rust-clippy rust-ci rust-sqlx-prepare wasm-ci
//! leptos leptos-debug leptos-build leptos-gates ci-local-leptos verify-cargo-target
//! print-cargo-target-dir reclaim-target-ci`. The Makefile itself is deleted by T-897; this slice
//! only has to make the equivalents exist and be provably identical.
//!
//! ── WHERE THE TARGET-DIR PIN LIVES ───────────────────────────────────────────────────────────
//!
//! In [`crate::mk_target_dir`], with the two `make` targets that police it. That module is the one
//! to read before changing anything here: `CARGO_TARGET_DIR` is derived from `git rev-parse
//! --git-common-dir` so that every linked worktree shares the PRIMARY repo's warm `target/`
//! (T-253/T-322), and a `.cargo/config.toml` `[env]` with `relative = true` would silently reverse
//! that.
//!
//! Because the pin is a *value we compute*, it must be **injected into every child cargo**
//! ([`run_steps`]) rather than left to inheritance: `make` `export`ed it, so its children saw it,
//! and an `xtask` invoked without it in the environment must reproduce that. The one recipe-level
//! override is `rust-api`'s private `$(CURDIR)/target-dev-api` (T-322) — the *other* root, and the
//! reason `mk_target_dir` has two.
//!
//! ── OUTPUT IS A CONTRACT ─────────────────────────────────────────────────────────────────────
//!
//! `make` echoes each recipe line to **stdout** before running it, and the child's own output goes
//! wherever the child writes it. Acceptance for this slice is a stdout+stderr+rc diff against
//! `make`, so [`Step::echo`] reproduces those lines — and derives them **from the argv/cwd/env that
//! actually run**, so the label and the command cannot drift apart.
//!
//! Two deliberate divergences, both reported rather than papered over:
//!
//! 1. `make` collapses every failure to **rc 2** and prints `make: *** [Makefile:N: t] Error C` on
//!    stderr. Here the child's **raw** exit code is propagated and nothing extra is printed. That
//!    is [`tbd_gate::proc`]'s rule (`compile.sh --selftest` passes only on exactly 1), and a
//!    Makefile line number is not something a Makefile-less tree can honestly print.
//! 2. Composites (`rust-ci`, `leptos-gates`) call Rust functions instead of `$(MAKE) sub-target`,
//!    so make's `make[1]: Entering/Leaving directory` scaffolding and its `make <target>` echo are
//!    absent. The leaf lines under them are byte-identical.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::Result;
use tbd_gate::proc::Run;
use tbd_gate::{Kind, NotRun, Verdict};

use crate::mk_target_dir::{
    DEV_API_TARGET, abi_guard, cwd_root, env_pin, primary_root, reclaim_target_ci,
    resolve_target_dir, verify_cargo_target,
};

// ── A RECIPE LINE ────────────────────────────────────────────────────────────────────────────

/// One line of a `make` recipe: where it runs, what it sets, what it execs.
///
/// `envs` are **recipe-level** assignments — the ones make echoed as part of the line, e.g.
/// `CARGO_TARGET_DIR=…/target-dev-api cargo run --bin api`. The inherited shared pin is NOT one of
/// these; it is injected by [`run_steps`] and was never echoed. That distinction is what
/// [`verify_cargo_target`] §5 checks, so it is structural rather than a convention.
pub(crate) struct Step {
    cwd: Option<String>,
    envs: Vec<(String, String)>,
    argv: Vec<String>,
    /// make's leading `-`: run it, ignore a non-zero status. Used by `rust-test-it`'s first DROP.
    ignore_error: bool,
}

impl Step {
    pub(crate) fn new(argv: &[&str]) -> Step {
        Step {
            cwd: None,
            envs: Vec::new(),
            argv: argv.iter().map(|s| (*s).to_string()).collect(),
            ignore_error: false,
        }
    }
    pub(crate) fn cd(mut self, dir: &str) -> Step {
        self.cwd = Some(dir.to_string());
        self
    }
    pub(crate) fn env(mut self, k: &str, v: &str) -> Step {
        self.envs.push((k.to_string(), v.to_string()));
        self
    }
    fn ignore_error(mut self) -> Step {
        self.ignore_error = true;
        self
    }

    /// The value of a **recipe-level** env assignment, if this line makes one.
    ///
    /// The accessor rather than a public field: `verify-cargo-target` §5 asks exactly this question
    /// and nothing else needs the vector. Keeping `envs` private is what stops a future caller from
    /// *appending* to a recipe from the outside, which is the shape that would let `rust-build`
    /// acquire a private target dir without the gate having anything to look at.
    pub(crate) fn recipe_env(&self, key: &str) -> Option<&str> {
        self.envs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The line `make` printed before running this — rendered FROM the fields that run.
    ///
    /// Hand-writing the label next to the argv is how a port drifts: the two are edited apart and
    /// the tool then narrates something it is not doing. Here there is only one source.
    ///
    /// `shell_word` re-quotes arguments containing whitespace because make echoed the recipe
    /// *text*, and every such argument in this lane was written quoted (the three `psql -qc "…"`
    /// calls). Nothing here contains a `"` or a `$`, so the naive rule is exact; a future argument
    /// that does would need real quoting, and [`tests::echo_matches_make`] would catch it.
    pub(crate) fn echo(&self) -> String {
        let mut s = String::new();
        if let Some(d) = &self.cwd {
            s.push_str(&format!("cd {d} && "));
        }
        for (k, v) in &self.envs {
            s.push_str(&format!("{k}={v} "));
        }
        s.push_str(
            &self
                .argv
                .iter()
                .map(|a| shell_word(a))
                .collect::<Vec<_>>()
                .join(" "),
        );
        s
    }
}

fn shell_word(a: &str) -> String {
    if a.contains(' ') || a.contains('\t') {
        format!("\"{a}\"")
    } else {
        a.to_string()
    }
}

// ── THE RUNNER ───────────────────────────────────────────────────────────────────────────────

/// Run a recipe: echo each line to stdout, exec it, stop at the first failure.
///
/// ── WHY NOT `proc::Run` HERE ─────────────────────────────────────────────────────────────────
///
/// [`tbd_gate::proc::Run`] pipes both streams by design, which is right for a gate that parses
/// output and wrong for this lane twice over: a ten-minute `cargo build` behind a pipe shows the
/// operator nothing until it exits, and re-emitting captured text afterwards **invents an
/// interleaving** — the hazard `Run::merged_output`'s own documentation warns about. `make` let
/// its children write straight to the inherited fds, so this does too, and the acceptance diff is
/// only exact because of it.
///
/// What is kept from `proc::Run` is the part bash gets wrong: a child killed by a signal has **no**
/// exit code, and `128+n` is a fiction the shell invents. Under eight parallel worktrees the OOM
/// killer is a routine visitor, so that is surfaced as [`NotRun::Signalled`] and never as a
/// build failure. `setsid` is deliberately NOT used (unlike `proc::Run`): `make` did not, and
/// `leptos` runs `trunk serve` in the foreground, where detaching from the controlling terminal's
/// process group would swallow the operator's Ctrl-C.
fn run_steps(steps: &[Step]) -> Result<u8> {
    let pin = resolve_target_dir(env_pin().as_deref());
    for step in steps {
        // The pin governs the dir this step will actually write into: a recipe-level override
        // (rust-api's private dir) wins, exactly as it did under make's `export` + inline
        // assignment.
        let effective = step
            .envs
            .iter()
            .find(|(k, _)| k == "CARGO_TARGET_DIR")
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| pin.clone());
        if is_rust_build_tool(&step.argv[0]) {
            if let Err(msg) = abi_guard(Path::new(&effective)) {
                eprintln!("{msg}");
                return Ok(1);
            }
        }

        println!("{}", step.echo());
        // Flush before the child inherits stdout, or the echo lands after the output it labels.
        let _ = std::io::stdout().flush();

        let mut cmd = Command::new(&step.argv[0]);
        cmd.args(&step.argv[1..])
            .env("CARGO_TARGET_DIR", &effective)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        if let Some(d) = &step.cwd {
            cmd.current_dir(cwd_root().join(d));
        }
        for (k, v) in &step.envs {
            cmd.env(k, v);
        }

        let status = match cmd.status() {
            Ok(s) => s,
            // The honest form of exit 127 — "it is not installed" is not "it ran and failed".
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!(
                    "{}",
                    Verdict::did_not_run(
                        format!("mk: {}", step.echo()),
                        Kind::Pin,
                        NotRun::ToolAbsent(step.argv[0].clone()),
                    )
                );
                return Ok(127);
            }
            Err(e) => {
                eprintln!("mk: failed to spawn `{}`: {e}", step.echo());
                return Ok(1);
            }
        };
        if let Some(sig) = std::os::unix::process::ExitStatusExt::signal(&status) {
            eprintln!(
                "{}",
                Verdict::did_not_run(
                    format!("mk: {}", step.echo()),
                    Kind::Pin,
                    NotRun::Signalled {
                        tool: step.echo(),
                        signal: sig,
                    },
                )
            );
            // 128+n is bash's fiction, but the *shell contract* callers have is a non-zero code;
            // the honest report is the line above, which names the signal and refuses to call it
            // a failure of the build.
            return Ok(128u8.saturating_add(sig as u8));
        }
        let code = status.code().unwrap_or(1);
        if code != 0 && !step.ignore_error {
            return Ok(code as u8);
        }
    }
    Ok(0)
}

fn is_rust_build_tool(prog: &str) -> bool {
    matches!(prog, "cargo" | "trunk")
}

// ── THE RECIPES ──────────────────────────────────────────────────────────────────────────────

pub(crate) const WEB: &str = "apps/website/api";
const FE: &str = "apps/website/frontend";

pub(crate) fn rust_api() -> Vec<Step> {
    // `$(CURDIR)/target-dev-api`, NOT the shared cache: this starts the same long-lived server as
    // `make api`, so it needs the same isolation (T-322). Build targets below exit, so they do not.
    let private = cwd_root().join(DEV_API_TARGET).display().to_string();
    vec![
        Step::new(&["cargo", "run", "--bin", "api"])
            .cd(WEB)
            .env("CARGO_TARGET_DIR", &private),
    ]
}
pub(crate) fn rust_build() -> Vec<Step> {
    vec![Step::new(&["cargo", "build", "--all-targets"]).cd(WEB)]
}
pub(crate) fn rust_test() -> Vec<Step> {
    vec![Step::new(&["cargo", "test", "--lib", "--bins"]).cd(WEB)]
}
pub(crate) fn rust_fmt() -> Vec<Step> {
    vec![
        Step::new(&["cargo", "fmt", "--check"]).cd(WEB),
        // `--all` covers xtask/tbd-tools, which the api-crate run does not (T-297).
        Step::new(&["cargo", "fmt", "--all", "--check"]),
    ]
}
pub(crate) fn rust_clippy() -> Vec<Step> {
    vec![Step::new(&["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]).cd(WEB)]
}
pub(crate) fn rust_sqlx_prepare() -> Vec<Step> {
    vec![Step::new(&["cargo", "sqlx", "prepare"]).cd(WEB)]
}
pub(crate) fn wasm_ci() -> Vec<Step> {
    vec![
        Step::new(&[
            "cargo",
            "fmt",
            "--check",
            "-p",
            "map-engine-core",
            "-p",
            "map-engine-render",
        ]),
        Step::new(&[
            "cargo",
            "clippy",
            "-p",
            "map-engine-core",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ]),
        Step::new(&[
            "cargo",
            "clippy",
            "-p",
            "map-engine-render",
            "--target",
            "wasm32-unknown-unknown",
            "--",
            "-D",
            "warnings",
        ]),
        Step::new(&["cargo", "test", "-p", "map-engine-core", "--all-features"]),
        Step::new(&["cargo", "test", "-p", "map-engine-render"]),
    ]
}
pub(crate) fn leptos() -> Vec<Step> {
    vec![Step::new(&["trunk", "serve", "--release"]).cd(FE)]
}
pub(crate) fn leptos_debug() -> Vec<Step> {
    vec![Step::new(&["trunk", "serve"]).cd(FE)]
}
pub(crate) fn leptos_build() -> Vec<Step> {
    vec![Step::new(&["trunk", "build", "--release"]).cd(FE)]
}
pub(crate) fn gate_doctor() -> Vec<Step> {
    let mut v = leptos_build();
    v.push(Step::new(&[
        "cargo",
        "run",
        "-q",
        "-p",
        "tbd-tools",
        "--bin",
        "gate",
        "--",
        "doctor",
    ]));
    v
}
pub(crate) fn leptos_gates() -> Vec<Step> {
    // T-843 option (b): this is the **required editor-factory pre-close** path. It runs
    // `gate editor-suite` (incl. save-dialog-rect / entrance-motion-rect). Chromium stays OUT of
    // `cargo xtask platform wave gate` — see docs/platform/EDITOR_FACTORY_FOR_CURSOR.md §5.
    // `leptos-gates: leptos-build gate-doctor` and `gate-doctor: leptos-build`. make builds a
    // prerequisite ONCE per run, so `trunk build --release` appears once here, not twice —
    // reproducing that dedupe is part of the byte-for-byte contract.
    let mut v = gate_doctor();
    v.push(Step::new(&[
        "cargo",
        "run",
        "-q",
        "-p",
        "tbd-tools",
        "--bin",
        "gate",
        "--",
        "editor-suite",
    ]));
    v.push(Step::new(&[
        "cargo",
        "run",
        "-q",
        "-p",
        "tbd-tools",
        "--bin",
        "gate",
        "--",
        "v-suite",
        "verify",
    ]));
    v
}
pub(crate) fn ci_local_leptos() -> Vec<Step> {
    vec![
        Step::new(&["cargo", "fmt", "-p", "website-frontend", "--check"]),
        Step::new(&[
            "cargo",
            "clippy",
            "-p",
            "website-frontend",
            "--target",
            "wasm32-unknown-unknown",
            "--all-targets",
        ]),
        Step::new(&["cargo", "test", "-p", "website-frontend"]),
        Step::new(&["trunk", "build", "--release"]).cd(FE),
    ]
}

/// `rust-test-it` — **T-894 owns the public target**; this is `rust-ci`'s fifth step.
///
/// It is duplicated here on purpose and the duplication is the smaller error. `rust-ci` is
/// `fmt + clippy + build + wasm-ci + test-it`; a composite that silently drops a step is the T-489
/// hollow-composite defect, and this slice may not edit T-894's files (they land in parallel).
/// **At merge: delete this and call T-894's `db test-it`** — the recipes must not diverge.
pub(crate) fn rust_test_it() -> Vec<Step> {
    let psql = |flag: &str, sql: &str| {
        Step::new(&[
            "podman",
            "exec",
            "tbd_reforger_db",
            "psql",
            "-U",
            "tbd",
            "-d",
            "tbd_reforger",
            flag,
            sql,
        ])
    };
    vec![
        // make's leading `-`: the DROP is allowed to fail (first run, no such DB).
        psql("-qc", "DROP DATABASE IF EXISTS rust_it WITH (FORCE);").ignore_error(),
        psql("-qc", "CREATE DATABASE rust_it;"),
        Step::new(&["cargo", "test"]).cd(WEB).env(
            "TEST_DATABASE_URL",
            "postgres://tbd:tbd@localhost:5434/rust_it?sslmode=disable",
        ),
    ]
}

/// T-558's reaper: drop `rust_it` and every per-binary `rust_it_<suite>_it` T-534 provisioned.
///
/// `@`-prefixed in the Makefile, so it is NOT echoed — and its `while read -r db` loop over psql
/// output was the one piece of genuinely non-trivial shell in the file. Here it is a captured
/// string and a `for`, so the bash hazard of a subshell death vanishing into an empty loop is gone.
fn reap_rust_it_databases() {
    const SELECT: &str = "SELECT datname FROM pg_database WHERE datname = 'rust_it' \
                          OR datname LIKE 'rust_it\\_%\\_it' ESCAPE '\\'";
    let listed = Run::new("podman")
        .args([
            "exec",
            "tbd_reforger_db",
            "psql",
            "-U",
            "tbd",
            "-d",
            "tbd_reforger",
            "-Atc",
            SELECT,
        ])
        .output();
    let Ok(out) = listed else { return };
    for db in out.stdout.lines().map(str::trim).filter(|s| !s.is_empty()) {
        // `>/dev/null` in the bash; the drop's own chatter is noise, its failure is not fatal.
        let _ = Run::new("podman")
            .args([
                "exec",
                "tbd_reforger_db",
                "psql",
                "-U",
                "tbd",
                "-d",
                "tbd_reforger",
                "-qc",
                &format!("DROP DATABASE IF EXISTS {db} WITH (FORCE);"),
            ])
            .output();
    }
}

/// `rust-ci` — fmt + clippy + build + wasm-ci + test-it, in that order, stopping at the first red.
///
/// Composed from the same leaf functions the individual targets use, which is what makes a hollow
/// composite structurally impossible: there is no second copy of the recipe to fall out of date.
fn rust_ci() -> Result<u8> {
    for steps in [rust_fmt(), rust_clippy(), rust_build(), wasm_ci()] {
        let rc = run_steps(&steps)?;
        if rc != 0 {
            return Ok(rc);
        }
    }
    let rc = run_steps(&rust_test_it())?;
    reap_rust_it_databases();
    Ok(rc)
}
// ── DISPATCH ─────────────────────────────────────────────────────────────────────────────────

/// Every Makefile target this module answers to, in `make help` order.
pub(crate) const TARGETS: &[&str] = &[
    "print-cargo-target-dir",
    "verify-cargo-target",
    "reclaim-target-ci",
    "rust-api",
    "rust-build",
    "rust-test",
    "rust-fmt",
    "rust-clippy",
    "rust-sqlx-prepare",
    "rust-ci",
    "wasm-ci",
    "leptos",
    "leptos-debug",
    "leptos-build",
    "gate-doctor",
    "leptos-gates",
    "ci-local-leptos",
];

/// Does this module own `target`? The seam for T-894/T-896, which add their own lanes: chain them
/// as `if mk_build::handles(t) { mk_build::run(a) } else { mk_db::run(a) }` rather than merging
/// three dispatch tables into one file.
pub(crate) fn handles(target: &str) -> bool {
    TARGETS.contains(&target)
}

fn unknown_target(target: &str) -> Result<u8> {
    eprintln!("mk: no such target: {target}");
    eprintln!("    known: {}", TARGETS.join(" "));
    Ok(2)
}

/// `cargo xtask mk <target> [--dry-run]`.
///
/// `--dry-run` is the analog of `make -n`: it prints the recipe lines without running them. It is
/// not a convenience — it is the only deterministic acceptance surface for the targets that never
/// terminate (`leptos`, `leptos-debug`) or that start a server (`rust-api`).
pub(crate) fn run(args: &[String]) -> Result<u8> {
    let dry = args.iter().any(|a| a == "--dry-run" || a == "-n");
    let target = args.iter().find(|a| !a.starts_with('-')).cloned();
    let Some(target) = target else {
        println!("usage: cargo xtask mk <target> [--dry-run]");
        for t in TARGETS {
            println!("  {t}");
        }
        return Ok(if args.iter().any(|a| a == "--list") {
            0
        } else {
            2
        });
    };

    // Asked and answered once, so the advertised list and the dispatch below cannot disagree —
    // and so `handles`, the T-894/T-896 chaining seam, is the same predicate callers get.
    if !handles(&target) {
        return unknown_target(&target);
    }

    // The three non-recipe targets: they compute or delete, they do not spawn a build.
    match target.as_str() {
        "print-cargo-target-dir" => {
            println!("{}", resolve_target_dir(env_pin().as_deref()));
            return Ok(0);
        }
        "verify-cargo-target" => return verify_cargo_target(&cwd_root()),
        "reclaim-target-ci" => return reclaim_target_ci(&primary_root()),
        _ => {}
    }

    let steps = match target.as_str() {
        "rust-api" => rust_api(),
        "rust-build" => rust_build(),
        "rust-test" => rust_test(),
        "rust-fmt" => rust_fmt(),
        "rust-clippy" => rust_clippy(),
        "rust-sqlx-prepare" => rust_sqlx_prepare(),
        "wasm-ci" => wasm_ci(),
        "leptos" => leptos(),
        "leptos-debug" => leptos_debug(),
        "leptos-build" => leptos_build(),
        "gate-doctor" => gate_doctor(),
        "leptos-gates" => leptos_gates(),
        "ci-local-leptos" => ci_local_leptos(),
        "rust-ci" => {
            if dry {
                let mut all = Vec::new();
                for s in [
                    rust_fmt(),
                    rust_clippy(),
                    rust_build(),
                    wasm_ci(),
                    rust_test_it(),
                ] {
                    all.extend(s);
                }
                all
            } else {
                return rust_ci();
            }
        }
        // Reachable only if TARGETS advertises something with no recipe — which
        // `tests::every_advertised_target_dispatches` forbids. Reported, never panicked.
        other => return unknown_target(other),
    };

    if dry {
        for s in &steps {
            println!("{}", s.echo());
        }
        return Ok(0);
    }
    run_steps(&steps)
}
