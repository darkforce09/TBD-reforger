//! `cargo xtask mcp wb-logs` — reads a Workbench Play console log and reports whether the
//! spawn pipeline actually ran.
//!
//! Four outcomes: 0 PASS · 1 FAIL · 2 PARTIAL · 3 ENVIRONMENT. Usage and bad flags return 3,
//! never 1 or 2, so a mistyped invocation cannot be read as a spawn verdict.
//!
//! An unreadable log is ENVIRONMENT (3), the same as a missing one: a log that was never
//! examined says nothing about the mod, so a read error must never collapse into a zero
//! tagged-line count and read as FAIL. The display extract prints from the same in-memory
//! text the verdict is computed over, so the dump and the verdict cannot disagree, and an
//! invalid user extract pattern is ENVIRONMENT rather than a silently empty extract.
//!
//! The loadout and assigned probes take the soft branch (note / PARTIAL) on anything other
//! than a confirmed match, including a probe that did not execute. The log vocabulary is a
//! hand-synced copy of the one in `commands::debug::remote_logs`; the two stay in step by
//! hand rather than through a shared library.

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Result;
use verification_core::gate::probe_str;
use verification_core::pattern::Pattern;

/// Hand-synced with the same vocabulary in `commands::debug::remote_logs` — EDIT BOTH.
const PAT_TAGGED: &str = r"\[TBD\]\[";
const PAT_MISSION: &str = r"\[TBD\]\[Mission\] loaded id=";
const PAT_SLOTS: &str = r"\[TBD\]\[Slots\] Slot-";
const PAT_ASSIGNED: &str = r"\[TBD\]\[Spawn\].*assigned|\[TBD\] SpawnManager: assigned";
const PAT_ERRORS: &str = r"Can.t compile|Unknown class .TBD_|RequestSpawn failed";
const PAT_LOADOUT: &str = r"\[TBD\]\[Loadout\]\[Slot\]";
const DEFAULT_EXTRACT: &str = r"\[TBD\]|SpawnLogic|assigned slot";

/// Printed on `--help` and on a `--file` with no value; both return ENVIRONMENT (3).
const USAGE: &str = "\
# cargo xtask mcp wb-logs — grep the latest Workbench Play console.log for TBD spawn
# diagnostics and assert the spawn pipeline actually ran. Run after MCP wb_play (and an
# optional sleep): enfusion-mcp has no wb_log tool, so this is the read-back half of a
# wb_play loop.
#
# Usage:
#   cargo xtask mcp wb-logs [extended-grep-pattern]   # latest Workbench log; pattern filters DISPLAY only
#   cargo xtask mcp wb-logs --file <path> [pattern]   # verdict over a specific log file (no Workbench)
#   cargo xtask mcp wb-logs --selftest                # prove the verdict logic can FAIL";

/// clap's `PathBufValueParser` rejects empty (`--file=` / `--file ''`) with rc=2. Accepting
/// empty here keeps those shapes inside this command's own ENVIRONMENT / usage exit (3).
pub fn parse_file_arg(s: &str) -> Result<PathBuf, String> {
    Ok(PathBuf::from(s))
}

/// `--file ''` prints usage; `--file=` reports an empty path as ENVIRONMENT. clap collapses
/// both to an empty value, so a following empty argv token becomes the bare-`--file` sentinel.
pub fn preprocess_cli_args(mut args: Vec<OsString>) -> Vec<OsString> {
    let Some(i) = args.iter().position(|a| a == "wb-logs") else {
        return args;
    };
    let mut j = i + 1;
    while j < args.len() {
        if args[j] == "--file" {
            if j + 1 < args.len() && args[j + 1].is_empty() {
                args[j + 1] = OsString::from("__MISSING__");
            }
            break;
        }
        if args[j].to_string_lossy().starts_with("--file=") {
            break;
        }
        j += 1;
    }
    args
}

/// Entry for `xtask mcp wb-logs`.
pub fn run(
    file: Option<PathBuf>,
    selftest: bool,
    help: bool,
    pattern: Option<String>,
) -> Result<u8> {
    if help {
        println!("{USAGE}");
        return Ok(3);
    }
    if selftest {
        return Ok(cmd_selftest());
    }
    let extract = pattern.as_deref().unwrap_or(DEFAULT_EXTRACT);
    if let Some(path) = file {
        // `__MISSING__` = bare `--file` or `--file ''` (after preprocess) → usage.
        // Empty path = `--file=` → check_log → ENVIRONMENT "no such log file: ".
        if path.as_os_str() == "__MISSING__" {
            println!("{USAGE}");
            return Ok(3);
        }
        return Ok(check_log(&path, extract));
    }
    Ok(cmd_latest(extract))
}

fn env_fail(msg: &str) -> u8 {
    eprintln!("ENVIRONMENT: {msg}");
    eprintln!("The log was never examined, so this says NOTHING about the mod.");
    3
}

fn count_matching_lines(pat: &Pattern, text: &str) -> u32 {
    text.lines().filter(|line| pat.is_match(line)).count() as u32
}

fn matching_lines<'a>(pat: &Pattern, text: &'a str) -> Vec<&'a str> {
    text.lines().filter(|line| pat.is_match(line)).collect()
}

fn check_log(log: &Path, extract: &str) -> u8 {
    let text = match fs::read_to_string(log) {
        Ok(t) => t,
        Err(_) if !log.is_file() => {
            return env_fail(&format!("no such log file: {}", log.display()));
        }
        Err(e) => {
            // An unreadable log was never examined, so it must not count as tagged=0 / FAIL.
            return env_fail(&format!("could not read log file {}: {e}", log.display()));
        }
    };

    let pat_extract = match Pattern::regex(extract) {
        Ok(p) => p,
        Err(e) => {
            return env_fail(&format!("invalid extract pattern: {e}"));
        }
    };
    let pat_tagged = Pattern::regex(PAT_TAGGED).expect("PAT_TAGGED");
    let pat_mission = Pattern::regex(PAT_MISSION).expect("PAT_MISSION");
    let pat_slots = Pattern::regex(PAT_SLOTS).expect("PAT_SLOTS");
    let pat_assigned = Pattern::regex(PAT_ASSIGNED).expect("PAT_ASSIGNED");
    let pat_errors = Pattern::regex(PAT_ERRORS).expect("PAT_ERRORS");
    let pat_loadout = Pattern::regex(PAT_LOADOUT).expect("PAT_LOADOUT");

    println!("Log: {}", log.display());
    println!("---");
    let extracted = matching_lines(&pat_extract, &text);
    let start = extracted.len().saturating_sub(60);
    for line in &extracted[start..] {
        println!("{line}");
    }
    println!("---");

    let mut fail = false;
    let tagged = count_matching_lines(&pat_tagged, &text);
    println!("[TBD][ tagged lines: {tagged}");
    if tagged == 0 {
        println!("FAIL: zero '[TBD][' subsystem-tagged lines — the current mod never logged.");
        println!(
            "      Flat '[TBD] …' lines only = a build older than subsystem tags; none at all ="
        );
        println!("      the mod is not loaded in this session. Either way the pipeline under test");
        println!("      did not run.");
        fail = true;
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
                // probe_str cannot fail today; the arm keeps the verdict honest if it ever can.
                println!("FAIL: {label} — the probe errored; the check did not execute.");
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
            println!("FAIL: the error scan errored; the check did not execute.");
            fail = true;
        }
    }

    // Soft branch: anything but a confirmed match is the note, including DidNotRun.
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

    // Soft branch: anything but a confirmed match is PARTIAL, including DidNotRun.
    match probe_str(&pat_assigned, &text) {
        Ok(true) => {
            println!("PASS: slot bodies built and a player was assigned a slot.");
            0
        }
        _ => {
            println!("PARTIAL: slot bodies built; no player has deployed yet.");
            2
        }
    }
}

fn cmd_selftest() -> u8 {
    let tmp = match tempfile_dir("tbd-wblogs-selftest") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SELFTEST: FAIL (tempdir: {e})");
            return 1;
        }
    };

    // (a) build older than subsystem tags — flat tags only. MUST fail.
    write_log(
        &tmp.join("stale.log"),
        &[
            "SCRIPT       : [TBD] Mission loaded from backend: Bridgehead at Levie",
            "SCRIPT       : [TBD] SpawnManager: built slot spawn blufor:Alpha:SL:0",
            "SCRIPT       : [TBD] Stage → LOBBY",
            "SCRIPT       : [TBD] SpawnManager: assigned slot blufor:Alpha:SL:0",
            "SCRIPT       : [TBD] SpawnManager: spawn requested",
        ],
    );

    // (b) current build, healthy, player seated. MUST pass.
    write_log(
        &tmp.join("healthy.log"),
        &[
            "SCRIPT       : [TBD][Mission] loaded id=msn_x name='N' slots=2 source=profile",
            "SCRIPT       : [TBD][Slots] Slot-1 blufor:Alpha:SL:0 (blufor:Alpha:SL:0) kit kit:rifleman_m16 at <4870, 135, 7760>",
            "SCRIPT       : [TBD][Loadout][Slot] slot=blufor:Alpha:SL:0 loadout pass complete gear=4/4 cargo=6/6",
            "SCRIPT       : [TBD][Stage] LOADING -> LOBBY",
            "SCRIPT       : [TBD] SpawnManager: assigned slot blufor:Alpha:SL:0 to player 1 at (4870,7760)",
        ],
    );

    // (c) current build, nobody joined. MUST be PARTIAL (2).
    write_log(
        &tmp.join("healthy-nojoin.log"),
        &[
            "SCRIPT       : [TBD][Mission] loaded id=msn_x name='N' slots=2 source=profile",
            "SCRIPT       : [TBD][Slots] Slot-1 blufor:Alpha:SL:0 (blufor:Alpha:SL:0) kit kit:rifleman_m16 at <4870, 135, 7760>",
            "SCRIPT       : [TBD][Stage] LOADING -> LOBBY",
        ],
    );

    // (d) mission invalid — MUST fail.
    write_log(
        &tmp.join("invalid.log"),
        &[
            "SCRIPT    (E): [TBD] Mission loaded but invalid — staying in LOADING.",
            "SCRIPT    (E): [TBD][Validate] mission result=FAIL errors=3 warnings=0",
        ],
    );

    let mut bad = false;
    bad |= !expect("stale-build-must-fail", 1, &tmp.join("stale.log"));
    bad |= !expect("healthy-with-player-passes", 0, &tmp.join("healthy.log"));
    bad |= !expect(
        "healthy-no-player-is-partial",
        2,
        &tmp.join("healthy-nojoin.log"),
    );
    bad |= !expect("mission-invalid-must-fail", 1, &tmp.join("invalid.log"));

    let _ = fs::remove_dir_all(&tmp);
    if bad {
        println!("SELFTEST: FAIL");
        1
    } else {
        println!("SELFTEST: PASS");
        0
    }
}

fn expect(name: &str, want: u8, file: &Path) -> bool {
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
fn check_log_quiet(log: &Path) -> u8 {
    let text = match fs::read_to_string(log) {
        Ok(t) => t,
        Err(_) => return 3,
    };
    let pat_tagged = Pattern::regex(PAT_TAGGED).unwrap();
    let pat_mission = Pattern::regex(PAT_MISSION).unwrap();
    let pat_slots = Pattern::regex(PAT_SLOTS).unwrap();
    let pat_assigned = Pattern::regex(PAT_ASSIGNED).unwrap();
    let pat_errors = Pattern::regex(PAT_ERRORS).unwrap();

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

fn cmd_latest(extract: &str) -> u8 {
    let home = match std::env::var_os("HOME") {
        Some(h) => PathBuf::from(h),
        None => {
            return env_fail("HOME is unset — cannot locate Workbench log directories");
        }
    };
    let proton = home.join(
        ".local/share/Steam/steamapps/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/logs",
    );
    let native = home.join("Documents/Games/ArmaReforgerWorkbench/logs");

    let Some(latest_dir) = latest_log_dir(&[&proton, &native]) else {
        return env_fail(&format!(
            "no Workbench log directory found (looked in {} and {})",
            proton.display(),
            native.display()
        ));
    };
    check_log(&latest_dir.join("console.log"), extract)
}

/// Newest `logs_*` directory, searching Proton then native — the first root with any match wins.
fn latest_log_dir(candidates: &[&Path]) -> Option<PathBuf> {
    for d in candidates {
        if !d.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(d) else {
            continue;
        };
        let mut best: Option<(SystemTime, PathBuf)> = None;
        for ent in entries.flatten() {
            let name = ent.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("logs_") {
                continue;
            }
            let path = ent.path();
            if !path.is_dir() {
                continue;
            }
            let mtime = ent
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            match &best {
                None => best = Some((mtime, path)),
                Some((t, _)) if mtime > *t => best = Some((mtime, path)),
                _ => {}
            }
        }
        if let Some((_, p)) = best {
            return Some(p);
        }
    }
    None
}

fn tempfile_dir(prefix: &str) -> Result<PathBuf> {
    let mut p = std::env::temp_dir();
    p.push(format!("{prefix}.{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p)?;
    Ok(p)
}

fn write_log(path: &Path, lines: &[&str]) {
    let mut f = fs::File::create(path).expect("create log");
    for line in lines {
        writeln!(f, "{line}").expect("write log");
    }
}

#[cfg(test)]
#[path = "tests/workbench_logs/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/workbench_logs/file_cli_tests.rs"]
mod file_cli_tests;
