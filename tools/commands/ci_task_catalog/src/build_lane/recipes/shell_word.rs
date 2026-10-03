//! The recipe step lists, the step runner and the echo quoting of the `mk` lane.
//!
//! **Role:** every recipe's [`Step`] list, [`run_steps`] (echo, glibc guard, spawn, stop at the
//! first red step) and the `rust-ci` composite over the database lane's suite.
//! **Position:** private to [`super`], which re-exports the recipes; the CI task table repeats
//! several of these recipes as its borrowed rows.
//! **Signals & state:** none held; each step's child inherits stdio.
//! **Invariants:** the echoed line is rendered from the argv, folder and environment that run; a
//! signal is reported as a signal; every child `cargo` and `trunk` gets the target pin.

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
/// ── WHY [`process_runner::Run::terminal`] AND NOT A CAPTURE ─────────────────────────────────
///
/// A capturing run pipes both streams, which is right for a gate that parses output and wrong for
/// this lane twice over: a ten-minute `cargo build` behind a pipe shows the operator nothing until
/// it exits, and re-emitting captured text afterwards **invents an interleaving** — the hazard
/// `Run::merged_output`'s own documentation warns about. `make` let its children write straight to
/// the inherited fds, so this does too, and the acceptance diff is only exact because of it.
///
/// What a terminal run keeps is the part bash gets wrong: a child killed by a signal has **no**
/// exit code, and `128+n` is a fiction the shell invents. Under eight parallel worktrees the OOM
/// killer is a routine visitor, so that is surfaced as [`NotRun::Signalled`] and never as a
/// build failure. A terminal run gives the child no new session: `make` did not, and `leptos`
/// runs `trunk serve` in the foreground, where detaching from the controlling terminal's process
/// group would swallow the operator's Ctrl-C.
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

        let mut child = Run::new(&step.argv[0])
            .args(&step.argv[1..])
            .env("CARGO_TARGET_DIR", &effective);
        if let Some(d) = &step.cwd {
            child = child.cwd(cwd_root().join(d));
        }
        for (k, v) in &step.envs {
            child = child.env(k, v);
        }

        let code = match child.terminal() {
            Ok(code) => code,
            // The honest form of exit 127 — "it is not installed" is not "it ran and failed".
            Err(NotRun::ToolAbsent(_)) => {
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
            Err(NotRun::Signalled { signal, .. }) => {
                eprintln!(
                    "{}",
                    Verdict::did_not_run(
                        format!("mk: {}", step.echo()),
                        Kind::Pin,
                        NotRun::Signalled {
                            tool: step.echo(),
                            signal,
                        },
                    )
                );
                // 128+n is bash's fiction, but the *shell contract* callers have is a non-zero
                // code; the honest report is the line above, which names the signal and refuses
                // to call it a failure of the build.
                return Ok(128u8.saturating_add(signal as u8));
            }
            Err(e) => {
                eprintln!("mk: failed to spawn `{}`: {e}", step.echo());
                return Ok(1);
            }
        };
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

/// One step running `line` over `api` and every API crate of the workspace under `repo_root`,
/// from the repository root. The `api-test`, `rust-test`, `rust-clippy` and `rust-build` rows of
/// the CI task table run the same argv through [`crate::api_package_lane`].
///
/// # Errors
/// The API packages cannot be derived from the workspace.
fn api_package_step(repo_root: &Path, line: ApiLine) -> Result<Vec<Step>> {
    let argv = api_line_argv(repo_root, line)?;
    let words: Vec<&str> = argv.iter().map(String::as_str).collect();
    Ok(vec![Step::new(&words)])
}

/// `cargo build --all-targets` over `api` and every API crate.
///
/// # Errors
/// As [`api_package_step`].
pub(crate) fn rust_build(repo_root: &Path) -> Result<Vec<Step>> {
    api_package_step(repo_root, ApiLine::Build)
}

/// `cargo test --lib --bins` over `api` and every API crate.
///
/// # Errors
/// As [`api_package_step`].
pub(crate) fn rust_test(repo_root: &Path) -> Result<Vec<Step>> {
    api_package_step(repo_root, ApiLine::UnitTests)
}

pub(crate) fn rust_fmt() -> Vec<Step> {
    vec![
        Step::new(&["cargo", "fmt", "--check"]).cd(WEB),
        // `--all` covers the tooling crates, which the api-crate run does not.
        Step::new(&["cargo", "fmt", "--all", "--check"]),
    ]
}

/// `cargo clippy --all-targets -- -D warnings` over `api` and every API crate.
///
/// # Errors
/// As [`api_package_step`].
pub(crate) fn rust_clippy(repo_root: &Path) -> Result<Vec<Step>> {
    api_package_step(repo_root, ApiLine::Clippy)
}

/// Fmt / clippy / test for the offline service worker, and the wasm32 lint.
///
/// The offline service worker is named in every step: a crate the lane does not name reaches CI
/// only as a dependency of the frontend, so nothing fmt-checks it, nothing clippies it and its
/// tests never run with every feature on. The wasm32 lint lints every package
/// [`crate::wasm32_lint_lane::wasm_ci_lint_packages`] derives from the workspace under
/// `repo_root`, so a crate declaring `targets = "wasm32"` is linted from its first commit. Kept in
/// lockstep with the `wasm-ci` row in `crate::task_definitions` — the two are the same lane
/// spelled twice, and `wasm_ci_recipe_and_ci_task_row_run_the_same_lines` pins them.
///
/// # Errors
/// The wasm32 lint's packages cannot be derived from the workspace.
pub(crate) fn wasm_ci(repo_root: &Path) -> Result<Vec<Step>> {
    let wasm32_packages = crate::wasm32_lint_lane::wasm_ci_lint_packages(repo_root)?;
    let wasm32_argv = crate::wasm32_lint_lane::wasm32_clippy_argv(&wasm32_packages);
    let wasm32_words: Vec<&str> = wasm32_argv.iter().map(String::as_str).collect();
    Ok(vec![
        Step::new(&["cargo", "fmt", "--check", "-p", "offline_service_worker"]),
        Step::new(&[
            "cargo",
            "clippy",
            "-p",
            "offline_service_worker",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ]),
        Step::new(&wasm32_words),
        Step::new(&[
            "cargo",
            "test",
            "-p",
            "offline_service_worker",
            "--all-features",
        ]),
    ])
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
            "--",
            "-D",
            "warnings",
        ]),
        Step::new(&[
            "cargo",
            "clippy",
            "-p",
            "frontend",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
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
    crate::task_runner::run_database_test_suite()
}

/// The four step lists `rust-ci` runs before [`rust_test_it`], in order.
///
/// # Errors
/// As [`wasm_ci`].
fn rust_ci_recipes() -> Result<[Vec<Step>; 4]> {
    Ok([
        rust_fmt(),
        rust_clippy(&cwd_root())?,
        rust_build(&cwd_root())?,
        wasm_ci(&cwd_root())?,
    ])
}

/// The lines `rust-ci` runs, in order, as `--dry-run` prints them.
///
/// # Errors
/// As [`wasm_ci`].
pub(crate) fn rust_ci_lines() -> Result<Vec<String>> {
    let mut lines: Vec<String> = rust_ci_recipes()?
        .iter()
        .flatten()
        .map(Step::echo)
        .collect();
    lines.push(RUST_TEST_IT_COMMAND.to_string());
    Ok(lines)
}

/// `rust-ci` — fmt + clippy + build + wasm-ci + test-it, in that order, stopping at the first red.
///
/// Composed from the same leaf functions the individual targets use, which is what makes a hollow
/// composite structurally impossible: there is no second copy of the recipe to fall out of date.
pub(super) fn rust_ci() -> Result<u8> {
    for steps in rust_ci_recipes()? {
        let rc = run_steps(&steps)?;
        if rc != 0 {
            return Ok(rc);
        }
    }
    rust_test_it()
}

/// Does this module own `target`? The seam the database and CI lanes chain onto: chain them
/// as `if crate::build_lane::recipes::handles(t) { crate::build_lane::recipes::run(a) } else { database_operations::local_database::run(a) }` rather than merging
/// three dispatch tables into one file.
pub(crate) fn handles(target: &str) -> bool {
    TARGETS.contains(&target)
}

pub(super) fn unknown_target(target: &str) -> Result<u8> {
    eprintln!("mk: no such target: {target}");
    eprintln!("    known: {}", TARGETS.join(" "));
    Ok(2)
}
