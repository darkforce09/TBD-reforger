use super::*;

pub(super) fn shell_word(a: &str) -> String {
    if a.contains(' ') || a.contains('\t') {
        format!("\"{a}\"")
    } else {
        a.to_string()
    }
}

/// Run a recipe: echo each line to stdout, exec it, stop at the first failure.
///
/// ── WHY NOT `proc::Run` HERE ─────────────────────────────────────────────────────────────────
///
/// [`verification_core::proc::Run`] pipes both streams by design, which is right for a gate that parses
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
pub(super) fn run_steps(steps: &[Step]) -> Result<u8> {
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
        if is_rust_build_tool(&step.argv[0])
            && let Err(msg) = abi_guard(Path::new(&effective))
        {
            eprintln!("{msg}");
            return Ok(1);
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

pub(super) fn is_rust_build_tool(prog: &str) -> bool {
    matches!(prog, "cargo" | "trunk")
}

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
        // `--all` covers tools_v2/xtask/tbd-tools, which the api-crate run does not (T-297).
        Step::new(&["cargo", "fmt", "--all", "--check"]),
    ]
}

pub(crate) fn rust_clippy() -> Vec<Step> {
    vec![Step::new(&["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]).cd(WEB)]
}

pub(crate) fn rust_sqlx_prepare() -> Vec<Step> {
    vec![Step::new(&["cargo", "sqlx", "prepare"]).cd(WEB)]
}

/// Fmt / clippy / test for the engine crates.
///
/// `website-graphics-engine` was added to every step when the engine split created it: the crate
/// reached CI only as a transitive dependency of the frontend, so nothing fmt-checked it, nothing
/// clippied it and its tests never ran. Kept in lockstep with the `wasm-ci` row in
/// `crate::commands::ci::task_definitions` — the two are the same lane spelled twice, and `mk_build_tests` pins the
/// wasm32 line's echo against drift.
pub(crate) fn wasm_ci() -> Vec<Step> {
    vec![
        Step::new(&[
            "cargo",
            "fmt",
            "--check",
            "-p",
            "website-map-engine",
            "-p",
            "website-graphics-engine",
        ]),
        Step::new(&[
            "cargo",
            "clippy",
            "-p",
            "website-map-engine",
            "-p",
            "website-graphics-engine",
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
            "website-map-engine",
            "-p",
            "website-graphics-engine",
            "--target",
            "wasm32-unknown-unknown",
            "--",
            "-D",
            "warnings",
        ]),
        Step::new(&[
            "cargo",
            "test",
            "-p",
            "website-map-engine",
            "--all-features",
        ]),
        Step::new(&[
            "cargo",
            "test",
            "-p",
            "website-graphics-engine",
            "--all-features",
        ]),
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
        "developer-tools",
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
        "developer-tools",
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
        "developer-tools",
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
pub(super) fn reap_rust_it_databases() {
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
pub(super) fn rust_ci() -> Result<u8> {
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

/// Does this module own `target`? The seam for T-894/T-896, which add their own lanes: chain them
/// as `if crate::commands::build::recipes::handles(t) { crate::commands::build::recipes::run(a) } else { crate::commands::db::operations::run(a) }` rather than merging
/// three dispatch tables into one file.
pub(crate) fn handles(target: &str) -> bool {
    TARGETS.contains(&target)
}

pub(super) fn unknown_target(target: &str) -> Result<u8> {
    eprintln!("mk: no such target: {target}");
    eprintln!("    known: {}", TARGETS.join(" "));
    Ok(2)
}
