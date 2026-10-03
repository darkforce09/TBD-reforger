//! The verdict of `mod remote-logs` over one log, its self-test, and the entry that picks a path.
//!
//! **Role:** `run` routes to the self-test, a local `--file` or the remote fetch of
//! [`super::remote_fetch`]; `check_log` grades one log.
//!
//! **Position:** called by `xtask`'s `mod remote-logs` dispatch; the patterns come from the
//! parent.
//!
//! **Signals & state:** none held; the self-test writes and removes a temp folder.
//!
//! **Invariants:** an unreadable or absent log is ENVIRONMENT 3, never a zero count; `--instance`
//! with `--file` or `--selftest` is ENVIRONMENT, because nothing is fetched.

use super::*;

/// Entry for `xtask mod remote-logs`.
pub fn run(file: Option<PathBuf>, selftest: bool, instance: Option<u16>) -> Result<u8> {
    if instance.is_some() && (selftest || file.is_some()) {
        return Ok(env_fail(
            "--instance picks the staging server whose log is fetched; it does not combine with \
             --file or --selftest",
        ));
    }
    if selftest {
        return Ok(cmd_selftest());
    }
    if let Some(path) = file {
        return Ok(check_log(&path));
    }
    super::remote_fetch::cmd_remote(instance)
}

pub(super) fn min_tagged() -> u32 {
    std::env::var("TBD_MIN_TAGGED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20)
}

pub(super) fn env_fail(msg: &str) -> u8 {
    eprintln!("ENVIRONMENT: {msg}");
    eprintln!("The log was never examined, so this says NOTHING about the mod.");
    3
}

pub(super) fn count_matching_lines(pat: &Pattern, text: &str) -> u32 {
    text.lines().filter(|line| pat.is_match(line)).count() as u32
}

pub(super) fn matching_lines<'a>(pat: &Pattern, text: &'a str) -> Vec<&'a str> {
    text.lines().filter(|line| pat.is_match(line)).collect()
}

/// Local verdict over one log file. Remote path fetches once, then calls this.
pub(super) fn check_log(log: &Path) -> u8 {
    let text = match fs::read_to_string(log) {
        Ok(t) => t,
        Err(_) if !log.is_file() => {
            return env_fail(&format!("no such log file: {}", log.display()));
        }
        Err(e) => {
            // An unreadable log was never examined: ENVIRONMENT, never a zero count, which
            // would read as STALE BUILD.
            return env_fail(&format!("could not read log file {}: {e}", log.display()));
        }
    };

    let pat_extract = Pattern::regex(PAT_EXTRACT).expect("PAT_EXTRACT");
    let pat_tagged = Pattern::regex(PAT_TAGGED).expect("PAT_TAGGED");
    let pat_mission = Pattern::regex(PAT_MISSION).expect("PAT_MISSION");
    let pat_slots = Pattern::regex(PAT_SLOTS).expect("PAT_SLOTS");
    let pat_lobby = Pattern::regex(PAT_LOBBY).expect("PAT_LOBBY");
    let pat_loadout = Pattern::regex(PAT_LOADOUT).expect("PAT_LOADOUT");
    let pat_assigned = Pattern::regex(PAT_ASSIGNED).expect("PAT_ASSIGNED");
    let pat_errors = Pattern::regex(PAT_ERRORS).expect("PAT_ERRORS");

    println!("Log: {}", log.display());
    println!("---");
    let extracted = matching_lines(&pat_extract, &text);
    let start = extracted.len().saturating_sub(80);
    for line in &extracted[start..] {
        println!("{line}");
    }
    println!("---");

    let mut fail = false;
    let tagged = count_matching_lines(&pat_tagged, &text);
    println!("[TBD][ tagged lines: {tagged}");
    let floor = min_tagged();
    if tagged == 0 {
        println!("FAIL: STALE BUILD — zero '[TBD][' lines.");
        println!("      Workshop 1.0.1 logs flat '[TBD] …' with no subsystem tag; the current");
        println!("      build tags every line. A '-config'-only server downloads that stale copy");
        println!("      and looks healthy while running months-old script. Boot with -addonsDir,");
        println!("      or use `cargo xtask mod playtest` which asserts the local addon won.");
        fail = true;
    } else if tagged < floor {
        println!("WARN: only {tagged} tagged lines (advisory floor {floor}).");
        println!(
            "      Measured healthy boots: 108 (18-slot msn_8f3a2c), 147 (slot-loadout-coverage)."
        );
        println!("      Not a failure — the count is mission-dependent — but a boot this quiet");
        println!(
            "      usually means the mod stopped early. The named checks below are the verdict."
        );
    }

    let require_line = |label: &str, pat: &Pattern, varies: &str| -> bool {
        match probe_str(pat, &text) {
            Ok(true) => {
                println!("ok   {label}");
                true
            }
            Ok(false) => {
                println!("MISSING: {label}");
                println!("         pattern: {}", pat.source());
                println!("         Everything after this prefix is expected to vary: {varies}");
                false
            }
            Err(_) => {
                // probe_str does not fail today; the arm keeps "did not execute" distinct.
                println!("FAIL: {label} — grep exited ?; the check did not execute.");
                false
            }
        }
    };

    if !require_line(
        "mission document loaded",
        &pat_mission,
        "name, slot count, source=",
    ) {
        fail = true;
    }
    if !require_line(
        "slot bodies materialized",
        &pat_slots,
        "slot id, faction:squad:role, kit, coordinates",
    ) {
        fail = true;
    }
    if !require_line(
        "reached LOBBY",
        &pat_lobby,
        "nothing — this is a state-machine edge, not prose",
    ) {
        fail = true;
    }

    match probe_str(&pat_errors, &text) {
        Ok(true) => {
            println!("FAIL: compile / unknown-class / spawn errors present:");
            for line in matching_lines(&pat_errors, &text).into_iter().take(10) {
                println!("{line}");
            }
            fail = true;
        }
        Ok(false) => println!("ok   no compile or spawn-logic errors"),
        Err(_) => {
            println!("FAIL: error scan exited ?; the check did not execute.");
            fail = true;
        }
    }

    match probe_str(&pat_loadout, &text) {
        Ok(true) => {
            let n = count_matching_lines(&pat_loadout, &text);
            println!("ok   loadout pass ran ({n} [Loadout][Slot] lines)");
        }
        _ => {
            println!(
                "note no [TBD][Loadout][Slot] lines — legitimate if the mission authors no loadouts."
            );
            println!(
                "     (Do NOT grep [TBD][Loadout][Player]; no Print emits it. The tag is [Slot].)"
            );
        }
    }

    if fail {
        println!("VERDICT: FAIL");
        return 1;
    }

    match probe_str(&pat_assigned, &text) {
        Ok(true) => {
            println!("VERDICT: PASS — boot healthy and at least one player was seated.");
            0
        }
        _ => {
            println!(
                "VERDICT: PARTIAL — boot healthy, no player has joined yet (join a client to finish V6)."
            );
            2
        }
    }
}

pub(super) fn cmd_selftest() -> u8 {
    let tmp = tempfile_dir("tbd-logrep-selftest");
    let tmp = match tmp {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SELFTEST: FAIL (tempdir: {e})");
            return 1;
        }
    };

    write_log(
        &tmp.join("stale.log"),
        &[
            "SCRIPT : [TBD] Mission loaded from backend: something",
            "SCRIPT : [TBD] SpawnManager: built slot spawn",
            "SCRIPT : [TBD] Stage → LOBBY",
        ],
    );
    write_log(
        &tmp.join("healthy.log"),
        &[
            "SCRIPT : [TBD][Mission] loaded id=msn_x name='N' slots=7 source=profile",
            "SCRIPT : [TBD][Slots] Slot-1 s (a:b:c:0) kit kit:x at <1, 2, 3>",
            "SCRIPT : [TBD][Loadout][Slot] slot=a:b:c:0 loadout pass complete gear=1/1 cargo=0/0",
            "SCRIPT : [TBD][Stage] LOADING -> LOBBY",
        ],
    );
    write_log(
        &tmp.join("invalid.log"),
        &[
            "SCRIPT (E): [TBD] Mission loaded but invalid — staying in LOADING.",
            "SCRIPT : [TBD][Validate] mission result=FAIL errors=3 warnings=0",
        ],
    );

    let seated_flat = tmp.join("seated-flat.log");
    let seated_tagged = tmp.join("seated-tagged.log");
    let _ = fs::copy(tmp.join("healthy.log"), &seated_flat);
    let _ = fs::copy(tmp.join("healthy.log"), &seated_tagged);
    append_line(
        &seated_flat,
        "SCRIPT : [TBD] SpawnManager: assigned slot a:b:c:0 to player 2 at (1,3)",
    );
    append_line(
        &seated_tagged,
        "SCRIPT : [TBD][Spawn] player=2 assigned slot=a:b:c:0 at=(1,3)",
    );

    let mut bad = false;
    bad |= !expect("stale-1.0.1-must-fail", 1, &tmp.join("stale.log"));
    bad |= !expect("healthy-no-player-is-partial", 2, &tmp.join("healthy.log"));
    bad |= !expect("mission-invalid-must-fail", 1, &tmp.join("invalid.log"));
    bad |= !expect("seated-flat-format-is-pass", 0, &seated_flat);
    bad |= !expect("seated-tagged-format-is-pass", 0, &seated_tagged);

    let _ = fs::remove_dir_all(&tmp);
    if bad {
        println!("SELFTEST: FAIL");
        1
    } else {
        println!("SELFTEST: PASS");
        0
    }
}

pub(super) fn expect(name: &str, want: u8, file: &Path) -> bool {
    // The quiet path: the same exit codes as `check_log`, nothing printed.
    let rc = check_log_quiet(file);
    if rc == want {
        println!("ok   selftest {name} -> {rc}");
        true
    } else {
        println!("FAIL selftest {name} -> {rc} (expected {want})");
        false
    }
}

/// Same verdict as [`check_log`] but no stdout (selftest suppresses the dump).
pub(super) fn check_log_quiet(log: &Path) -> u8 {
    let text = match fs::read_to_string(log) {
        Ok(t) => t,
        Err(_) => return 3,
    };
    let pat_tagged = Pattern::regex(PAT_TAGGED).expect("PAT_TAGGED");
    let pat_mission = Pattern::regex(PAT_MISSION).expect("PAT_MISSION");
    let pat_slots = Pattern::regex(PAT_SLOTS).expect("PAT_SLOTS");
    let pat_lobby = Pattern::regex(PAT_LOBBY).expect("PAT_LOBBY");
    let pat_assigned = Pattern::regex(PAT_ASSIGNED).expect("PAT_ASSIGNED");
    let pat_errors = Pattern::regex(PAT_ERRORS).expect("PAT_ERRORS");

    let mut fail = false;
    if count_matching_lines(&pat_tagged, &text) == 0 {
        fail = true;
    }
    if !matches!(probe_str(&pat_mission, &text), Ok(true)) {
        fail = true;
    }
    if !matches!(probe_str(&pat_slots, &text), Ok(true)) {
        fail = true;
    }
    if !matches!(probe_str(&pat_lobby, &text), Ok(true)) {
        fail = true;
    }
    if matches!(probe_str(&pat_errors, &text), Ok(true)) {
        fail = true;
    }
    if fail {
        return 1;
    }
    if matches!(probe_str(&pat_assigned, &text), Ok(true)) {
        0
    } else {
        2
    }
}
