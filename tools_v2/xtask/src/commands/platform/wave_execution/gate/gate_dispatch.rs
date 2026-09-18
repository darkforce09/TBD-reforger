use super::*;

/// Full gate — runs once per wave on merged main.
///
/// Takes the wave's BASE commit (the SHA main was at before this wave's merges). Two things depend
/// on it, and getting it wrong is silent:
///   * the frontend check. It used to diff `HEAD~1..HEAD`, which after landing N slices sees only
///     the LAST merge — so a frontend-touching slice merged first, followed by a backend slice,
///     skipped the trunk build entirely and a frontend regression landed green.
///   * anything else that needs to reason about "what this wave changed".
///
/// T-602: with no base argument it is DERIVED from the last wave-close commit and then VERIFIED to
/// cover the whole wave; an explicit base is verified the same way. It no longer falls back to
/// `HEAD~1` — see [`super::super::base`] for the wave-75 incident that default caused, the wave-76
/// reproduction, and why derive-and-verify rather than a mandatory argument.
pub fn cmd_gate(ctx: &Ctx, base_arg: &str) -> u8 {
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
    // OBSERVED 2026-07-26 (found by T-394's slice agent, fixed here): the command center's own
    // slice briefs said `wave.sh gate T-394`, putting a ticket id where a rev belongs. `git
    // rev-parse T-394` fails, so `T-394..HEAD` resolved to nothing and the gate reported `wasm32
    // (frontend) PASS` plus `trunk build SKIP (frontend untouched)` on a slice that changed ONLY
    // frontend Rust. Verdict: GATE: PASS. Three slices in that wave ran this way.
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
    // T-602. Resolvable is not the same as CORRECT. `gate HEAD~1` resolves, is an ancestor of HEAD,
    // and contains changed files — it clears both the check above and refuse_empty_range below, and
    // it is exactly what shrank wave 75's gate to one merge.
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
    // T-421, same placement and same reason as in gate_slice — inside the lock, ahead of every
    // cargo step. This is the one that mattered most here: wave 5's range touched three crates, so
    // every OTHER workspace member's `cargo check` and `clippy` verdict rested on artifacts nothing
    // in this file could attribute to a tree.
    if touch::touch_workspace(ctx) != 0 {
        r.fail = true;
    }

    r.run("cargo check", || {
        checkrun(ctx, &["cargo", "check", "--workspace", "--quiet"])
    });
    r.run("wasm32 (frontend)", || changed::wasm_changed(ctx, &range));
    r.run("fmt (changed)", || changed::fmt_changed(ctx, &range));
    // Clippy is scoped per-crate, NOT --workspace.
    //
    // `cargo clippy --workspace --all-targets -- -D warnings` is still red on clean main, so a
    // workspace-wide gate would be red before a single slice merged and nothing could ever land.
    //
    // T-603 CORRECTION — THE REASON MOVED, AND THE NOTE HAD NOT. This used to read "~45 errors,
    // almost all in tools_v2/developer-tools and xtask, which have never been clippy-gated". MEASURED
    // 2026-07-31, that attribution is now exactly backwards: 60 errors in the bin target (61 with
    // --all-targets), ALL SIXTY in `website-frontend` linted natively, and ZERO in tools_v2/developer-tools
    // or xtask — those two are clean and are gated by the `clippy xtask+tbd-tools` step below.
    //
    // ci.yml gates per-crate (:59 website-api, :91 map-engine, :112 website-frontend on wasm32) and
    // the three steps here mirror it; the fourth (below) covers what ci.yml has no job for at all.
    r.run("clippy api", || {
        checkrun(
            ctx,
            &[
                "cargo",
                "clippy",
                "-p",
                "website-api",
                "--all-targets",
                "--quiet",
                "--",
                "-D",
                "warnings",
            ],
        )
    });
    // --all-features is the floor: without it clippy compiles none of the feature-gated modules
    // and passes on code it never read. Measured blind on flatten.rs. Since T-0xx Phase 2A that
    // cuts deeper — `lib.rs` gates every module and the default is `scenario` alone, so a bare
    // clippy would read the mission compiler and nothing else. Gate test step matches
    // (T-747 / wave139 F2).
    r.run("clippy map-engine", || {
        checkrun(
            ctx,
            &[
                "cargo",
                "clippy",
                "-p",
                "website-map-engine",
                "--all-features",
                "--all-targets",
                "--quiet",
                "--",
                "-D",
                "warnings",
            ],
        )
    });
    // NOTE: no `-D warnings` here, deliberately — ci.yml website-frontend clippy runs WITHOUT it,
    // so warnings are advisory upstream. Adding -D here would make the gate stricter than CI and
    // red on arrival. T-742 adds --all-targets (load-bearing for #[cfg(test)] / benches) so this
    // step and clippy_changed stay aligned with T-752's Makefile/ci-local-leptos fix; -D stays off
    // to match CI.
    r.run("clippy frontend", || {
        checkrun(
            ctx,
            &[
                "cargo",
                "clippy",
                "-p",
                "website-frontend",
                "--target",
                "wasm32-unknown-unknown",
                "--all-targets",
                "--quiet",
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
    // T-515. Adjacent to migrate DB prep: Class-R pins 0016 claim UPDATE body on disk.
    r.run("db_migrate claim body", || {
        migrate::gate_db_migrate_claim_body(ctx)
    });
    // T-555. ADVANCE mode — the wave gate is the only caller allowed to move the persist DB
    // forward, because only merged main is history that will not be abandoned. Deliberately placed
    // AFTER ensure_gate_db (which owns the throwaway forward-from-empty DB) and BEFORE `test api`.
    r.run("db_migrate persist", || {
        migrate::gate_db_migrate_persist(ctx, &state, "advance") as i32
    });
    r.run("test api", || db::gate_test_api(ctx));
    // --all-features is REQUIRED (T-747 / wave139 F2). Bare `cargo test -p website-map-engine` is
    // a vacuous pass and the merged tripwire REDs on it. Measured 2026-08-08 on the two crates that
    // existed then: mission-core bare 140, `doc,mission` 502, --all-features 635. T-0xx Phase 2A
    // folded them into one crate and flipped the default to `scenario` alone, which makes a bare
    // run emptier still: re-measured after the fold, --all-features is 1174 lib + 19 integration +
    // 3 doc. `ci-local` and this gate must match. Private target dir for the same reason as
    // `test api` and `test frontend`: this step RUNS test binaries.
    let mapengine_dir = format!(
        "CARGO_TARGET_DIR={}",
        ctx.main_root.join("target-gate-mapengine").display()
    );
    r.run("test map-engine", || {
        hostrun(
            ctx,
            &[
                "env",
                &mapengine_dir,
                "CARGO_INCREMENTAL=0",
                "cargo",
                "test",
                "-p",
                "website-map-engine",
                "--all-features",
                "--quiet",
            ],
        )
    });
    // Frontend tests get a PRIVATE target dir. Two agents (T-193, T-195) independently proved that
    // with the shared CARGO_TARGET_DIR, `cargo test -p website-frontend` runs a stale
    // website_frontend-<hash> test binary built from ANOTHER worktree: T-193 saw 113 passing from a
    // binary lacking its new tests; T-195 hit it twice and had to use a private dir to get true
    // numbers. Same package name + version across worktrees = same artifact hash = clobbering.
    let frontend_dir = format!(
        "CARGO_TARGET_DIR={}",
        ctx.main_root.join("target-gate-frontend").display()
    );
    r.run("test frontend", || {
        hostrun(
            ctx,
            &[
                "env",
                &frontend_dir,
                "cargo",
                "test",
                "-p",
                "website-frontend",
                "--quiet",
            ],
        )
    });
    // T-597 — THE STRUCTURAL GAP. `xtask` and `tools_v2/developer-tools` were tested by NOTHING. The gate ran
    // `test api`, `test map-engine`, `test frontend` and stopped. MEASURED 2026-07-31: ci.yml's
    // `test` step is a bare `cargo test` under the website-api job, whose
    // `defaults.run.working-directory` is `apps/website/api`. Cargo with no `-p` selects the package
    // in the CWD, so both are workspace members that no gate and no workflow has ever run. What that
    // cost: density::tests::corner_partition_identity sat red from T-176 to T-597 — four weeks.
    // PRIVATE TARGET DIR, same reason and not negotiable: this step BUILDS AND RUNS test binaries.
    let tools_dir = format!(
        "CARGO_TARGET_DIR={}",
        ctx.main_root.join("target-gate-tools").display()
    );
    r.run("test xtask+tbd-tools", || {
        hostrun(
            ctx,
            &[
                "env",
                &tools_dir,
                "CARGO_INCREMENTAL=0",
                "cargo",
                "test",
                "-p",
                "xtask",
                "-p",
                "developer-tools",
                "--quiet",
            ],
        )
    });
    // T-603 — THE OTHER HALF OF T-597's GAP. Nothing LINTED them either. 14 errors on clean main
    // under `-D warnings` — 10 in tools_v2/developer-tools and 4 in xtask, all mechanical, all older than the
    // ticket that found them, fixed in the same commit that added this step because a gate step that
    // is red the moment it lands teaches the next agent that gate failures are noise.
    // `checkrun`, not `hostrun`: this is a check-class step and carries the T-421 exposure verbatim.
    r.run("clippy xtask+tbd-tools", || {
        checkrun(
            ctx,
            &[
                "cargo",
                "clippy",
                "-p",
                "xtask",
                "-p",
                "developer-tools",
                "--all-targets",
                "--quiet",
                "--",
                "-D",
                "warnings",
            ],
        )
    });
    // The Leptos build is the single most expensive gate (2-6 min warm). Wave-level only, and only
    // when the wave actually touched the frontend — measured across the WHOLE wave, not the last
    // merge. NOTE: committed diff only, no working-tree union; that is what the bash asked.
    // T-946: the scope is the frontend crate AND every workspace crate it compiles in, derived
    // from the dependency graph — see `changed::wasm_scope_prefixes`. Wave 237 rewrote
    // the TBDD decode (then `website-mission-core`, now `map-engine/src/data`), which the SPA
    // links, and this step skipped.
    let wave_diff = git_stdout_lossy(&["diff", "--name-only", &range]);
    if changed::wasm_scope_touched(&ctx.root, wave_diff.lines()) {
        r.run("trunk build", || trunk::gate_trunk_build(ctx));
    } else {
        wprintln!(
            "  {:<24} SKIP (nothing the SPA compiles changed this wave: {})",
            "trunk build",
            changed::wasm_scope_prefixes(&ctx.root).join(" ")
        );
    }
    // T-420. Placed next to `ticket registry` rather than up with the compile steps because the two
    // are the gate's repo-artifact validators. Unconditional, never behind the frontend `if`:
    // wave 4's schema change was backend-only and would have skipped a conditional step.
    r.run("schema", || schema::gate_schema(ctx));
    // T-583/T-594 — cold-path twin of the gate_slice step. Per T-556, a step wired into only one
    // half drifts green.
    r.run("T-278 catalogue drift", || {
        checkrun(
            ctx,
            &[
                "cargo",
                "run",
                "-q",
                "-p",
                "developer-tools",
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
    // T-912.2: the committed wave.lock must match the tickets. `ticket check` above already
    // embeds this, but the explicit step survives refactors of either side — a plan the gate
    // never validates is the TSV-era drift class all over again.
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
    // T-620/T-621/T-904 — THE LANGUAGE GATES, AND WHY THEY ARE HERE RATHER THAN ONLY IN ci.yml.
    //
    // `verify-no-python` existed since T-162 and was wired into one Makefile target and `make
    // ci-local` — which this file's own header explains is deliberately NOT used by the gate. It was
    // therefore in NO path that runs: not ci.yml (measured, zero hits), not this gate. Meanwhile it
    // was RED, on scripts/{platform,mod}/slice-collisions.py, from the day the factory opened. Four
    // waves of "GATE PASS 28/28" were printed over a hard gate that was failing the whole time and
    // that nothing invoked. That is the exact shape T-556 and T-478 keep finding, at gate scope.
    //
    // T-904: both `verify no-python` and `verify no-shell` run the same TrackedLanguageBan table
    // (hard zero; inventories deleted). Both CLI names stay so CI job names do not break; they
    // cannot disagree. xtask is already built by `test xtask+tbd-tools` above.
    r.run("no-python (T-620)", || {
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
    r.run("no-node (T-165.10)", || {
        hostrun(
            ctx,
            &[
                "cargo", "run", "-q", "-p", "xtask", "--", "verify", "no-node",
            ],
        )
    });
    r.run("no-shell (T-621)", || {
        hostrun(
            ctx,
            &[
                "cargo", "run", "-q", "-p", "xtask", "--", "verify", "no-shell",
            ],
        )
    });
    r.run("ci-shell (T-901)", || {
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
