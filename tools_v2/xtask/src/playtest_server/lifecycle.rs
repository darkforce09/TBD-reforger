//! ═══ KILL DISCIPLINE, THE LIVENESS PROBE, AND THE RUN LOCK (T-608) ═══════════════════════════
//!
//! Reached before every other check on purpose, for two reasons: `--selftest` has to be able to get
//! here without a mission id, and [`assert_no_live_server`] has to run BEFORE staging rewrites
//! `server.json` underneath a server that is still running.

use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use super::Opts;
use super::host::Host;

/// The three run-dir paths the lifecycle owns.
pub struct RunPaths {
    pub run_dir: String,
    pub pidfile: String,
    /// The launcher's merged stdout+stderr. The boot loop polls this FILE rather than tailing a
    /// stream — see [`super::boot`].
    pub srv_out: String,
    pub lockdir: String,
}

impl RunPaths {
    pub fn new(run_dir: &str) -> RunPaths {
        RunPaths {
            run_dir: run_dir.to_string(),
            pidfile: format!("{run_dir}/server.pid"),
            srv_out: format!("{run_dir}/server.out"),
            lockdir: format!("{run_dir}/.run.lock"),
        }
    }
}

/// What the probe found. **Four states, and `Unknown` is NOT `Dead`.**
///
/// A three-valued answer would have been enough to make the T-608 bug unrepresentable, but `Zombie`
/// has to be separate too — see [`probe_group`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probe {
    Alive,
    Zombie,
    Dead,
    Unknown,
}

impl Probe {
    /// The four words the bash `printf`ed, used verbatim in the escalation message.
    pub fn word(self) -> &'static str {
        match self {
            Probe::Alive => "alive",
            Probe::Zombie => "zombie",
            Probe::Dead => "dead",
            Probe::Unknown => "unknown",
        }
    }

    /// Is the group CONFIRMED not to be holding sockets? `Unknown` is deliberately not included and
    /// there is no `bool` conversion that could let it slip in.
    fn confirmed_gone(self) -> bool {
        matches!(self, Probe::Dead | Probe::Zombie)
    }
}

/// The far side of the probe, run on the host, verbatim from the bash.
///
/// The sentinel is printed by the FAR SIDE. See [`probe_group`] for why that is the whole design.
const PROBE_SH: &str = r#"
    p=$1
    if kill -0 -- "-$p" 2>/dev/null; then
      live=0; seen=0
      for q in $(pgrep -g "$p" 2>/dev/null); do
        seen=$((seen + 1))
        st=$(sed -n "s/^State:[[:space:]]*\([A-Z]\).*/\1/p" "/proc/$q/status" 2>/dev/null)
        [ "$st" = "Z" ] || live=$((live + 1))
      done
      if [ "$seen" -gt 0 ] && [ "$live" -eq 0 ]; then echo "TBDPROBE=zombie"; else echo "TBDPROBE=alive"; fi
    else
      echo "TBDPROBE=dead"
    fi
  "#;

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

/// Whether `kill_run` may print its escalation line.
///
/// bash called it as `kill_run >/dev/null 2>&1` at every one of the FOUR selftest call sites and
/// bare everywhere else. Without this distinction the port's `--selftest` output gains two
/// `TERM did not settle …` lines the baseline does not have — caught by the byte diff, which is
/// exactly what that diff is for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Volume {
    Loud,
    Quiet,
}

fn kill_run_inner(paths: &RunPaths, host: &Host, volume: Volume) -> Result<(), String> {
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

// ── the run lock (T-608 / F5) ────────────────────────────────────────────────────────────────
// There was no lock and the run dir is fixed, so running the S3 "restart with --admin" command
// before Ctrl-C'ing the first server orphaned the running group: the second invocation's
// `rm -f "$PIDFILE"` destroyed the only handle, the first instance's `kill_run` then read no pidfile
// and reported "server exited" while its engine was still up, and the new boot died on the port.
// Two guards now, because they answer different questions:
//   claim_lock            — is another copy of THIS PROGRAM using this run dir?
//   assert_no_live_server — is a SERVER still running from a previous invocation of it?
// The second one matters on its own: a program that was killed leaves no lock but can very much
// leave a server.

/// Holds the lock dir for as long as it is alive.
///
/// bash used `trap 'release_lock' EXIT`. This is the same contract expressed as `Drop`, which is why
/// nothing under the lock may call `process::exit` — see the note at the `claim_lock` call site.
pub struct LockGuard {
    dir: String,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
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
            "REFUSING: another run-playtest-server.sh (pid {owner}) already owns {}.",
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
fn local_pid_is_alive(owner: &str) -> bool {
    match owner.parse::<i32>() {
        // SAFETY: `kill(pid, 0)` performs the permission and existence check without delivering a
        // signal. It cannot affect the target and has no memory effects.
        Ok(pid) if pid > 0 => unsafe { libc::kill(pid, 0) == 0 },
        _ => false,
    }
}

/// The verdict of [`check_no_live_server`], so the selftest can read the refusal text without
/// capturing a terminal — and so the `Unknown` arm is reachable in a unit test at all.
pub enum LiveVerdict {
    /// Nothing is running from a previous invocation. The pidfile has been cleared if it named a
    /// confirmed-dead group.
    Clear,
    Refuse {
        code: u8,
        message: Vec<String>,
    },
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

// ── --selftest ───────────────────────────────────────────────────────────────────────────────

/// Spawn a real `setsid` process group on the host and echo its pgid. `code` is shell the group
/// leader runs; that is how the TERM-ignoring case is built.
const SPAWN_SH: &str = r#"
      f=$(mktemp /tmp/tbd-rps-pg.XXXXXX)
      setsid sh -c "echo \$\$ > $f; $1" >/dev/null 2>&1 &
      n=0
      while [ ! -s "$f" ] && [ "$n" -lt 50 ]; do n=$((n+1)); sleep 0.1; done
      cat "$f"; rm -f "$f"
    "#;

fn st_spawn(host: &Host, code: &str) -> String {
    host.capture_trimmed(&["sh", "-c", SPAWN_SH, "_", code])
}

/// Tallies the ok/FAIL lines exactly as bash's `st_pass` / `st_fail` did.
struct Tally {
    rc: u8,
}

impl Tally {
    fn pass(&self, msg: &str) {
        println!("  ok    {msg}");
    }
    fn fail(&mut self, msg: &str) {
        println!("  FAIL  {msg}");
        self.rc = 1;
    }
    /// `cond ? pass : fail` — the `[ … ] && st_pass … || st_fail …` idiom, made unable to run both
    /// arms (which the bash form does whenever `st_pass` itself returns non-zero).
    fn check(&mut self, cond: bool, ok: &str, bad: &str) {
        if cond {
            self.pass(ok);
        } else {
            self.fail(bad);
        }
    }
}

/// `--selftest` — prove the kill path can FAIL, and cannot lie.
///
/// Same principle as `world-boot.sh:264` — a gate nobody has watched fail is not a gate. This one
/// exists because T-608's defect was invisible on every passing run: `kill_run` only lied when the
/// bridge flaked, which no green boot ever exercises. So the lie is reproduced here on purpose.
/// Boots no game server; spawns disposable `sleep` groups on the host and kills them.
pub fn selftest(host: &Host) -> u8 {
    println!(
        "==> run-playtest-server selftest (kill discipline must be unable to claim a false death)"
    );
    if !host.require_host() {
        return super::env_fail(
            "no host bridge — the selftest exercises the real bridge, so it needs one",
            "",
        );
    }

    let mut t = Tally { rc: 0 };
    let tmp = match mktemp_dir() {
        Some(d) => d,
        None => return super::env_fail("could not create a selftest temp dir", ""),
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
            "S1 kill_run returned SUCCESS with the group alive — this is the T-608 defect",
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
fn mktemp_dir() -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> RunPaths {
        let d = std::env::temp_dir().join(format!("tbd-rps-ut-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        RunPaths::new(d.to_str().unwrap())
    }

    /// A host that is in a container with no bridge — every probe answers `Unknown`.
    fn unreachable() -> Host {
        Host::detect().broken()
    }

    #[test]
    fn unknown_is_not_dead_and_has_no_bool_shortcut() {
        assert!(!Probe::Unknown.confirmed_gone());
        assert!(!Probe::Alive.confirmed_gone());
        assert!(Probe::Dead.confirmed_gone());
        // Zombie counts as gone — see `probe_group`: it holds no sockets, and folding it into
        // `alive` would make death permanently unconfirmable.
        assert!(Probe::Zombie.confirmed_gone());
    }

    #[test]
    fn an_empty_pgid_is_unknown_never_dead() {
        assert_eq!(probe_group(&Host::detect(), ""), Probe::Unknown);
    }

    #[test]
    fn a_broken_bridge_probes_unknown_not_dead() {
        // THE T-608 REGRESSION, at the unit level. `hostrun kill -0 … || return 0` produced `dead`
        // here, and that single misreading manufactured both halves of the orphan.
        assert_eq!(probe_group(&unreachable(), "424242"), Probe::Unknown);
    }

    #[test]
    fn no_pidfile_is_a_clean_kill_run_but_not_a_claim_of_death() {
        let p = tmp("nopid");
        assert!(kill_run(&p, &unreachable()).is_ok());
        let _ = std::fs::remove_dir_all(&p.run_dir);
    }

    #[test]
    fn kill_run_keeps_the_pidfile_when_it_cannot_confirm() {
        // The 15 s of polling this walks through is the price of pinning the property that matters
        // most: an unconfirmable death must not delete the only handle on the group.
        let p = tmp("unconfirmed");
        std::fs::write(&p.pidfile, "424242\n").unwrap();
        match kill_run(&p, &unreachable()) {
            Err(pgid) => assert_eq!(pgid, "424242"),
            Ok(()) => panic!("returned success over a group it never examined — the T-608 defect"),
        }
        assert!(
            Path::new(&p.pidfile).is_file(),
            "the pidfile is the only handle on a live group and must survive"
        );
        let _ = std::fs::remove_dir_all(&p.run_dir);
    }

    #[test]
    fn unreachable_bridge_refuses_to_stage_rather_than_guessing() {
        // The `Probe::Unknown` arm of `assert_no_live_server`. NOT reachable from the CLI on this
        // machine — the `require_host` preflight fires first (baseline e06) — so it is only ever
        // exercised here. Named in the port's report as such.
        let p = tmp("unkstate");
        std::fs::write(&p.pidfile, "424242\n").unwrap();
        let o = Opts::defaults("/h");
        match check_no_live_server(&p, &unreachable(), &o) {
            LiveVerdict::Refuse { code, message } => {
                assert_eq!(code, 1);
                let text = message.join("\n");
                assert!(text.contains("could not reach the"), "{text}");
                assert!(
                    text.contains("'I cannot tell' is not 'it is dead'"),
                    "{text}"
                );
                assert!(text.contains("424242"), "{text}");
            }
            LiveVerdict::Clear => panic!("staged over a server it could not rule out"),
        }
        assert!(
            Path::new(&p.pidfile).is_file(),
            "an unknown state must not clear the pidfile"
        );
        let _ = std::fs::remove_dir_all(&p.run_dir);
    }

    #[test]
    fn an_empty_pidfile_clears_the_way() {
        let p = tmp("emptypid");
        std::fs::write(&p.pidfile, "  \n").unwrap();
        assert!(matches!(
            check_no_live_server(&p, &unreachable(), &Opts::defaults("/h")),
            LiveVerdict::Clear
        ));
        let _ = std::fs::remove_dir_all(&p.run_dir);
    }

    #[test]
    fn the_lock_is_exclusive_and_released_on_drop() {
        let p = tmp("lock");
        let o = Opts::defaults("/h");
        {
            let _g = claim_lock(&p, &o).expect("first claim");
            assert!(Path::new(&p.lockdir).is_dir());
            // A second claim by a LIVE owner (this very process) must refuse.
            match claim_lock(&p, &o) {
                Err(code) => assert_eq!(code, 1),
                Ok(_) => panic!("two instances took the same run dir — the F5 orphan bug"),
            }
        }
        assert!(
            !Path::new(&p.lockdir).exists(),
            "Drop must release the lock, as bash's `trap release_lock EXIT` did"
        );
        let _ = std::fs::remove_dir_all(&p.run_dir);
    }

    #[test]
    fn a_stale_lock_is_taken_over() {
        let p = tmp("stale");
        std::fs::create_dir_all(&p.lockdir).unwrap();
        // pid 1 is alive but 999999 will not be; bash's own test used the same trick.
        std::fs::write(format!("{}/owner", p.lockdir), "999999\n").unwrap();
        let g = claim_lock(&p, &Opts::defaults("/h")).expect("stale lock must be taken over");
        let owner = std::fs::read_to_string(format!("{}/owner", p.lockdir)).unwrap();
        assert_eq!(owner.trim(), std::process::id().to_string());
        drop(g);
        let _ = std::fs::remove_dir_all(&p.run_dir);
    }

    #[test]
    fn an_owner_file_that_never_fills_is_stale_not_a_refusal() {
        // The 20 x 0.1 s retry window: an empty owner after 2 s means the writer died mid-claim.
        let p = tmp("emptyowner");
        std::fs::create_dir_all(&p.lockdir).unwrap();
        std::fs::write(format!("{}/owner", p.lockdir), "").unwrap();
        let g = claim_lock(&p, &Opts::defaults("/h"));
        assert!(
            g.is_ok(),
            "an empty owner file must not deadlock the run dir"
        );
        let _ = std::fs::remove_dir_all(&p.run_dir);
    }

    #[test]
    fn local_liveness_handles_junk_owners() {
        assert!(local_pid_is_alive(&std::process::id().to_string()));
        assert!(!local_pid_is_alive("abc"));
        assert!(!local_pid_is_alive(""));
        assert!(
            !local_pid_is_alive("0"),
            "pgid 0 means our own group — never a lock owner"
        );
        assert!(!local_pid_is_alive("-1"));
    }

    #[test]
    fn stray_warning_names_the_group_the_ports_and_the_pidfile() {
        let p = RunPaths::new("/run/dir");
        let mut o = Opts::defaults("/h");
        o.game_port = "2001".into();
        o.a2s_port = "17777".into();
        let text = stray_warning(&p, &Host::detect(), &o, "31337").join("\n");
        assert!(text.contains("process group: 31337"));
        assert!(text.contains("kill -9 -- -31337"));
        assert!(text.contains("holds UDP 2001 / 17777"));
        assert!(text.contains("rm -f '/run/dir/server.pid'"));
        assert!(text.contains("pgrep -af '[A]rmaReforgerServer'"));
    }

    #[test]
    fn read_pgid_strips_all_whitespace() {
        let p = tmp("readpgid");
        std::fs::write(&p.pidfile, " 12 34 \n").unwrap();
        assert_eq!(read_pgid(&p.pidfile), "1234");
        assert_eq!(read_pgid("/nonexistent/pid"), "");
        let _ = std::fs::remove_dir_all(&p.run_dir);
    }
}
