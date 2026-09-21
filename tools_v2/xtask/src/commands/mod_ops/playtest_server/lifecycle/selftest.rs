use super::*;

/// `--selftest` — prove the kill path can FAIL, and cannot lie.
///
/// A gate nobody has watched fail is not a gate. This one
/// exists because the defect is invisible on every passing run: `kill_run` only lies when the
/// bridge flaked, which no green boot ever exercises. So the lie is reproduced here on purpose.
/// Boots no game server; spawns disposable `sleep` groups on the host and kills them.
pub fn selftest(host: &Host) -> u8 {
    println!(
        "==> run-playtest-server selftest (kill discipline must be unable to claim a false death)"
    );
    if !host.require_host() {
        return super::super::env_fail(
            "no host bridge — the selftest exercises the real bridge, so it needs one",
            "",
        );
    }

    let mut t = Tally { rc: 0 };
    let tmp = match mktemp_dir() {
        Some(d) => d,
        None => return super::super::env_fail("could not create a selftest temp dir", ""),
    };
    // The selftest drives kill_run against ITS OWN pidfile, never the real run dir's.
    let paths = RunPaths {
        run_dir: tmp.clone(),
        pidfile: format!("{tmp}/server.pid"),
        srv_out: format!("{tmp}/server.out"),
        lockdir: format!("{tmp}/.run.lock"),
    };
    // Only `game_port` / `a2s_port` are read by the messages under test.
    let o = Opts::defaults("/nonexistent");

    // S1 — THE REGRESSION. A live group plus a broken bridge must never be called dead.
    println!("  -- S1: live group + broken host bridge");
    let pg = st_spawn(host, "sleep 120");
    if pg.is_empty() {
        t.fail("S1 could not spawn a test group on the host");
    } else {
        let _ = std::fs::write(&paths.pidfile, format!("{pg}\n"));
        // bash overrode `hostrun() { return 127; }` in a subshell so the real bridge was untouched
        // afterwards; `Host::broken` is that override with a type. rc and the stray pgid come back
        // as values instead of being scraped out of a subshell's stdout.
        let broken = host.broken();
        let (rc, stray) = match kill_run_inner(&paths, &broken, Volume::Quiet) {
            Ok(()) => (0u8, String::new()),
            Err(p) => (1u8, p),
        };
        let st_out = format!("rc={rc} stray={stray}");
        t.check(
            rc != 0,
            &format!("S1 kill_run refused to claim success ({st_out})"),
            "S1 kill_run returned SUCCESS with the group alive",
        );
        t.check(
            stray == pg,
            &format!("S1 named the stray process group ({pg}) instead of exiting quietly"),
            &format!("S1 did not record the stray pgid: {st_out}"),
        );
        t.check(
            Path::new(&paths.pidfile).is_file(),
            "S1 kept the pidfile (it is the only handle on a live group)",
            "S1 deleted the pidfile of a process it never confirmed dead",
        );
        t.check(
            probe_group(host, &pg) == Probe::Alive,
            "S1 the group really was alive throughout",
            "S1 test group died on its own — the case did not exercise anything",
        );
        host.signal_quietly(&["kill", "-9", "--", &format!("-{pg}")]);
    }

    // S2 — TERM ignored, exactly as the engine ignores it during world load. Must escalate to KILL,
    // confirm the death, and only then drop the pidfile.
    println!("  -- S2: group that ignores SIGTERM (models the engine during world load)");
    let pg = st_spawn(host, r#"trap "" TERM; sleep 120"#);
    if pg.is_empty() {
        t.fail("S2 could not spawn a test group on the host");
    } else {
        let _ = std::fs::write(&paths.pidfile, format!("{pg}\n"));
        let krc = match kill_run_inner(&paths, host, Volume::Quiet) {
            Ok(()) => 0,
            Err(_) => 1,
        };
        t.check(
            krc == 0,
            "S2 kill_run escalated past the ignored TERM and returned 0",
            &format!("S2 kill_run returned {krc} against a killable group"),
        );
        t.check(
            probe_group(host, &pg) == Probe::Dead,
            "S2 the group is CONFIRMED gone, not assumed gone",
            "S2 returned success while the group still answers",
        );
        t.check(
            !Path::new(&paths.pidfile).is_file(),
            "S2 removed the pidfile only after confirming death",
            "S2 left a pidfile behind for a confirmed-dead group",
        );
        host.signal_quietly(&["kill", "-9", "--", &format!("-{pg}")]);
    }

    // S3 — the ordinary case still works, and no pidfile is not an error.
    println!("  -- S3: cooperative group, and the empty case");
    let pg = st_spawn(host, "sleep 120");
    if pg.is_empty() {
        t.fail("S3 could not spawn a test group on the host");
    } else {
        let _ = std::fs::write(&paths.pidfile, format!("{pg}\n"));
        let krc = match kill_run_inner(&paths, host, Volume::Quiet) {
            Ok(()) => 0,
            Err(_) => 1,
        };
        let state = probe_group(host, &pg);
        t.check(
            krc == 0 && state == Probe::Dead,
            "S3 TERM path confirmed the death and returned 0",
            &format!(
                "S3 cooperative kill did not confirm (rc={krc} state={})",
                state.word()
            ),
        );
        host.signal_quietly(&["kill", "-9", "--", &format!("-{pg}")]);
    }
    let _ = std::fs::remove_file(&paths.pidfile);
    t.check(
        kill_run_inner(&paths, host, Volume::Quiet).is_ok(),
        "S3 no pidfile is rc 0, not an invented failure",
        "S3 no pidfile should be rc 0",
    );

    // S4 — refuse-if-running. The pidfile names a live group; staging must not proceed.
    println!("  -- S4: assert_no_live_server refuses, and leaves the pidfile alone");
    let pg = st_spawn(host, "sleep 120");
    if pg.is_empty() {
        t.fail("S4 could not spawn a test group on the host");
    } else {
        let _ = std::fs::write(&paths.pidfile, format!("{pg}\n"));
        let (krc, text) = match check_no_live_server(&paths, host, &o) {
            LiveVerdict::Clear => (0u8, String::new()),
            LiveVerdict::Refuse { code, message } => (code, message.join("\n")),
        };
        t.check(
            krc != 0,
            &format!("S4 refused to stage over a live server (rc {krc})"),
            "S4 allowed staging over a live server",
        );
        t.check(
            text.contains("STILL RUNNING") && text.contains(&pg),
            "S4 named the running process group",
            &format!("S4 refusal did not name the group: {text}"),
        );
        t.check(
            Path::new(&paths.pidfile).is_file(),
            "S4 left the first run's pidfile untouched",
            "S4 destroyed the first run's pidfile — the F5 orphan bug",
        );
        host.signal_quietly(&["kill", "-9", "--", &format!("-{pg}")]);
        sleep(Duration::from_millis(500));
        let (krc, text) = match check_no_live_server(&paths, host, &o) {
            LiveVerdict::Clear => (0u8, String::new()),
            LiveVerdict::Refuse { code, message } => (code, message.join("\n")),
        };
        t.check(
            krc == 0,
            "S4 allows staging once the group is confirmed dead",
            &format!("S4 still refuses after the group died: {text}"),
        );
        t.check(
            !Path::new(&paths.pidfile).is_file(),
            "S4 cleared the pidfile only after confirming death",
            "S4 kept a pidfile for a confirmed-dead group",
        );
    }

    let _ = std::fs::remove_dir_all(&tmp);
    println!();
    if t.rc == 0 {
        println!("SELFTEST: PASS");
    } else {
        println!("SELFTEST: FAIL");
    }
    t.rc
}

/// bash `mktemp -d "${TMPDIR:-/tmp}/tbd-rps-selftest.XXXXXX"`.
///
/// `create_dir` is the atomicity `mktemp -d` provides: the first name that does not already exist
/// wins, and losing the race means trying again rather than sharing a directory.
pub(super) fn mktemp_dir() -> Option<String> {
    let base = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let pid = std::process::id();
    for n in 0..64 {
        let cand = format!("{base}/tbd-rps-selftest.{pid}{n:02}");
        if std::fs::create_dir(&cand).is_ok() {
            return Some(cand);
        }
    }
    None
}
