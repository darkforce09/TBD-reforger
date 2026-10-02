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
        if code != 0 {
            return Ok(code as u8);
        }
    }
    Ok(0)
}

pub(super) fn is_rust_build_tool(prog: &str) -> bool {
    matches!(prog, "cargo" | "trunk")
}

pub(crate) fn rust_api() -> Vec<Step> {
    // `<this checkout>/target/dev-api`, NOT the shared cache: this starts a long-lived server that
    // must not wait in the shared build-lock queue. The build targets below exit, so they do not.
    let private = dev_api_target_dir().display().to_string();
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
        // `--all` covers the tooling crates, which the api-crate run does not.
        Step::new(&["cargo", "fmt", "--all", "--check"]),
    ]
}

pub(crate) fn rust_clippy() -> Vec<Step> {
    vec![Step::new(&["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]).cd(WEB)]
}

/// Fmt / clippy / test for the engine crates and the offline service worker.
///
/// Every crate that ships to the browser is named in every step, wasm32 included: a crate the
/// lane does not name reaches CI only as a dependency of the frontend, so nothing fmt-checks it,
/// nothing clippies it and its tests never run. Kept in lockstep with the `wasm-ci` row in
/// `crate::commands::ci::task_definitions` — the two are the same lane spelled twice, and
/// `mk_build_tests` pins the wasm32 line's echo and the two spellings against drift.
pub(crate) fn wasm_ci() -> Vec<Step> {
    vec![
        Step::new(&[
            "cargo",
            "fmt",
            "--check",
            "-p",
            "map_engine",
            "-p",
            "graphics_engine",
            "-p",
            "offline_service_worker",
        ]),
        Step::new(&[
            "cargo",
            "clippy",
            "-p",
            "map_engine",
            "-p",
            "graphics_engine",
            "-p",
            "offline_service_worker",
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
            "map_engine",
            "-p",
            "graphics_engine",
            "-p",
            "offline_service_worker",
            "--target",
            "wasm32-unknown-unknown",
            "--",
            "-D",
            "warnings",
        ]),
        Step::new(&["cargo", "test", "-p", "map_engine", "--all-features"]),
        Step::new(&["cargo", "test", "-p", "graphics_engine", "--all-features"]),
        Step::new(&[
            "cargo",
            "test",
            "-p",
            "offline_service_worker",
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
        "developer_tools",
        "--bin",
        "gate",
        "--",
        "doctor",
    ]));
    v
}

pub(crate) fn leptos_gates() -> Vec<Step> {
    // The **required editor-factory pre-close** path. It runs
    // `gate editor-suite` (incl. save-dialog-rect / entrance-motion-rect). Chromium stays OUT of
    // `cargo xtask platform wave gate` — see documentation/runbooks/factory_waves/README.md §5.
    // `leptos-gates: leptos-build gate-doctor` and `gate-doctor: leptos-build`. make builds a
    // prerequisite ONCE per run, so `trunk build --release` appears once here, not twice —
    // reproducing that dedupe is part of the byte-for-byte contract.
    let mut v = gate_doctor();
    v.push(Step::new(&[
        "cargo",
        "run",
        "-q",
        "-p",
        "developer_tools",
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
        "developer_tools",
        "--bin",
        "gate",
        "--",
        "v-suite",
        "verify",
    ]));
    v
}

/// `mortar-offline-gate` — a release build, then `gate mortar-offline`: the mortar calculator's
/// offline pack downloads on the first visit and the page solves with the server gone.
pub(crate) fn mortar_offline_gate() -> Vec<Step> {
    let mut v = leptos_build();
    v.push(Step::new(&[
        "cargo",
        "run",
        "-q",
        "-p",
        "developer_tools",
        "--bin",
        "gate",
        "--",
        "mortar-offline",
    ]));
    v
}

/// `ballistics-wasm-agreement` — the release build, then `gate ballistics-agreement`: the
/// fire-mission solver's wasm build in the browser against its native build, case by case.
pub(crate) fn ballistics_wasm_agreement() -> Vec<Step> {
    let mut v = leptos_build();
    v.push(Step::new(&[
        "cargo",
        "run",
        "-q",
        "-p",
        "developer_tools",
        "--bin",
        "gate",
        "--",
        "ballistics-agreement",
    ]));
    v
}

pub(crate) fn ci_local_leptos() -> Vec<Step> {
    vec![
        Step::new(&["cargo", "fmt", "-p", "frontend", "--check"]),
        Step::new(&[
            "cargo",
            "clippy",
            "-p",
            "frontend",
            "--target",
            "wasm32-unknown-unknown",
            "--all-targets",
        ]),
        Step::new(&["cargo", "test", "-p", "frontend"]),
        Step::new(&["trunk", "build", "--release"]).cd(FE),
    ]
}

/// The command `rust-ci`'s fifth step runs: the database lane's complete integration suite.
const RUST_TEST_IT_COMMAND: &str = "cargo xtask db test-it";

/// `rust-test-it`, `rust-ci`'s fifth step: [`RUST_TEST_IT_COMMAND`], called in process.
///
/// The database lane owns the whole run: it resolves the container runtime
/// (`TBD_CONTAINER_RUNTIME`, `podman`, `docker`, or either through the distrobox bridge), creates
/// a fresh database for the run and drops it and every per-binary database derived from it after
/// every outcome. This step therefore names no container runtime and keeps no cleanup of its own.
fn rust_test_it() -> Result<u8> {
    println!("{RUST_TEST_IT_COMMAND}");
    // Flush before the suite writes, or the echo lands after the output it labels.
    let _ = std::io::stdout().flush();
    crate::commands::db::operations::test_it::run_complete_suite()
}

/// The four step lists `rust-ci` runs before [`rust_test_it`], in order.
fn rust_ci_recipes() -> [Vec<Step>; 4] {
    [rust_fmt(), rust_clippy(), rust_build(), wasm_ci()]
}

/// The lines `rust-ci` runs, in order, as `--dry-run` prints them.
pub(crate) fn rust_ci_lines() -> Vec<String> {
    let mut lines: Vec<String> = rust_ci_recipes().iter().flatten().map(Step::echo).collect();
    lines.push(RUST_TEST_IT_COMMAND.to_string());
    lines
}

/// `rust-ci` — fmt + clippy + build + wasm-ci + test-it, in that order, stopping at the first red.
///
/// Composed from the same leaf functions the individual targets use, which is what makes a hollow
/// composite structurally impossible: there is no second copy of the recipe to fall out of date.
pub(super) fn rust_ci() -> Result<u8> {
    for steps in rust_ci_recipes() {
        let rc = run_steps(&steps)?;
        if rc != 0 {
            return Ok(rc);
        }
    }
    rust_test_it()
}

/// Does this module own `target`? The seam the database and CI lanes chain onto: chain them
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
