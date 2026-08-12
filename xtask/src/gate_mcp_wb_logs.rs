//! T-857 — port of `scripts/mod/mcp-wb-logs.sh` → `cargo xtask mcp wb-logs`.
//!
//! Four outcomes (preserved exactly): 0 PASS · 1 FAIL · 2 PARTIAL · 3 ENVIRONMENT.
//! Usage / bad flags go to 3 (not 1/2) so a mistype cannot read as a spawn verdict.
//!
//! Fail-opens closed vs bash:
//! - `grep -c PAT 2>/dev/null || true` on the tagged-line count collapsed a read/pattern
//!   error into `0` (stale / unloaded). We count after a successful read; an unreadable
//!   log is ENVIRONMENT (3), same as a missing file.
//! - Display extract used `grep -E … 2>/dev/null`; we print matching lines from the same
//!   in-memory text used for the verdict (no silent empty extract on a read error).
//!   An invalid user extract pattern is ENVIRONMENT (bash would hide the regex error).
//!
//! Preserved oddity: loadout / assigned probes treat any non-0 status as the soft branch
//! (note / PARTIAL), matching bash `if [ "$status" = "0" ]` — including a hypothetical
//! DidNotRun. Vocabulary is a HAND-SYNCED COPY of remote-logs; do not invent a shared lib.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Result;
use tbd_gate::gate::probe_str;
use tbd_gate::pattern::Pattern;

/// Hand-synced with `gate_remote_log_grep` / former remote-log-grep.sh — EDIT BOTH.
const PAT_TAGGED: &str = r"\[TBD\]\[";
const PAT_MISSION: &str = r"\[TBD\]\[Mission\] loaded id=";
const PAT_SLOTS: &str = r"\[TBD\]\[Slots\] Slot-";
const PAT_ASSIGNED: &str = r"\[TBD\]\[Spawn\].*assigned|\[TBD\] SpawnManager: assigned";
const PAT_ERRORS: &str = r"Can.t compile|Unknown class .TBD_|RequestSpawn failed";
const PAT_LOADOUT: &str = r"\[TBD\]\[Loadout\]\[Slot\]";
const DEFAULT_EXTRACT: &str = r"\[TBD\]|SpawnLogic|assigned slot";

const USAGE: &str = "\
# cargo xtask mcp wb-logs — grep the latest Workbench Play console.log for TBD spawn diagnostics and
# assert the spawn pipeline actually ran. Run after MCP wb_play (and optional sleep) —
# enfusion-mcp has no wb_log tool, so this is the read-back half of a wb_play loop.
#
# Usage:
#   cargo xtask mcp wb-logs [extended-grep-pattern]     # latest Workbench log; pattern filters DISPLAY only
#   cargo xtask mcp wb-logs --file <path> [pattern]     # verdict over a specific log file (no Workbench)
#   cargo xtask mcp wb-logs --selftest                  # prove the verdict logic can FAIL";

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
        // clap sentinel `__MISSING__` = bare `--file` (no path); empty = `--file=`.
        // Both map to bash usage → rc=3 (not clap's MissingValue rc=2).
        if path.as_os_str().is_empty() || path.as_os_str() == "__MISSING__" {
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
            // Closed fail-open: bash `grep -c … 2>/dev/null || true` would have treated this
            // as tagged=0 / FAIL. An unreadable log was never examined → ENVIRONMENT.
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
            "      Flat '[TBD] …' lines only = a stale (June-era) build; none at all = the mod"
        );
        println!(
            "      is not loaded in this session. Either way the pipeline under test did not run."
        );
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
                // probe_str is infallible today; keep the bash "did not execute" arm.
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

    // Soft branch: bash treats any non-0 status as the note (includes DidNotRun).
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

    // Soft branch: bash `if [ "$status" = "0" ]` — non-0 (incl. DidNotRun) → PARTIAL.
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

    // (a) stale June build — flat tags, both dead strings. MUST fail.
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
    // bash: check_log >/dev/null 2>&1 — quiet path, same exit codes.
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

/// `ls -td DIR/logs_* | head -1` over Proton then native — first dir with any match wins.
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
mod tests {
    use super::*;

    fn fixture(name: &str, lines: &[&str]) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("t857-{name}-{}", std::process::id()));
        write_log(&p, lines);
        p
    }

    #[test]
    fn selftest_pass() {
        assert_eq!(cmd_selftest(), 0);
    }

    #[test]
    fn healthy_with_player_passes() {
        let p = fixture(
            "healthy",
            &[
                "SCRIPT       : [TBD][Mission] loaded id=msn_x name='N' slots=2 source=profile",
                "SCRIPT       : [TBD][Slots] Slot-1 blufor:Alpha:SL:0 (blufor:Alpha:SL:0) kit kit:rifleman_m16 at <4870, 135, 7760>",
                "SCRIPT       : [TBD][Loadout][Slot] slot=blufor:Alpha:SL:0 loadout pass complete gear=4/4 cargo=6/6",
                "SCRIPT       : [TBD][Stage] LOADING -> LOBBY",
                "SCRIPT       : [TBD] SpawnManager: assigned slot blufor:Alpha:SL:0 to player 1 at (4870,7760)",
            ],
        );
        assert_eq!(check_log_quiet(&p), 0);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn healthy_no_player_is_partial() {
        let p = fixture(
            "nojoin",
            &[
                "SCRIPT       : [TBD][Mission] loaded id=msn_x name='N' slots=2 source=profile",
                "SCRIPT       : [TBD][Slots] Slot-1 blufor:Alpha:SL:0 (blufor:Alpha:SL:0) kit kit:rifleman_m16 at <4870, 135, 7760>",
                "SCRIPT       : [TBD][Stage] LOADING -> LOBBY",
            ],
        );
        assert_eq!(check_log_quiet(&p), 2);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn stale_fails() {
        let p = fixture(
            "stale",
            &[
                "SCRIPT       : [TBD] Mission loaded from backend: Bridgehead at Levie",
                "SCRIPT       : [TBD] SpawnManager: built slot spawn blufor:Alpha:SL:0",
                "SCRIPT       : [TBD] Stage → LOBBY",
                "SCRIPT       : [TBD] SpawnManager: assigned slot blufor:Alpha:SL:0",
                "SCRIPT       : [TBD] SpawnManager: spawn requested",
            ],
        );
        assert_eq!(check_log_quiet(&p), 1);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn missing_file_is_environment() {
        let p = PathBuf::from("/tmp/t857-no-such-log-file-ever");
        assert_eq!(check_log(&p, DEFAULT_EXTRACT), 3);
    }

    #[test]
    fn errors_present_fail() {
        let p = fixture(
            "errs",
            &[
                "SCRIPT       : [TBD][Mission] loaded id=msn_x name='N' slots=2 source=profile",
                "SCRIPT       : [TBD][Slots] Slot-1 blufor:Alpha:SL:0 (blufor:Alpha:SL:0) kit kit:rifleman_m16 at <4870, 135, 7760>",
                "SCRIPT       : Can't compile TBD_Foo",
                "SCRIPT       : [TBD] SpawnManager: assigned slot blufor:Alpha:SL:0 to player 1 at (4870,7760)",
            ],
        );
        assert_eq!(check_log_quiet(&p), 1);
        let _ = fs::remove_file(&p);
    }
}
