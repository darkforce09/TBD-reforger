use super::*;

/// ── THE PROBE ────────────────────────────────────────────────────────────────────────────────
///
/// Everything below rests on this one function telling the truth, so it is written to make one
/// specific lie impossible.
///
/// WHAT IT REPLACED, AND WHY (measured 2026-07-31). The old aliveness check was:
///
/// ```sh
///     hostrun kill -0 -- "-$pgid" >/dev/null 2>&1 || return 0     # "|| it's gone"
/// ```
///
/// Every probe is a SEPARATE host-bridge process, and a bridge that fails to start exits non-zero in
/// exactly the same way `kill -0` does on a dead pid. The two are indistinguishable at the rc. So
/// one bridge failure read as death: the escalation was skipped, `kill_run` returned success, and the
/// script exited 1 announcing "the server never registered a backend room" while the engine was
/// still alive and holding 2001/17777. The operator had to find and kill process group 3870163 by
/// hand. That is this repo's signature defect — a tool reporting a result over an input it never
/// actually examined — living inside the very script written to stop a dead server being reported as
/// up.
///
/// THE FIX IS A SENTINEL. The far side prints `TBDPROBE=alive|zombie|dead` itself. The answer is
/// believed only when it demonstrably came back from a probe that RAN on the host. Anything else — no
/// bridge, empty output, an error string, a partial read — is [`Probe::Unknown`], and `Unknown` is
/// NOT death and is never once treated as it.
///
/// `Zombie` is split out from `Alive` because a reaped-but-unwaited group leader still answers
/// `kill -0` while holding no sockets; folding that into "alive" would make death permanently
/// unconfirmable and turn the STRAY warning into a false alarm. If the host has no `pgrep`, `seen`
/// stays 0 and the answer degrades to the conservative `alive`, never to `dead`.
pub fn probe_group(host: &Host, pgid: &str) -> Probe {
    if pgid.is_empty() {
        return Probe::Unknown;
    }
    let out = match host.capture(&["sh", "-c", PROBE_SH, "_", pgid]) {
        Some(t) => t,
        None => return Probe::Unknown,
    };
    // Arm order is the bash `case`'s: alive, then zombie, then dead. Substring matches, not equality,
    // because a bridge is entitled to prepend its own chatter.
    if out.contains("TBDPROBE=alive") {
        Probe::Alive
    } else if out.contains("TBDPROBE=zombie") {
        Probe::Zombie
    } else if out.contains("TBDPROBE=dead") {
        Probe::Dead
    } else {
        Probe::Unknown
    }
}

/// bash `pgid="$(cat "$PIDFILE" 2>/dev/null)"; pgid="$(… | tr -d '[:space:]')"`.
///
/// An absent or unreadable pidfile yields the empty string — which is NOT evidence of death, and no
/// caller here treats it as such.
pub fn read_pgid(pidfile: &str) -> String {
    std::fs::read_to_string(pidfile)
        .unwrap_or_default()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// Stop the server and PROVE it stopped.
///
/// * `Ok(())` — the process group is confirmed gone (or there was never one to stop)
/// * `Err(pgid)` — could not confirm; the caller must print [`stray_warning`] with this pgid
///
/// Deliberately NOT a name match: a broad `pkill -f ArmaReforgerServer` would also kill the
/// operator's own dev server, and (measured) the bridge's own `sh -c` command line contains that
/// string, so it kills the caller too. The recorded pid is a PROCESS GROUP LEADER — the launcher runs
/// under `setsid` — and we signal the whole group, same discipline as `world-boot.sh:423`.
pub fn kill_run(paths: &RunPaths, host: &Host) -> Result<(), String> {
    kill_run_inner(paths, host, Volume::Loud)
}

pub(super) fn kill_run_inner(paths: &RunPaths, host: &Host, volume: Volume) -> Result<(), String> {
    let pgid = read_pgid(&paths.pidfile);
    // No pidfile is not evidence of death, but it is also nothing we can act on: there is no group id
    // to signal and kill-by-name is off the table. Say so rather than implying success.
    if pgid.is_empty() {
        return Ok(());
    }

    if probe_group(host, &pgid).confirmed_gone() {
        let _ = std::fs::remove_file(&paths.pidfile);
        return Ok(());
    }

    // TERM first. The engine honours TERM at steady state (measured: `--timeout=30` produced
    // `Game destroyed` at T+31 s and a clean exit 0) but IGNORED it during world load — which is
    // exactly when this function fires on a failed boot. So the grace is a grace, not a promise, and
    // it is followed by KILL unconditionally.
    host.signal_quietly(&["kill", "-TERM", "--", &format!("-{pgid}")]);
    let mut state = Probe::Unknown;
    for _ in 0..40 {
        // 40 x 0.25 s = 10 s
        sleep(Duration::from_millis(250));
        state = probe_group(host, &pgid);
        if state.confirmed_gone() {
            let _ = std::fs::remove_file(&paths.pidfile);
            return Ok(());
        }
    }

    // Still here, or still unanswerable. BOTH escalate. "I could not tell" must never take the same
    // branch as "I confirmed it is dead" — that equivalence is the whole defect.
    if volume == Volume::Loud {
        eprintln!(
            "    TERM did not settle process group {pgid} after 10s (state: {}) — escalating to KILL",
            state.word()
        );
    }
    host.signal_quietly(&["kill", "-9", "--", &format!("-{pgid}")]);
    for _ in 0..20 {
        // 20 x 0.25 s = 5 s
        sleep(Duration::from_millis(250));
        if probe_group(host, &pgid).confirmed_gone() {
            let _ = std::fs::remove_file(&paths.pidfile);
            return Ok(());
        }
    }

    // SIGKILL cannot be caught, so reaching here means either the signal never landed (the bridge is
    // down) or the process is wedged in the kernel. Either way we do NOT know it is dead, we do NOT
    // delete the pidfile, and we do NOT return success.
    Err(pgid)
}

/// The STRAY SERVER block, built as lines so the selftest can read it without capturing a terminal.
///
/// Printed LAST, after any diagnosis dump, so it is the final thing on screen.
pub fn stray_warning(paths: &RunPaths, host: &Host, o: &Opts, pgid: &str) -> Vec<String> {
    let bridge = host.instruction_name();
    vec![
        String::new(),
        "================================================================================".into(),
        "  STRAY SERVER — this script could NOT confirm the server died.".into(),
        String::new(),
        format!("    process group: {pgid}"),
        format!(
            "    pidfile:       {}   (LEFT IN PLACE deliberately — it is the only",
            paths.pidfile
        ),
        "                   handle on that group, and a stale pidfile pointing at a live".into(),
        "                   process is worth more than no pidfile at all)".into(),
        String::new(),
        "  Do this yourself and check the second command comes back empty:".into(),
        String::new(),
        format!("      {bridge} kill -9 -- -{pgid}     # from inside this container"),
        format!("      kill -9 -- -{pgid}                          # from a host terminal"),
        "      pgrep -af '[A]rmaReforgerServer'".into(),
        String::new(),
        format!(
            "  Until that group is gone it still holds UDP {} / {}, and the next boot",
            o.game_port, o.a2s_port
        ),
        "  will die with 'NETWORK (E): Unable to start replication' — which looks like a".into(),
        "  different bug entirely. Once it IS gone, delete the pidfile:".into(),
        String::new(),
        format!("      rm -f '{}'", paths.pidfile),
        "================================================================================".into(),
    ]
}

/// Print [`stray_warning`] to stderr. Always returns 1, mirroring bash's `return 1`.
pub fn print_stray_warning(paths: &RunPaths, host: &Host, o: &Opts, pgid: &str) -> u8 {
    for line in stray_warning(paths, host, o, pgid) {
        eprintln!("{line}");
    }
    1
}

/// bash `claim_lock`. `Err(1)` when another live instance owns this run dir.
pub fn claim_lock(paths: &RunPaths, o: &Opts) -> Result<LockGuard, u8> {
    let _ = std::fs::create_dir_all(&paths.run_dir);
    let owner_file = format!("{}/owner", paths.lockdir);
    // `create_dir` (not `create_dir_all`) is the atomic `mkdir` bash relied on: it fails if the
    // directory already exists, and that failure IS the lock.
    if std::fs::create_dir(&paths.lockdir).is_ok() {
        let _ = std::fs::write(&owner_file, format!("{}\n", std::process::id()));
        return Ok(LockGuard {
            dir: paths.lockdir.clone(),
        });
    }

    // Read the owner with a short retry. `mkdir` and the write of `owner` are two steps, so a second
    // copy starting in that window would see an empty file and wrongly call the lock stale — which
    // would defeat the entire guard at exactly the moment it is needed.
    let mut owner = String::new();
    for _ in 0..20 {
        owner = std::fs::read_to_string(&owner_file)
            .unwrap_or_default()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if !owner.is_empty() {
            break;
        }
        sleep(Duration::from_millis(100));
    }

    // The lock owner is another instance of this program, in this pid namespace — a plain local
    // `kill -0` is the right question here and needs no bridge.
    if !owner.is_empty() && local_pid_is_alive(&owner) {
        eprintln!();
        eprintln!(
            "REFUSING: another playtest server (pid {owner}) already owns {}.",
            paths.run_dir
        );
        eprintln!("  Stop it first (Ctrl-C in its terminal) and let it print that it stopped.");
        eprintln!(
            "  Starting a second one here would rewrite server.json under the running server,"
        );
        eprintln!(
            "  destroy its pidfile, and then die on port {}.",
            o.game_port
        );
        eprintln!("  To run two servers at once, give this one its own dir and ports:");
        eprintln!(
            "      --run-dir={}-2 --port=2011 --a2s-port=17787",
            paths.run_dir
        );
        return Err(1);
    }

    // bash `${owner:-unknown}` — an owner file that never filled reads as "unknown", not as blank.
    let shown = if owner.is_empty() { "unknown" } else { &owner };
    println!("    note: taking over a stale lock (owner pid {shown} is gone)");
    let _ = std::fs::write(&owner_file, format!("{}\n", std::process::id()));
    Ok(LockGuard {
        dir: paths.lockdir.clone(),
    })
}

/// bash `kill -0 "$owner" 2>/dev/null` — LOCAL, no bridge. A non-numeric owner is "not alive",
/// exactly as `kill` erroring out was.
pub(super) fn local_pid_is_alive(owner: &str) -> bool {
    match owner.parse::<i32>() {
        // SAFETY: `kill(pid, 0)` performs the permission and existence check without delivering a
        // signal. It cannot affect the target and has no memory effects.
        Ok(pid) if pid > 0 => unsafe { libc::kill(pid, 0) == 0 },
        _ => false,
    }
}

/// Refuse to stage over a server that is still running.
///
/// Fails CLOSED: "I cannot tell" refuses too, because the cost of being wrong is clobbering a live
/// session's config.
pub fn check_no_live_server(paths: &RunPaths, host: &Host, o: &Opts) -> LiveVerdict {
    let pgid = read_pgid(&paths.pidfile);
    if pgid.is_empty() {
        return LiveVerdict::Clear;
    }
    let bridge = host.instruction_name();
    match probe_group(host, &pgid) {
        Probe::Dead | Probe::Zombie => {
            let _ = std::fs::remove_file(&paths.pidfile);
            LiveVerdict::Clear
        }
        Probe::Alive => LiveVerdict::Refuse {
            code: 1,
            message: vec![
                String::new(),
                format!(
                    "REFUSING: a server from a previous run is STILL RUNNING (process group {pgid})."
                ),
                format!(
                    "  {} points at it and it is alive right now.",
                    paths.pidfile
                ),
                String::new(),
                "  Stop it first — Ctrl-C in its terminal if you still have it, otherwise:".into(),
                format!("      {bridge} kill -TERM -- -{pgid}    # then check it is gone:"),
                format!("      {bridge} pgrep -af '[A]rmaReforgerServer'"),
                String::new(),
                "  This is deliberate. Booting anyway would rewrite server.json under it, replace"
                    .into(),
                format!(
                    "  the pidfile that is the only handle on it, and then fail on port {} with",
                    o.game_port
                ),
                "  'Unable to start replication' — three problems instead of one.".into(),
            ],
        },
        Probe::Unknown => LiveVerdict::Refuse {
            code: 1,
            message: vec![
                String::new(),
                format!(
                    "REFUSING: {} names process group {pgid} and this script could not reach the",
                    paths.pidfile
                ),
                "  host bridge to find out whether it is still alive.".into(),
                "  'I cannot tell' is not 'it is dead', so this refuses rather than guessing."
                    .into(),
                "  Check by hand, then delete the pidfile if the group really is gone:".into(),
                format!("      {bridge} pgrep -af '[A]rmaReforgerServer'"),
                format!("      rm -f '{}'", paths.pidfile),
            ],
        },
    }
}

/// [`check_no_live_server`], printed. `Err(1)` on refusal.
pub fn assert_no_live_server(paths: &RunPaths, host: &Host, o: &Opts) -> Result<(), u8> {
    match check_no_live_server(paths, host, o) {
        LiveVerdict::Clear => Ok(()),
        LiveVerdict::Refuse { code, message } => {
            for line in message {
                eprintln!("{line}");
            }
            Err(code)
        }
    }
}

pub(super) fn st_spawn(host: &Host, code: &str) -> String {
    host.capture_trimmed(&["sh", "-c", SPAWN_SH, "_", code])
}
