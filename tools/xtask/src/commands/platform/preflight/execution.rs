use super::*;

/// Entry for `xtask platform preflight [--warn]`.
pub fn run(warn_only: bool) -> Result<u8> {
    let root = resolve_root()?;
    env::set_current_dir(&root).with_context(|| format!("cd {}", root.display()))?;

    let mut c = Counters { block: 0, warn: 0 };
    println!("═══ platform factory preflight ═══");

    // 1. Host bridge
    if Path::new("/run/.containerenv").is_file() {
        if status_ok(&mut hostrun(&["true"])) {
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
        let mut cmd = Command::new("cargo");
        cmd.arg("--version");
        match capture_stdout(&mut cmd) {
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
        let run_dir =
            PathBuf::from(crate::commands::platform::wave_execution::resolve_run_target_dir(&root));
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

    // 8. ticket check
    {
        let mut cmd = Command::new("cargo");
        cmd.args(["run", "-q", "-p", "xtask", "--", "ticket", "check"])
            .current_dir(&root);
        if status_ok(&mut cmd) {
            ok("ticket check", "registry valid");
        } else {
            nope(
                &mut c,
                "ticket check",
                "registry INVALID — every wave gate will fail",
            );
        }
    }

    // 9. Wave lock.
    // `wave check` recomputes from the tickets and structurally compares; a missing lock is a
    // DidNotRun refusal inside it, so an absent plan can never read as green here.
    {
        let mut cmd = Command::new("cargo");
        cmd.args(["run", "-q", "-p", "xtask", "--", "wave", "check"])
            .current_dir(&root);
        if status_ok(&mut cmd) {
            let (n, w) = wave_lock_open_count(&root).unwrap_or((0, 0));
            ok(
                "wave lock",
                &format!("{n} open tickets in {w} waves, matches the ticket files"),
            );
        } else {
            nope(
                &mut c,
                "wave lock",
                &format!(
                    "cargo xtask wave check failed — stale or missing {}",
                    ticket_engine::repository::WAVE_LOCK
                ),
            );
        }
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
        let newest = git_out(
            &root,
            &["log", "-1", "--format=%ct", "--", "apps/website/api_v2"],
        )
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
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
