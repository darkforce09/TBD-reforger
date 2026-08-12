//! Launching the engine, waiting for a verdict, and shutting down.
//!
//! The log READING this drives lives in [`super::logread`] — split out when this file passed
//! SIZE-3's 1000-line hard fail. Everything left here needs a live process.
//!
//! ── NEVER TAIL A POSSIBLY-HANGING STREAM ─────────────────────────────────────────────────────
//!
//! The launcher's merged output goes to a FILE (`$RUN_DIR/server.out`) and the wait loop polls that
//! file. Three outcomes are polled for: the room registration, a config refusal, and an engine fatal.
//!
//! HOW LONG THIS TAKES IS NOT A RELIABLE NUMBER, and the bash comment here used to claim one
//! ("~95 s measured, 300 s generous"). Measured on this machine, two boots minutes apart:
//!
//! ```text
//!   passing boot   `Server registered with address:` landed 13 s after `Starting RPL server`
//!   failing boot   never, across the full 300 s — world long up, mission in LOBBY, vehicles
//!                  spawned, and the room simply never registered
//! ```
//!
//! Same binary, same config. So the wait is variable and the 300 s below is a bound on our patience,
//! not an estimate of the engine's. What is worth printing is not a countdown against a fictional
//! average but WHICH PHASE the boot is in, which [`boot_phase`] reads out of the log — "still
//! compiling" and "world up, registration pending" are different problems and the old loop printed
//! the same sentence for both, thirty times.
//!
//! ── LIVENESS IS CHECKED ON THE PROCESS GROUP, NOT ON THE LAUNCHER ────────────────────────────
//!
//! Under the host bridge the local launcher returns almost immediately (`world-boot.sh:809` records
//! the same trap), so `kill -0 $LAUNCHER` reports "died" while the engine is still compiling
//! scripts — measured here as a FAILED verdict 9 KB into a boot that was going fine. And it is
//! checked every 10 s rather than every tick because each probe spawns a bridge process.

use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;

use super::Opts;
use super::host::Host;
use super::lifecycle::{self, Probe, RunPaths};
use super::logread::{
    assert_local_addon_won, boot_phase, console_log_path, dump_engine_errors, grep_n, has, has_re,
    loaded_addon_line, log, world_is_up,
};

/// Set by the SIGINT/SIGTERM handler; polled by both wait loops.
///
/// bash used `trap '…' INT TERM`, which runs the handler body between commands. A Rust signal handler
/// may only do async-signal-safe work, so it stores a flag and the loops act on it — the same
/// "between commands" granularity, made explicit. Poll slices are 100 ms so Ctrl-C still feels
/// immediate inside the 5 s steady-state sleep.
static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_stop_signal(_sig: libc::c_int) {
    // A single relaxed-or-stronger atomic store: no allocation, no locks, no reentrancy. This is the
    // only thing that may legally happen in here.
    STOP_REQUESTED.store(true, Ordering::SeqCst);
}

fn install_stop_handlers() {
    // SAFETY: installing a handler that performs one atomic store. `signal` is the same primitive
    // bash's `trap` compiles down to, and the handler touches nothing that could be mid-mutation.
    let handler = on_stop_signal as *const () as libc::sighandler_t;
    unsafe {
        libc::signal(libc::SIGINT, handler);
        libc::signal(libc::SIGTERM, handler);
    }
}

/// Sleep in 100 ms slices. Returns true as soon as a stop was requested.
fn nap(total: Duration) -> bool {
    let slice = Duration::from_millis(100);
    let mut left = total;
    while left > Duration::ZERO {
        if STOP_REQUESTED.load(Ordering::SeqCst) {
            return true;
        }
        let step = if left < slice { left } else { slice };
        sleep(step);
        left -= step;
    }
    STOP_REQUESTED.load(Ordering::SeqCst)
}

/// Everything the boot half needs from the caller.
pub struct BootCtx<'a> {
    pub host: &'a Host,
    pub paths: &'a RunPaths,
    pub opts: &'a Opts,
    pub server_dir: &'a str,
    pub cmd_display: &'a str,
    pub addon_guid: &'a str,
    pub lan_ip: &'a str,
    pub scenario: &'a str,
}

/// The shell the far side runs. Verbatim from the bash, including the double space left where
/// `$TIMEOUT_PREFIX` interpolates empty — it is shell whitespace and the engine never sees it, but
/// keeping the shape means a `set -x` transcript of either implementation reads the same.
fn launcher_script(timeout_prefix: &str) -> String {
    format!(
        "\n  echo $$ > \"$1/server.pid\"\n  exec {timeout_prefix} ./ArmaReforgerServer \\\n    -addonsDir \"$1/addons\" -config \"$1/server.json\" -profile \"$1/profile\" \\\n    -maxFPS 60 -logStats 30000 -nothrow\n"
    )
}

/// Everything from `rm -rf $LOGROOT` to the final exit code.
pub fn boot_and_wait(c: &BootCtx<'_>) -> u8 {
    let o = c.opts;
    // The pidfile is NOT deleted here. `assert_no_live_server` above already removed it, and only
    // after confirming the group it named was dead; if it could not confirm that, this line was never
    // reached. An unconditional `rm -f "$PIDFILE"` is what let a second invocation orphan the first
    // one's server (T-608 / F5) — it threw away the only handle on a live process group.
    let logroot = format!("{}/profile/logs", o.run_dir);
    let _ = std::fs::remove_dir_all(&logroot);

    install_stop_handlers();

    // bash `[ -n "$RUN_TIMEOUT" ] && TIMEOUT_PREFIX="timeout -s TERM $RUN_TIMEOUT"`.
    //
    // NOTE ON `timeout(1)` AND WHY IT IS STILL USED HERE. `tbd_gate::proc::Run` kills a process
    // GROUP on timeout precisely because bash's `timeout` kills only its direct child and the engine
    // forks — grandchildren kept the port and the log. That correction does NOT apply to this call
    // site: the `timeout` here runs on the FAR side of the bridge, inside the `setsid` group, and its
    // TERM goes to the engine it `exec`ed (so `timeout` is not even in the tree any more — `exec`
    // replaced the shell). The group-level cleanup is `kill_run`'s job either way, and `kill_run`
    // signals `-$pgid`. So this stays a far-side `timeout` and the local `Run` timeout is deliberately
    // NOT used for the launcher: a local deadline would kill the bridge proxy and leave the engine up,
    // which is the orphan this whole program exists to prevent.
    let timeout_prefix = if o.run_timeout.is_empty() {
        String::new()
    } else {
        format!("timeout -s TERM {}", o.run_timeout)
    };

    println!(
        "==> booting (addon {}, scenario {}, mission {})",
        c.addon_guid, c.scenario, o.mission_id
    );
    println!("    {}", c.cmd_display);

    let sink = match std::fs::File::create(&c.paths.srv_out) {
        Ok(f) => f,
        Err(e) => {
            return super::env_fail(&format!("cannot write {}: {e}", c.paths.srv_out), "");
        }
    };
    let script = launcher_script(&timeout_prefix);
    let launcher = c.host.spawn_background(
        &[
            "env",
            "-C",
            c.server_dir,
            "setsid",
            "sh",
            "-c",
            &script,
            "_",
            &o.run_dir,
        ],
        sink,
    );
    // bash's `&` never checked that the launcher started; a spawn failure showed up much later as
    // "the server never registered a backend room", blaming the engine for a bridge that never ran.
    // FAIL-OPEN CLOSED — it made the program lie about having booted anything.
    let mut launcher = match launcher {
        Ok(child) => child,
        Err(e) => {
            return super::env_fail(&format!("could not start the host launcher: {e}"), "");
        }
    };

    let verdict = wait_for_verdict(c);

    // The trap body: `echo ""; echo "==> stopping server"; if kill_run; then …; exit 0; fi; …`
    if matches!(verdict, Verdict::Interrupted) {
        println!();
        println!("==> stopping server");
        let code = match lifecycle::kill_run(c.paths, c.host) {
            Ok(()) => {
                println!("==> stopped (process group confirmed gone)");
                0
            }
            Err(pgid) => lifecycle::print_stray_warning(c.paths, c.host, o, &pgid),
        };
        let _ = launcher.wait();
        return code;
    }

    if let Some(code) = report_failure(c, &verdict) {
        let _ = launcher.wait();
        return code;
    }

    // Registered. Now the hard gate.
    if !assert_local_addon_won(c.paths, o, c.addon_guid) {
        let mut code = 1;
        if let Err(pgid) = lifecycle::kill_run(c.paths, c.host) {
            code = lifecycle::print_stray_warning(c.paths, c.host, o, &pgid);
        }
        let _ = launcher.wait();
        return code;
    }

    print_banner(c);
    let code = tail_until_the_server_stops(c);
    let _ = launcher.wait();
    code
}

/// Which of the four ways the wait loop can end.
enum Verdict {
    Registered,
    /// The engine refused the config or could not start.
    Fatal,
    /// The process group is CONFIRMED gone.
    Died,
    /// 300 s elapsed with none of the above.
    NeverRegistered,
    /// Ctrl-C / SIGTERM.
    Interrupted,
}

fn wait_for_verdict(c: &BootCtx<'_>) -> Verdict {
    let mut i = 0;
    while i < 600 {
        i += 1;
        if Path::new(&c.paths.srv_out).is_file() {
            let t = log(c.paths);
            if has(&t, "Server registered with address:") {
                return Verdict::Registered;
            }
            if has_re(
                &t,
                "There are errors in server config!|Unable to initialize the game|Unable to start replication",
            ) {
                return Verdict::Fatal;
            }
        }
        if i % 20 == 0 {
            let pgid = lifecycle::read_pgid(&c.paths.pidfile);
            // PRESERVED: an EMPTY pidfile skips the liveness check entirely and does NOT end the
            // loop. The launcher writes the pidfile as its first act, so an empty one means "not yet",
            // and reading that as death would fail every boot in its first ten seconds.
            if !pgid.is_empty() {
                // Same rule as kill_run: only a CONFIRMED death breaks this loop. A bridge failure
                // used to land here as DIED, and then the kill path could not clean up what it had
                // wrongly declared dead — one unreachable probe manufactured both halves of the
                // orphan.
                match lifecycle::probe_group(c.host, &pgid) {
                    Probe::Dead | Probe::Zombie => return Verdict::Died,
                    Probe::Unknown => println!(
                        "    ... note: could not reach the host bridge to check the server; NOT reading that as death"
                    ),
                    Probe::Alive => {}
                }
            }
            println!("    ... {}s — {}", i / 2, boot_phase(c.paths));
        }
        if nap(Duration::from_millis(500)) {
            return Verdict::Interrupted;
        }
    }
    Verdict::NeverRegistered
}

/// The diagnosis block. `None` means "no failure — carry on to the hard gate".
fn report_failure(c: &BootCtx<'_>, v: &Verdict) -> Option<u8> {
    let o = c.opts;
    match v {
        Verdict::Registered | Verdict::Interrupted => return None,
        _ => {}
    }
    // bash `kill_run || true` — the stray, if any, is reported by `print_stray_warning` at the end.
    let stray = lifecycle::kill_run(c.paths, c.host).err();
    eprintln!();
    match v {
        Verdict::Died => {
            eprintln!("FAILED: the server process exited before registering a room.");
            eprintln!("        phase reached: {}", boot_phase(c.paths));
            dump_engine_errors(c.paths);
        }
        Verdict::Fatal => {
            eprintln!("FAILED: the engine refused the config or could not start.");
            eprintln!("        phase reached: {}", boot_phase(c.paths));
            dump_engine_errors(c.paths);
        }
        _ if world_is_up(c.paths) => {
            // The measured failure. The world came up, the mod loaded the mission, bodies spawned —
            // and the room never registered. Pointing at `(E)` lines here actively misleads, because
            // there is no `(E)` line for this: the engine logs nothing at all when the backend
            // handshake stalls. The evidence is an ABSENCE, so the diagnosis has to say so out loud.
            eprintln!("FAILED: the world came up but the server never registered a backend room.");
            eprintln!("        phase reached: {}", boot_phase(c.paths));
            eprintln!();
            eprintln!(
                "  This is a REGISTRATION hang, not a load failure. Nothing was wrong with your mod,"
            );
            eprintln!(
                "  your mission or your config — all of that demonstrably worked. What is missing is"
            );
            eprintln!(
                "  one line, 'BACKEND : Server registered with address:', and NO error line names it."
            );
            eprintln!();
            eprintln!(
                "--- every BACKEND line in this boot (the answer is here, or its absence is) ---"
            );
            let t = log(c.paths);
            let backend = grep_n(
                &t,
                r"^[[:space:]]*BACKEND[[:space:]]*(\([EWF]\))?[[:space:]]*:",
            );
            // `| tail -25`
            for l in backend.iter().skip(backend.len().saturating_sub(25)) {
                eprintln!("{l}");
            }
            eprintln!();
            if has(&t, "Attempting online Game Config instead.") {
                eprintln!(
                    "  Fingerprint: 'Attempting online Game Config instead.' with no BACKEND progress after"
                );
                eprintln!(
                    "  it means the online handshake never completed. Measured 2026-07-31 on a boot that"
                );
                eprintln!(
                    "  was otherwise perfect. It is upstream of us: check that this machine can reach"
                );
                eprintln!(
                    "  Bohemia's backend at all, then simply run this script again — the same command"
                );
                eprintln!("  registered in 13 s a few minutes later with nothing changed.");
                eprintln!();
            }
            dump_engine_errors(c.paths);
        }
        _ => {
            eprintln!(
                "FAILED: the server never registered a backend room, and never finished loading either."
            );
            eprintln!("        phase reached: {}", boot_phase(c.paths));
            eprintln!(
                "        It did not get far enough for registration to be the suspect — read the phase."
            );
            dump_engine_errors(c.paths);
        }
    }
    eprintln!("--- full output: {}", c.paths.srv_out);
    eprintln!();
    eprintln!("The server binary exits 0 even when compilation fails — read the log, never $?.");
    if let Some(pgid) = stray {
        // bash `print_stray_warning || true` — the warning's own rc 1 is discarded because the
        // script's exit is already 1 for a different reason.
        let _ = lifecycle::print_stray_warning(c.paths, c.host, o, &pgid);
    }
    Some(1)
}

fn print_banner(c: &BootCtx<'_>) {
    let o = c.opts;
    let t = log(c.paths);
    // `grep -m1` — the FIRST match only.
    let reg_line = t
        .lines()
        .find(|l| l.contains("Server registered with address:"))
        .unwrap_or_default()
        .to_string();
    let reg_addr = super::grep_o(r"[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+:[0-9]+", &reg_line)
        .first()
        .cloned()
        .unwrap_or_default();
    let join_code = t
        .lines()
        .find(|l| l.contains("Direct Join Code:"))
        .map(|l| super::grep_o("[0-9]{6,}", l))
        .and_then(|v| v.first().cloned())
        .unwrap_or_default();
    let loaded_line = loaded_addon_line(&t, c.addon_guid)
        .map(|l| l.trim_start().to_string()) // `sed 's/^[[:space:]]*//'`
        .unwrap_or_default();

    // bash `${REG_ADDR:-$LAN_IP:$GAME_PORT}` / `${JOIN_CODE:-<none printed>}`.
    let addr = if reg_addr.is_empty() {
        format!("{}:{}", c.lan_ip, o.game_port)
    } else {
        reg_addr
    };
    let code = if join_code.is_empty() {
        "<none printed>".to_string()
    } else {
        join_code
    };

    println!();
    println!("================================================================================");
    println!("  SERVER UP — second client joins with:");
    println!();
    println!("    Multiplayer -> Direct Join -> {addr}");
    println!("    or Direct Join Code:          {code}");
    println!();
    println!("  (the join code is re-minted on every boot — read it from THIS run, not a doc)");
    println!("================================================================================");
    println!("  proof, from this boot:");
    println!("    {reg_line}");
    println!("    {loaded_line}");
    println!("================================================================================");
    println!();
    println!("  THE CLIENT SIDE IS NOT PROVEN BY THIS SCRIPT. The server advertises");
    println!(
        "  game.mods[] = [{}], and the joining client resolves that id from the",
        c.addon_guid
    );
    println!("  WORKSHOP, where it is pinned at the stale version 1.0.1. The server is running");
    println!("  your checkout. If the friend's client refuses the join on a version/content");
    println!("  mismatch, or joins and behaves like older code, that skew is the first suspect");
    println!("  — re-publish tbd-framework from Workbench before blaming the mod.");
    println!();
    println!("  Ctrl-C stops the server. Wait for '==> stopped (process group confirmed gone)'");
    println!("  before starting another one — that line means the group was PROVED dead, not");
    println!("  assumed dead. If you get a STRAY SERVER block instead, run the command it prints:");
    println!(
        "  a survivor holds {}/{} and the next boot will fail on the port.",
        o.game_port, o.a2s_port
    );

    if !o.admins.is_empty() {
        // bash `${ADMINS[*]}` — space-joined.
        println!("  admins configured: {}", o.admins.join(" "));
        println!("  (the engine schema-validated these; whether a given id maps to the human who");
        println!("   connects is only observable once they DO connect — check '#tbd' in chat.)");
        println!();
    }
}

/// Foreground from here so the operator watches the live log; Ctrl-C stops the server.
///
/// Wait on the SERVER's process group, NOT on the launcher.
///
/// `wait "$LAUNCHER"` is what the bash said first, and it was a live instance of the defect this
/// whole program exists to prevent: under the host bridge the local launcher returns the moment the
/// server is `setsid`-detached, so `wait` fell straight through, `kill_run` ran, and the script
/// printed "SERVER UP" and then killed the server about five seconds later. It exited 0. The only
/// symptom was a friend who could not join a server the banner had just declared up.
///
/// The probe is the tri-state one for the same reason `kill_run` uses it: a bridge hiccup here would
/// otherwise fall straight out of this loop and report a running server as exited.
fn tail_until_the_server_stops(c: &BootCtx<'_>) -> u8 {
    let console = console_log_path(&c.opts.run_dir);
    let target = if Path::new(&console).is_file() {
        println!("==> tailing {console}");
        console
    } else {
        println!("==> tailing {}", c.paths.srv_out);
        c.paths.srv_out.clone()
    };
    // A local `tail -f`: the console log lives under $HOME, which is shared with the host, so there
    // is nothing to bridge. Left as a child process rather than reimplemented — `tail -f` handles
    // truncation and rotation and we would only get that wrong.
    let mut tail = std::process::Command::new("tail")
        .arg("-f")
        .arg(&target)
        .spawn()
        .ok();

    loop {
        let pgid = lifecycle::read_pgid(&c.paths.pidfile);
        if pgid.is_empty() {
            break;
        }
        match lifecycle::probe_group(c.host, &pgid) {
            Probe::Dead | Probe::Zombie => break,
            Probe::Unknown => println!(
                "    (host bridge unreachable — still assuming the server is UP; it is not evidence of exit)"
            ),
            Probe::Alive => {}
        }
        if nap(Duration::from_secs(5)) {
            // Ctrl-C. bash's trap ran the same three lines from inside the sleep.
            break;
        }
    }

    if let Some(t) = tail.as_mut() {
        let _ = t.kill();
        let _ = t.wait();
    }
    // The interrupted path prints the trap's own header; the natural-exit path does not.
    if STOP_REQUESTED.load(Ordering::SeqCst) {
        println!();
        println!("==> stopping server");
        return match lifecycle::kill_run(c.paths, c.host) {
            Ok(()) => {
                println!("==> stopped (process group confirmed gone)");
                0
            }
            Err(pgid) => lifecycle::print_stray_warning(c.paths, c.host, c.opts, &pgid),
        };
    }
    let _ = std::io::stdout().flush();
    println!();
    match lifecycle::kill_run(c.paths, c.host) {
        Ok(()) => {
            println!("==> server stopped (process group confirmed gone)");
            0
        }
        Err(pgid) => lifecycle::print_stray_warning(c.paths, c.host, c.opts, &pgid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_launcher_argv_is_the_one_the_engine_needs() {
        // NEVER EXECUTED BY A TEST MACHINE WITHOUT THE ENGINE, so the argv itself is the contract.
        // Flag ORDER matches `deploy-staging.sh:1659` deliberately.
        let s = launcher_script("");
        assert!(s.contains("echo $$ > \"$1/server.pid\""));
        assert!(
            s.contains("exec  ./ArmaReforgerServer"),
            "the empty-prefix double space is bash's: {s}"
        );
        assert!(s.contains("-addonsDir \"$1/addons\""));
        assert!(s.contains("-config \"$1/server.json\""));
        assert!(s.contains("-profile \"$1/profile\""));
        assert!(s.contains("-maxFPS 60 -logStats 30000 -nothrow"));
        // BOTH flags together. That combination is the entire finding of T-604: `-addonsDir` alone
        // registers no room, `-config` alone silently runs the stale Workshop pak.
        assert!(s.contains("-addonsDir") && s.contains("-config"));
    }

    #[test]
    fn a_timeout_becomes_a_far_side_timeout_prefix() {
        assert!(
            launcher_script("timeout -s TERM 30")
                .contains("exec timeout -s TERM 30 ./ArmaReforgerServer")
        );
    }

    #[test]
    fn the_join_details_are_scraped_out_of_the_engines_lines() {
        let reg = "BACKEND  : Server registered with address: 192.168.0.117:2001";
        assert_eq!(
            super::super::grep_o(r"[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+:[0-9]+", reg)[0],
            "192.168.0.117:2001"
        );
        assert_eq!(
            super::super::grep_o("[0-9]{6,}", "BACKEND  : Direct Join Code: 0207990185")[0],
            "0207990185"
        );
    }
}
