//! The preflight check sequence.
//!
//! **Role:** `run` executes every preflight check in its fixed order, prints each result line and
//! the closing summary, and returns the exit code.
//!
//! **Position:** called by the `platform preflight` dispatch through the parent module's re-export;
//! every probe it calls lives in the sibling `ok.rs`.
//!
//! **Signals & state:** one `Counters` value per run; reads the checkout, the machine and the local
//! services, and writes only to stdout.
//!
//! **Invariants:** checks never mutate the machine or the checkout; a block-class failure makes the
//! exit `1` unless `warn_only`, a warn-class one never does; the order of the lines is the order of
//! the checks, so two runs on one machine compare line by line.

use super::*;

/// The source folder of the running API: the API server crate and every API crate it assembles
/// sit under it. The newest commit touching it is the code the API process must be at least as
/// new as.
const API_SOURCE_FOLDER: &str = "crates/api";

/// The commit time (`%ct`) of the newest commit under [`API_SOURCE_FOLDER`] in `root`; 0 when
/// git answers nothing.
fn newest_api_commit_epoch(root: &Path) -> i64 {
    git_out(
        root,
        &["log", "-1", "--format=%ct", "--", API_SOURCE_FOLDER],
    )
    .and_then(|s| s.trim().parse().ok())
    .unwrap_or(0)
}

/// Entry for `xtask platform preflight [--warn]`.
pub(crate) fn run(warn_only: bool) -> Result<u8> {
    let root = resolve_root()?;
    env::set_current_dir(&root).map_err(|e| Error::file(format!("cd {}", root.display()), e))?;

    let mut c = Counters { block: 0, warn: 0 };
    println!("═══ platform factory preflight ═══");

    // 1. Host bridge
    if Path::new("/run/.containerenv").is_file() {
        if status_ok(hostrun(&["true"])) {
            ok("host bridge", "distrobox-host-exec live");
        } else {
            nope(
                &mut c,
                "host bridge",
                "in a container and distrobox-host-exec is dead — every cargo gate will fail",
            );
        }
    } else {
        ok("host bridge", "not containerised");
    }

    // 2. cargo (direct)
    {
        match capture_stdout(Run::new("cargo").arg("--version")) {
            Some(v) => ok("cargo", &v),
            None => nope(&mut c, "cargo", "cargo unusable via the bridge"),
        }
    }

    // 3. Disk
    match free_gb(&root) {
        Some(gb) if gb >= 40 => ok("disk", &format!("{gb}G free")),
        Some(gb) if gb >= 20 => soft(
            &mut c,
            "disk",
            &format!("{gb}G free — tight; make clean-targets first"),
        ),
        Some(gb) => nope(&mut c, "disk", &format!("{gb}G free — below the 20G floor")),
        None => nope(&mut c, "disk", "df failed — below the 20G floor"),
    }
    let orphan_mb = orphan_cache_mb();
    if orphan_mb > 4096 {
        soft(
            &mut c,
            "reclaimable",
            &format!(
                "{}G of build caches in /var/tmp — cargo xtask platform wave reclaim",
                orphan_mb / 1024
            ),
        );
    } else {
        ok(
            "reclaimable",
            &format!("{}G of stale build caches", orphan_mb / 1024),
        );
    }

    // 4. CARGO_TARGET_DIR + no per-worktree target/
    match env::var("CARGO_TARGET_DIR") {
        Ok(v) if !v.is_empty() => ok("CARGO_TARGET_DIR", &v),
        _ => soft(
            &mut c,
            "CARGO_TARGET_DIR",
            "unset in this shell — `cargo xtask platform wave` exports it, but a dispatcher must too",
        ),
    }
    let stray = stray_worktree_targets(&root);
    if stray == 0 {
        ok("no per-worktree target/", "");
    } else {
        nope(
            &mut c,
            "per-worktree target/",
            &format!("{stray} worktree(s) built into their own target — will exhaust disk"),
        );
    }
    // The shared cache is fine for check/test/clippy and fatal for a launched binary; this
    // is the check that says which one the run target currently holds.
    {
        let run_dir = PathBuf::from(crate::wave_execution::resolve_run_target_dir(&root));
        let head = git_out(&root, &["rev-parse", "HEAD"]).unwrap_or_default();
        let state = run_target_state(&run_dir, &head, &root);
        let (block, detail) = run_target_detail(&state, &run_dir, &head);
        if block {
            nope(&mut c, "run target", &detail);
        } else {
            ok("run target", &detail);
        }
    }

    // 5. RAM + swap
    match mem_available_mib() {
        Some(mb) if mb >= 1024 => ok("memory", &format!("{mb}MiB available")),
        Some(mb) => nope(
            &mut c,
            "memory",
            &format!("{mb}MiB — gate-env floor is 1024"),
        ),
        None => nope(&mut c, "memory", "0MiB — gate-env floor is 1024"),
    }
    if let Some(sw_used) = swap_used_pct() {
        if sw_used < 70 {
            ok("swap", &format!("{sw_used}% used"));
        } else {
            soft(
                &mut c,
                "swap",
                &format!("{sw_used}% used — OOM risk over a long run"),
            );
        }
    }

    // 6. Clean tree + synced remote
    match git_status_porcelain(&root) {
        Some(s) if s.is_empty() => ok("working tree", "clean"),
        Some(_) => nope(&mut c, "working tree", "dirty — commit or stash first"),
        None => nope(&mut c, "working tree", "dirty — commit or stash first"),
    }
    match git_out(&root, &["rev-parse", "--abbrev-ref", "HEAD"]) {
        Some(b) if b == "main" => ok("branch", "main"),
        _ => nope(&mut c, "branch", "not on main"),
    }
    let ahead =
        git_out(&root, &["rev-list", "--count", "origin/main..HEAD"]).unwrap_or_else(|| "?".into());
    if ahead == "0" {
        ok("remote", "in sync");
    } else {
        soft(&mut c, "remote", &format!("{ahead} commit(s) unpushed"));
    }

    // 7. Stale worktrees
    let wts = worktree_paths(&root);
    let wt = wts.len() as u64;
    if wt == 0 {
        ok("worktrees", "none stale");
    } else {
        soft(
            &mut c,
            "worktrees",
            &format!("{wt} left over — cargo xtask platform wave land will reuse or trip on them"),
        );
    }

    // 7b. Idle undispatched worktrees
    let idle_min: u64 = env::var("TBD_IDLE_WORKTREE_MIN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let mut idle: Vec<String> = Vec::new();
    for w in &wts {
        let t = w
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ahead_w =
            git_out(w, &["rev-list", "--count", "main..HEAD"]).unwrap_or_else(|| "0".into());
        let dirty_n = git_status_porcelain(w)
            .map(|s| s.lines().filter(|l| !l.is_empty()).count())
            .unwrap_or(0);
        let young = is_git_young(w, idle_min);
        if ahead_w == "0" && dirty_n == 0 && !young {
            idle.push(t);
        }
    }
    if idle.is_empty() {
        ok(
            "worktrees busy",
            &format!("every worktree is working or newer than {idle_min}m"),
        );
    } else {
        soft(
            &mut c,
            "idle worktrees",
            &format!(
                "nothing written in {idle_min}m+ — {} — created and never dispatched?",
                idle.join(" ")
            ),
        );
    }

    // 8. The central ticket manager answers, and the project's tickets validate.
    let ticket_manager = TicketManager::from_env();
    match ticket_manager.version() {
        Ok(version) => {
            ok(
                "ticket manager",
                &format!(
                    "ttm {} (contract v{})",
                    version.version, version.json_version
                ),
            );
            match ticket_manager.check() {
                Ok(check) if check.ok => ok(
                    "ticket check",
                    &format!("project {} valid", ticket_manager.project()),
                ),
                Ok(check) => nope(
                    &mut c,
                    "ticket check",
                    &format!(
                        "{} error(s) in project {} — `{}` lists them",
                        check.errors,
                        ticket_manager.project(),
                        ticket_manager.display_command(&["check"])
                    ),
                ),
                Err(e) => nope(&mut c, "ticket check", &crate::error::error_chain_text(&e)),
            }
        }
        Err(e) => nope(
            &mut c,
            "ticket manager",
            &format!(
                "{} — every wave and slice command will refuse",
                crate::error::error_chain_text(&e)
            ),
        ),
    }

    // 9. Wave plan: `ttm wave check` compares the stored plan with the tickets and the close
    // ledger; a project without a plan is a refusal, so an absent plan never reads as green. The
    // stored wave base must also agree with the newest standing marker in git.
    match ticket_manager.wave_check() {
        Ok(check) if check.ok => match ticket_manager.wave_show() {
            Ok(plan) => {
                let (n, w) = plan.open_counts();
                match wave_ledgers_agree(&root, &plan) {
                    Ok(()) => ok(
                        "wave plan",
                        &format!(
                            "{n} open tickets in {w} waves, matches the tickets and the marker ledger"
                        ),
                    ),
                    Err(why) => nope(&mut c, "wave plan", &why),
                }
            }
            Err(e) => nope(&mut c, "wave plan", &crate::error::error_chain_text(&e)),
        },
        Ok(check) => nope(
            &mut c,
            "wave plan",
            &format!(
                "`{}` found {} problem(s): {}",
                ticket_manager.display_command(&["wave", "check"]),
                check.findings.len(),
                check.findings.join("; ")
            ),
        ),
        Err(e) => nope(&mut c, "wave plan", &crate::error::error_chain_text(&e)),
    }

    // 10. Optional env — postgres + API freshness
    if tcp_up("127.0.0.1:5434") {
        ok("postgres :5434", "up");
    } else {
        soft(
            &mut c,
            "postgres :5434",
            "down — API integration tests will skip (cargo xtask db up on the HOST)",
        );
    }

    let api_code = curl_http_code("http://127.0.0.1:8080/healthz");
    if api_code == "200" {
        let api_pid = api_listen_pid().unwrap_or_default();
        let started = if api_pid.is_empty() {
            0
        } else {
            proc_start_epoch(&api_pid)
        };
        let newest = newest_api_commit_epoch(&root);
        if started > 0 && newest > started {
            soft(
                &mut c,
                "api :8080",
                &format!(
                    "healthy but STALE — running since {}, API code changed {}. Restart it or verifications lie.",
                    format_hhmm_epoch(started),
                    format_hhmm_epoch(newest)
                ),
            );
        } else {
            ok("api :8080", "healthz 200, binary current");
        }
    } else if !api_code.is_empty() && api_code != "000" {
        soft(
            &mut c,
            "api :8080",
            &format!("listening but /healthz returned {api_code} — wedged or mid-restart"),
        );
    } else {
        soft(
            &mut c,
            "api :8080",
            "down — editor smokes would report gate-red for an env reason",
        );
    }

    // 11b. trunk serve (informational)
    let ts = count_pgrep("trunk serve");
    if ts == 0 {
        ok("trunk serve", "not running");
    } else {
        ok(
            "trunk serve",
            &format!("{ts} running — fine; the gate builds into private dist + target"),
        );
    }

    // 11. Stray chrome
    let ch = count_pgrep("chrome-linux64/chrome");
    if ch == 0 {
        ok("chrome", "none stray");
    } else {
        soft(
            &mut c,
            "chrome",
            &format!("{ch} process(es) alive — leptos-gates will refuse"),
        );
    }

    println!();
    let mut out = io::stdout();
    if c.block > 0 {
        writeln!(
            out,
            "PREFLIGHT: {} BLOCK, {} warn — DO NOT START",
            c.block, c.warn
        )?;
        if warn_only {
            return Ok(0);
        }
        return Ok(1);
    }
    writeln!(out, "PREFLIGHT: PASS ({} warn)", c.warn)?;
    Ok(0)
}
