//! The full wave gate.
//!
//! **Role:** `cmd_gate` resolves and verifies the wave's base, takes the gate lock, invalidates
//! fingerprints and runs the wave gate's steps in order: cargo check, wasm32, format, the clippy
//! steps (the applications and library crates, the wasm32 members, the frontend family twice,
//! every tool crate), the migration and test steps, the trunk build when the SPA's scope changed,
//! the schema, catalogue, ticket and wave-lock steps, the shared verify steps and the language
//! gates.
//!
//! **Position:** re-exported by the parent `gate` module and reached through `wave gate [<base>]`;
//! `wave --close` runs it before writing a marker.
//!
//! **Signals & state:** the gate lock for the whole run; the step runner's fail flag.
//!
//! **Invariants:** a base git cannot resolve, a base that does not cover the wave and an empty
//! range each refuse with exit 2 before any step runs; the lock is held across every step, so the
//! verdict describes one tree; each clippy step lints with the flags of its CI job and `-D
//! warnings`, and the clippy steps together name every workspace member, derived from the root
//! manifest; the API and frontend family tests run in private target folders, and every other
//! workspace member is tested by `test workspace members`, one `cargo test -p` per package.

use super::clippy_package_sets::{native_clippy_packages, wasm32_clippy_packages};
use super::*;
use crate::wave_execution::gate_folder;
use ci_task_catalog::frontend_package_lane::{frontend_family_argv, frontend_packages};
use ci_task_catalog::wasm32_lint_lane::wasm32_clippy_argv;
use repository_layout::build_output;

/// Full gate — runs once per wave on merged main.
///
/// Takes the wave's BASE commit (the SHA main was at before this wave's merges). Two things depend
/// on it, and getting it wrong is silent:
///   * the frontend check. It used to diff `HEAD~1..HEAD`, which after landing N slices sees only
///     the LAST merge — so a frontend-touching slice merged first, followed by a backend slice,
///     skipped the trunk build entirely and a frontend regression landed green.
///   * anything else that needs to reason about "what this wave changed".
///
/// With no base argument the base is DERIVED from the last wave-close commit and then VERIFIED to
/// cover the whole wave; an explicit base is verified the same way. There is no `HEAD~1` fallback
/// — see [`super::super::base`] for why derive-and-verify rather than a mandatory argument.
pub(crate) fn cmd_gate(ctx: &Ctx, base_arg: &str) -> u8 {
    let mut base = base_arg.to_string();
    if base.is_empty() {
        let Some(derived) = base::prev_wave_close() else {
            wprintln!("gate: no base given, and no 'wave N CLOSED' commit is reachable from HEAD.");
            wprintln!(
                "        There is nothing to derive the wave's base from, and HEAD~1 is not a safe"
            );
            wprintln!(
                "        guess — it is the exact default that reported PASS 26/26 over four unexamined"
            );
            wprintln!("        frontend slices in wave 75. Pass the base explicitly:");
            wprintln!("        cargo xtask platform wave gate <sha main was at before this wave>");
            return 2;
        };
        base = derived;
        wprintln!(
            "gate: no base given — derived {} from the last wave-close commit",
            super::super::short(&base)
        );
        wprintln!("        {}", super::super::subject(&base));
    }
    // A base git cannot resolve makes EVERY change-scoped step below diff against nothing:
    // touch_changed, wasm_changed, fmt_changed and the trunk build each see an empty file list and
    // print PASS/SKIP without examining a single line. That is this program's signature defect —
    // a tool reporting success over an input it never looked at — living inside the gate runner.
    //
    // A ticket id where a rev belongs is the common way this happens: `git rev-parse` fails on
    // it, so `<id>..HEAD` resolves to nothing, and the gate reports `wasm32 (frontend) PASS` plus
    // `trunk build SKIP (frontend untouched)` on a slice that changed only frontend Rust.
    //
    // Refuse instead. An unresolvable base is never a thing you meant.
    if super::super::git_stdout(&[
        "rev-parse",
        "--verify",
        "--quiet",
        &format!("{base}^{{commit}}"),
    ])
    .filter(|s| !s.is_empty())
    .is_none()
    {
        if base.starts_with("T-")
            && base[2..]
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        {
            wprintln!(
                "gate: '{base}' is a ticket id, not a git base — the per-slice gate is a different command."
            );
            wprintln!("        per-slice:  cargo xtask platform wave gate --slice {base}");
            wprintln!(
                "        wave gate:  cargo xtask platform wave gate [<base>]   (derived when omitted)"
            );
        } else {
            wprintln!("gate: base '{base}' is not a resolvable commit — refusing to run.");
            wprintln!(
                "        Every change-scoped step would diff against nothing and PASS without looking."
            );
        }
        return 2;
    }
    // Resolvable is not the same as CORRECT. `gate HEAD~1` resolves, is an ancestor of HEAD, and
    // contains changed files — it clears both the check above and refuse_empty_range below, while
    // shrinking the gate's range to a single merge.
    if base::gate_base_covers_wave(ctx, &base) != 0 {
        return 2;
    }
    // Resolving is not the same as containing anything — `gate HEAD` cleared the check above and
    // still gated an empty range.
    let range = format!("{base}..HEAD");
    if base::refuse_empty_range(
        &range,
        "Pick a base that actually precedes the work — e.g. the commit before this wave opened.",
    ) != 0
    {
        return 2;
    }
    // Serialise against every other gate on this machine. The wave gate is the one that runs the
    // test steps and the trunk build, so it is the one with the most shared mutable state to lose:
    // three private-per-step target dirs that are shared per WORKTREE, one gate database, and one
    // gate dist. Taken BEFORE touch_changed — the fingerprint invalidation and the steps that
    // depend on it have to be inside the same critical section or the invalidation means nothing.
    let base12: String = base.chars().take(12).collect();
    let mut state = GateState::new();
    match state.take(ctx, &format!("wave gate {base12}")) {
        0 => {}
        n => return n,
    }
    wprintln!("═══ platform wave gate (base {base12}) ═══");

    let mut r = Runner {
        fail: false,
        timeout_arm: Some(ctx.gate_timeout),
    };
    // rc honoured, not discarded — see the same call in gate_slice.
    if touch::touch_changed(&range) != 0 {
        r.fail = true;
    }
    // Same placement and same reason as in gate_slice — inside the lock, ahead of every cargo
    // step. It matters most here: a wave range touches a few crates, so every OTHER workspace
    // member's `cargo check` and `clippy` verdict would otherwise rest on artifacts this driver
    // cannot attribute to a tree.
    if touch::touch_workspace(ctx) != 0 {
        r.fail = true;
    }

    r.run("cargo check", || {
        checkrun(ctx, &["cargo", "check", "--workspace", "--quiet"])
    });
    r.run("wasm32 (frontend)", || changed::wasm_changed(ctx, &range));
    r.run("fmt (changed)", || changed::fmt_changed(ctx, &range));
    // Clippy is scoped per lane, NOT --workspace: each lane is linted with the flags its ci.yml
    // job uses (host target with every target, wasm32, the frontend job on wasm32 and natively),
    // every one with `-D warnings`. The lanes partition the workspace members, derived from the
    // root manifest (`gate/clippy_package_sets.rs`), so every member a wave can change — the
    // applications, the API crates and every `crates/**` library — is linted by one of them.
    r.run("clippy apps and crates", || {
        match native_clippy_packages(&ctx.root) {
            Ok(packages) => checkrun(ctx, &native_clippy_argv(&packages)),
            Err(error) => {
                wprintln!("    {error}");
                1
            }
        }
    });
    r.run("clippy wasm32 members", || {
        match wasm32_clippy_packages(&ctx.root) {
            Ok(packages) => {
                let argv = wasm32_clippy_argv(&packages);
                let words: Vec<&str> = argv.iter().map(String::as_str).collect();
                checkrun(ctx, &words)
            }
            Err(error) => {
                wprintln!("    {error}");
                1
            }
        }
    });
    // The frontend family (the app and every crate under `crates/frontend`, derived from the
    // workspace) is linted twice, both with `-D warnings`, as `ci-local-leptos` and clippy_changed
    // do: for wasm32 (the browser build) and natively (the native test build), since each
    // compiles a different `cfg(target_arch)` half. --all-targets is load-bearing for
    // `#[cfg(test)]` code and benches.
    r.run("clippy frontend", || {
        frontend_family_step(
            ctx,
            checkrun,
            &["cargo", "clippy"],
            &[
                "--target",
                "wasm32-unknown-unknown",
                "--all-targets",
                "--quiet",
                "--",
                "-D",
                "warnings",
            ],
        )
    });
    r.run("clippy frontend (native)", || {
        frontend_family_step(
            ctx,
            checkrun,
            &["cargo", "clippy"],
            &[
                "--all-targets",
                "--locked",
                "--quiet",
                "--",
                "-D",
                "warnings",
            ],
        )
    });
    // ensure_gate_db + the skip count check are what stop `test api` passing vacuously. A suite that
    // reports "ok" while every DB test printed `skip:` is worse than a red one: it is a green one.
    // rc honoured: ensure_gate_db refuses to prune without the gate lock, and a gate that could not
    // prepare its database must not go on to interpret the result. NOT wrapped in run() — the bash
    // called it bare, so its output is not captured or indented.
    if db::ensure_gate_db(ctx, &state) != 0 {
        r.fail = true;
    }
    // Adjacent to migrate DB prep: Class-R pins migration 0016's claim UPDATE body on disk.
    r.run("db_migrate claim body", || {
        migrate::gate_db_migrate_claim_body(ctx)
    });
    // ADVANCE mode — the wave gate is the only caller allowed to move the persist DB
    // forward, because only merged main is history that will not be abandoned. Deliberately placed
    // AFTER ensure_gate_db (which owns the throwaway forward-from-empty DB) and BEFORE `test api`.
    r.run("db_migrate persist", || {
        migrate::gate_db_migrate_persist(ctx, &state, "advance") as i32
    });
    r.run("test api", || db::gate_test_api(ctx));
    // Frontend tests get a PRIVATE target dir. With a shared CARGO_TARGET_DIR,
    // `cargo test -p frontend` runs a stale `frontend-<hash>` test binary built by
    // ANOTHER worktree, reporting that worktree's test count. Same package name + version across
    // worktrees = same artifact hash = clobbering. The step tests the whole frontend family.
    let frontend_dir = format!(
        "CARGO_TARGET_DIR={}",
        gate_folder(&ctx.main_root, build_output::GATE_FRONTEND_SUBFOLDER)
    );
    r.run("test frontend", || {
        frontend_family_step(
            ctx,
            hostrun,
            &["env", &frontend_dir, "cargo", "test"],
            &["--quiet"],
        )
    });
    // Every workspace member the three test steps above do not run, derived from the root
    // manifest: a member the workspace gains is tested here from the moment the manifest names
    // it, never only once someone extends a list. One `cargo test -p` per package, so features
    // never unify across packages.
    // PRIVATE TARGET DIR, same reason and not negotiable: this step BUILDS AND RUNS test binaries.
    let tools_dir = format!(
        "CARGO_TARGET_DIR={}",
        gate_folder(&ctx.main_root, build_output::GATE_TOOLS_SUBFOLDER)
    );
    r.run("test workspace members", || {
        test_workspace_members(ctx, &tools_dir)
    });
    // The linting half of the same gap: every tool crate a wave can touch (the workspace members
    // under `tools/`, xtask and developer_tools among them), derived from the root manifest.
    // `checkrun`, not `hostrun`: this is a check-class step and shares the check-class exposure.
    r.run("clippy xtask+developer_tools", || {
        let packages = match tool_clippy_packages(&ctx.root) {
            Ok(packages) => packages,
            Err(error) => {
                wprintln!("    {error}");
                return 1;
            }
        };
        checkrun(ctx, &native_clippy_argv(&packages))
    });
    // The Leptos build is the single most expensive gate (2-6 min warm). Wave-level only, and only
    // when the wave actually touched the frontend — measured across the WHOLE wave, not the last
    // merge. NOTE: committed diff only, no working-tree union.
    // The scope is the frontend crate AND every workspace crate it compiles in, derived from the
    // dependency graph — see `changed::wasm_scope_prefixes`. A change confined to an engine crate
    // the SPA links still changes what the SPA compiles, so the crate list cannot be frontend-only.
    let wave_diff = git_stdout_lossy(&["diff", "--name-only", &range]);
    if changed::wasm_scope_touched(&ctx.root, wave_diff.lines()) {
        r.run("trunk build", || trunk::gate_trunk_build(ctx));
    } else {
        wprintln!(
            "  {:<28} SKIP (nothing the SPA compiles changed this wave: {})",
            "trunk build",
            changed::wasm_scope_prefixes(&ctx.root).join(" ")
        );
    }
    // Placed next to `ticket registry` rather than up with the compile steps because the two are
    // the gate's repo-artifact validators. Unconditional, never behind the frontend `if`: a
    // backend-only schema change would skip a conditional step.
    r.run("schema", || schema::gate_schema(ctx));
    // Cold-path twin of the gate_slice step: a step wired into only one half drifts green.
    r.run("catalogue drift", || {
        checkrun(
            ctx,
            &[
                "cargo",
                "run",
                "-q",
                "-p",
                "developer_tools",
                "--bin",
                "world",
                "--",
                "reclassify",
                "--terrain",
                "everon",
            ],
        )
    });
    r.run("ticket registry", || {
        checkrun(
            ctx,
            &["cargo", "run", "-q", "-p", "xtask", "--", "ticket", "check"],
        )
    });
    // The committed wave.lock must match the tickets. `ticket check` above already embeds this,
    // but the explicit step survives refactors of either side — a plan the gate never validates
    // drifts from the tickets it claims to describe.
    r.run("wave lock", || {
        checkrun(
            ctx,
            &["cargo", "run", "-q", "-p", "xtask", "--", "wave", "check"],
        )
    });
    for (label, name) in VERIFY_STEPS {
        r.run(label, || {
            checkrun(
                ctx,
                &["cargo", "run", "-q", "-p", "xtask", "--", "verify", name],
            )
        });
    }
    // THE LANGUAGE GATES, AND WHY THEY ARE HERE RATHER THAN ONLY IN ci.yml. A gate wired only
    // into a composite this driver deliberately does not run is in no path that runs, and can be
    // red for waves while the gate prints PASS.
    //
    // `verify no-python` and `verify no-shell` run the same TrackedLanguageBan table (hard zero),
    // so they cannot disagree; both CLI names stay because CI job names use them. xtask is already
    // built by `test workspace members` above.
    r.run("no-python", || {
        checkrun(
            ctx,
            &[
                "cargo",
                "run",
                "-q",
                "-p",
                "xtask",
                "--",
                "verify",
                "no-python",
            ],
        )
    });
    r.run("no-node", || {
        hostrun(
            ctx,
            &[
                "cargo", "run", "-q", "-p", "xtask", "--", "verify", "no-node",
            ],
        )
    });
    r.run("no-shell", || {
        hostrun(
            ctx,
            &[
                "cargo", "run", "-q", "-p", "xtask", "--", "verify", "no-shell",
            ],
        )
    });
    r.run("ci-shell", || {
        hostrun(
            ctx,
            &[
                "cargo", "run", "-q", "-p", "xtask", "--", "verify", "ci-shell",
            ],
        )
    });

    wprintln!();
    if r.fail {
        state.verdict("FAIL", "GATE");
        return 1;
    }
    state.verdict("PASS", "GATE");
    0
}

/// The package `test api` tests against the gate database, with every API crate, which the step
/// derives.
pub(super) const WAVE_GATE_API_TEST_PACKAGE: &str = "api";

/// The members whose tests a step of [`cmd_gate`] other than `test workspace members` runs:
/// [`WAVE_GATE_API_TEST_PACKAGE`] and the frontend family `test frontend` tests, derived from the
/// workspace under `root`.
///
/// # Errors
/// The frontend family cannot be derived.
pub(super) fn wave_gate_dedicated_test_packages(
    root: &std::path::Path,
) -> ci_task_catalog::Result<Vec<String>> {
    let mut packages = vec![WAVE_GATE_API_TEST_PACKAGE.to_string()];
    packages.extend(frontend_packages(root)?);
    Ok(packages)
}

/// Derives `leading -p <package>… trailing` over the frontend family and runs it through `run`
/// (`checkrun` or `hostrun`); red when the family cannot be derived, never a step over fewer
/// packages.
fn frontend_family_step(
    ctx: &Ctx,
    run: fn(&Ctx, &[&str]) -> i32,
    leading: &[&str],
    trailing: &[&str],
) -> i32 {
    match frontend_family_argv(&ctx.root, leading, trailing) {
        Ok(argv) => {
            let words: Vec<&str> = argv.iter().map(String::as_str).collect();
            run(ctx, &words)
        }
        Err(error) => {
            wprintln!("    {error}");
            1
        }
    }
}

/// `test workspace members`: `cargo test -p <package>` in `target_dir_assignment`'s private target
/// directory for every workspace member outside [`wave_gate_dedicated_test_packages`] and the API
/// family `test api` covers, each its own run; the first red package's code, after every package ran. A workspace whose members
/// cannot be derived is red, never an empty step.
fn test_workspace_members(ctx: &Ctx, target_dir_assignment: &str) -> i32 {
    let dedicated = match wave_gate_dedicated_test_packages(&ctx.root) {
        Ok(dedicated) => dedicated,
        Err(error) => {
            wprintln!("    {error:#}");
            return 1;
        }
    };
    let dedicated: Vec<&str> = dedicated.iter().map(String::as_str).collect();
    let packages = match ci_task_catalog::workspace_member_tests::member_packages_outside_api_family(
        &ctx.root, &dedicated,
    ) {
        Ok(packages) => packages,
        Err(error) => {
            wprintln!("    {error:#}");
            return 1;
        }
    };
    let mut first_red = 0;
    for package in &packages {
        let rc = hostrun(
            ctx,
            &[
                "env",
                target_dir_assignment,
                "CARGO_INCREMENTAL=0",
                "cargo",
                "test",
                "-p",
                package,
                "--quiet",
            ],
        );
        if rc != 0 && first_red == 0 {
            first_red = rc;
        }
    }
    first_red
}
