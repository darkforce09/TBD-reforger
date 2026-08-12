//! Verdict + `--selftest` half of the T-892 port of `scripts/mod/world-boot.sh`.
//!
//! Split like `gate_ui_layouts` / `gate_ui_layouts_awk`: this module is the pure log triage
//! (`assess_log`) plus the offline fixture harness. It knows nothing about Steam, the API, or
//! process groups — [`crate::mod_world_boot`] owns the boot driver and is the CLI front door.
//!
//! Exit contract for assess (mirrors bash `return`):
//! - `true`  → log holds every assertion (bash `return 0`)
//! - `false` → at least one assertion failed (bash `return 1`)
//!
//! `--selftest` overall exit is owned by the driver: **0** when every good fixture passes and
//! every bad fixture is rejected. A hollow gate is one that returns `true` on a bad fixture
//! (assess exit 0 on reject-required input) — pinned by the bad-* arms below.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use regex::Regex;
use tbd_gate::pattern::Pattern;

/// Errors that are CORRECT on a bare boot (no backend / no missionId).
const EXPECTED_ERRORS: &str = r"missionId not configured|MissionList: backend not configured";

/// Structural breakage — fail whoever "owns" the text.
const HARD_FAIL: &str = r"WORLD +\(E\): Unknown class|Virtual Machine Exception|Unable to find component class|Cannot find component";

/// Fail-closed allowlist: anything else that is not TBD-owned fails.
const VANILLA_BENIGN: &str = r"needs a entity catalog manager";

const TBD_OWNED: &str = r"\[tbd\]|tbd_|/tbd/";

/// Optional mission-seeded ratchet inputs (bash globals `MISSION_ID` / `WARN_KEY`).
pub struct MissionCtx<'a> {
    pub mission_id: &'a str,
    pub warn_key: &'a str,
    pub warn_baseline: &'a Path,
}

/// Prints findings to stdout. Returns `true` iff every assertion held (bash rc 0).
pub fn assess_log(log: &Path, scenario: &str, mission: Option<MissionCtx<'_>>) -> bool {
    let text = match fs::read_to_string(log) {
        Ok(t) => t,
        Err(_) => {
            println!("  FAIL  world never loaded (no 'Starting new playthrough' for {scenario})");
            return false;
        }
    };
    assess_log_text(&text, scenario, mission)
}

/// In-memory twin of [`assess_log`].
pub fn assess_log_text(text: &str, scenario: &str, mission: Option<MissionCtx<'_>>) -> bool {
    assess_inner(text, scenario, mission, true)
}

/// Return-code only (bash `assess_log >/dev/null 2>&1`).
fn assess_quiet(text: &str, scenario: &str) -> bool {
    assess_inner(text, scenario, None, false)
}

fn assess_inner(text: &str, scenario: &str, mission: Option<MissionCtx<'_>>, print: bool) -> bool {
    let mut rc_ok = true;
    let say = |s: String| {
        if print {
            println!("{s}");
        }
    };

    let play_pat = Pattern::regex(&format!(
        "Starting new playthrough.*{}",
        regex::escape(scenario)
    ))
    .expect("playthrough");
    if !text.lines().any(|l| play_pat.is_match(l)) {
        say(format!(
            "  FAIL  world never loaded (no 'Starting new playthrough' for {scenario})"
        ));
        rc_ok = false;
    } else {
        say("  ok    world loaded".into());
    }

    let hard_pat = Pattern::regex(HARD_FAIL).expect("HARD_FAIL");
    let hard: Vec<&str> = text.lines().filter(|l| hard_pat.is_match(l)).collect();
    if !hard.is_empty() {
        say("  FAIL  engine reported structural breakage:".into());
        let stamp = Regex::new(r"^[0-9:. ]*").expect("stamp");
        let mut cleaned: Vec<String> = hard
            .iter()
            .map(|l| stamp.replace(l, "").into_owned())
            .collect();
        cleaned.sort();
        cleaned.dedup();
        for line in cleaned.into_iter().take(6) {
            say(format!("        {line}"));
        }
        rc_ok = false;
    } else {
        say("  ok    no unresolvable classes / VM exceptions".into());
    }

    let roll_re = Regex::new(r"\[TBD\] roll-call:.*").expect("roll-call");
    let mut rollcall = roll_re
        .find(text)
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();
    if rollcall.ends_with('\'') {
        rollcall.pop();
    }
    if rollcall.is_empty() {
        say("  FAIL  no roll-call line — TBD_FrameworkManager did not instantiate".into());
        rc_ok = false;
    } else if rollcall.contains("MISSING") {
        say("  FAIL  component(s) declared on TBD_GameMode.et did not instantiate:".into());
        say(format!("        {rollcall}"));
        rc_ok = false;
    } else {
        let clean = rollcall
            .split_once("roll-call: ")
            .map(|(_, rest)| rest)
            .unwrap_or(rollcall.as_str());
        say(format!("  ok    roll-call clean: {clean}"));
    }

    let script_e = Pattern::regex(r"SCRIPT +\(E\)").expect("SCRIPT (E)");
    let expected = Pattern::regex(EXPECTED_ERRORS).expect("EXPECTED_ERRORS");
    let tbd = Pattern::regex(TBD_OWNED)
        .expect("TBD_OWNED")
        .case_insensitive()
        .expect("TBD_OWNED i");
    let benign_pat = Pattern::regex(VANILLA_BENIGN).expect("VANILLA_BENIGN");

    let errors: Vec<&str> = text
        .lines()
        .filter(|l| script_e.is_match(l) && !expected.is_match(l))
        .collect();

    let mine: Vec<&str> = errors.iter().copied().filter(|l| tbd.is_match(l)).collect();
    let non_tbd: Vec<&str> = errors
        .iter()
        .copied()
        .filter(|l| !tbd.is_match(l))
        .collect();
    let benign: Vec<&str> = non_tbd
        .iter()
        .copied()
        .filter(|l| benign_pat.is_match(l))
        .collect();
    let unknown: Vec<&str> = non_tbd
        .iter()
        .copied()
        .filter(|l| !benign_pat.is_match(l) && script_e.is_match(l))
        .collect();

    if !mine.is_empty() {
        say("  FAIL  TBD script error(s) at boot:".into());
        for line in mine.iter().take(8) {
            say(format!("        {line}"));
        }
        rc_ok = false;
    } else {
        say("  ok    no TBD script errors".into());
    }

    if !unknown.is_empty() {
        say(
            "  FAIL  unrecognised script error(s) — neither TBD-owned nor a known-benign vanilla"
                .into(),
        );
        say(
            "        pattern. If genuinely vanilla and harmless, add it to VANILLA_BENIGN with a"
                .into(),
        );
        say("        reason; do NOT widen the TBD match to make it disappear.".into());
        let strip = Regex::new(r".*SCRIPT +\(E\): ").expect("strip SCRIPT");
        let mut msgs: Vec<String> = unknown
            .iter()
            .map(|l| strip.replace(l, "").into_owned())
            .collect();
        msgs.sort();
        msgs.dedup();
        for line in msgs.into_iter().take(6) {
            say(format!("        {line}"));
        }
        rc_ok = false;
    }

    if !benign.is_empty() {
        let n = benign.len();
        say(format!(
            "  note  {n} known-benign vanilla script error(s), not failing:"
        ));
        let strip = Regex::new(r".*SCRIPT +\(E\): ").expect("strip SCRIPT");
        let mut msgs: Vec<String> = benign
            .iter()
            .map(|l| strip.replace(l, "").into_owned())
            .collect();
        msgs.sort();
        msgs.dedup();
        for line in msgs.into_iter().take(4) {
            say(format!("        {line}"));
        }
    }

    if let Some(m) = mission {
        let verdict_re =
            Regex::new(r"mission result=[A-Z]+ errors=[0-9]+ warnings=[0-9]+").expect("verdict");
        let verdict = verdict_re
            .find_iter(text)
            .last()
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        if verdict.is_empty() {
            say(format!(
                "  FAIL  mission '{}' never reached the validator (no result line)",
                m.mission_id
            ));
            return false;
        }
        let errs = capture_num(&verdict, r"errors=([0-9]+)");
        let warns = capture_num(&verdict, r"warnings=([0-9]+)");

        if verdict.contains("result=PASS") && errs == Some(0) {
            say(format!("  ok    mission validated: {verdict}"));
        } else {
            say(format!("  FAIL  mission did not validate: {verdict}"));
            let val_re = Regex::new(r#"\[TBD\]\[Validate\][^"]{0,120}"#).expect("validate");
            let err_i = Pattern::regex("error")
                .expect("error")
                .case_insensitive()
                .expect("error i");
            let mut n = 0;
            for mat in val_re.find_iter(text) {
                if err_i.is_match(mat.as_str()) {
                    say(format!("        {}", mat.as_str()));
                    n += 1;
                    if n >= 6 {
                        break;
                    }
                }
            }
            rc_ok = false;
        }

        let budget = read_warn_budget(m.warn_baseline, m.warn_key);
        let warns_n = warns.unwrap_or(0);
        let base_name = m
            .warn_baseline
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(".world-boot-warning-baseline");
        match budget {
            None => {
                say(format!(
                    "  note  no warning baseline for {} (observed {warns_n}) — add: '{} {warns_n}' to {base_name}",
                    m.warn_key, m.warn_key
                ));
            }
            Some(b) if warns_n > b => {
                say(format!(
                    "  FAIL  validator warnings rose: {warns_n} > baseline {b} for {}",
                    m.warn_key
                ));
                rc_ok = false;
            }
            Some(b) if warns_n < b => {
                say(format!(
                    "  note  warnings IMPROVED ({warns_n} < baseline {b}) — tighten {base_name} to '{} {warns_n}'",
                    m.warn_key
                ));
            }
            Some(_) => {
                say(format!(
                    "  ok    validator warnings at baseline ({warns_n})"
                ));
            }
        }

        if warns_n > 0 {
            let warn_line = Regex::new(r"\[TBD\]\[Validate\] WARNING .*").expect("warn line");
            let mut shown = 0usize;
            let mut total = 0usize;
            for mat in warn_line.find_iter(text) {
                total += 1;
                if shown < 10 {
                    let rest = mat
                        .as_str()
                        .strip_prefix("[TBD][Validate] WARNING ")
                        .unwrap_or(mat.as_str());
                    say(format!("        warn  {rest}"));
                    shown += 1;
                }
            }
            if total > 10 {
                say(format!(
                    "        … and {} more (see console.log)",
                    total - 10
                ));
            }
        }
    }

    rc_ok
}

fn capture_num(s: &str, pat: &str) -> Option<u32> {
    let re = Regex::new(pat).ok()?;
    re.captures(s)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

fn read_warn_budget(baseline: &Path, key: &str) -> Option<u32> {
    let text = fs::read_to_string(baseline).ok()?;
    let re = Regex::new(&format!(r"^{}[[:space:]]+(\d+)", regex::escape(key))).ok()?;
    for line in text.lines() {
        if let Some(c) = re.captures(line) {
            return c.get(1).and_then(|m| m.as_str().parse().ok());
        }
    }
    None
}

/// Offline anti-vacuity harness (bash `--selftest` body). Returns **0** on SELFTEST OK.
///
/// Critical: every `bad-*` fixture must make assess return `false` (bash assess exit **1**).
/// If any bad fixture is accepted (assess exit 0), this returns 1 — a hollow gate.
pub fn cmd_selftest() -> u8 {
    println!("==> world-boot selftest (verdict logic must reject a bad log)");
    let t = match tempfile_dir("tbd-wb-selftest") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("selftest: could not create temp dir: {e}");
            return 1;
        }
    };
    let _guard = TempDirGuard(t.clone());

    write_fixture(
        &t,
        "good.log",
        r#"DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
SCRIPT    (E): [TBD] missionId not configured — cannot load mission.
"#,
    );
    write_fixture(
        &t,
        "bad-missing.log",
        r#"DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT    (E): string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=MISSING'
"#,
    );
    write_fixture(
        &t,
        "bad-noworld.log",
        "ENGINE       : Game successfully created.\n",
    );
    write_fixture(
        &t,
        "bad-scripterr.log",
        r#"DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
SCRIPT    (E): @"Scripts/Game/TBD/Boom.c,12": null pointer to instance
"#,
    );
    write_fixture(
        &t,
        "bad-unknown-class.log",
        r#"DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
WORLD     (E): Unknown class 'TBD_ThisComponentDoesNotExist' at offset 530(0x212)
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
"#,
    );
    write_fixture(
        &t,
        "bad-vm-exception.log",
        r#"DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
SCRIPT       : Virtual Machine Exception - Null pointer to instance in TBD_SafestartManager::Restore
"#,
    );
    write_fixture(
        &t,
        "bad-lowercase-path.log",
        r#"DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
SCRIPT    (E): @"scripts/game/tbd/Gamemode/TBD_SpawnManager.c,1400": null pointer to instance
"#,
    );
    write_fixture(
        &t,
        "bad-untagged-tbd.log",
        r#"DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
SCRIPT    (E): Instance of class TBD_SpawnManager is null
"#,
    );
    write_fixture(
        &t,
        "bad-unrecognised.log",
        r#"DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
SCRIPT    (E): Resource file worlds/SomeOther.ent not found
"#,
    );
    write_fixture(
        &t,
        "good-vanilla-noise.log",
        r#"DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf'.
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'
SCRIPT    (E): 'SCR_BaseResupplySupportStationComponent' needs a entity catalog manager!
"#,
    );

    let scen = "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf";
    let mut st: u8 = 0;

    for good in ["good", "good-vanilla-noise"] {
        println!("-- {good} (must PASS)");
        let text = fs::read_to_string(t.join(format!("{good}.log"))).unwrap_or_default();
        if assess_quiet(&text, scen) {
            println!("   PASS");
        } else {
            println!("   FAIL: rejected {good}");
            st = 1;
        }
    }
    for bad in [
        "bad-missing",
        "bad-noworld",
        "bad-scripterr",
        "bad-unknown-class",
        "bad-vm-exception",
        "bad-lowercase-path",
        "bad-untagged-tbd",
        "bad-unrecognised",
    ] {
        println!("-- {bad} (must FAIL)");
        let text = fs::read_to_string(t.join(format!("{bad}.log"))).unwrap_or_default();
        if assess_quiet(&text, scen) {
            // Hollow: assess returned true (exit 0) on a fixture that must be rejected (exit 1).
            println!("   FAIL: accepted {bad}");
            st = 1;
        } else {
            println!("   PASS (correctly rejected)");
        }
    }

    if st == 0 {
        println!("SELFTEST OK");
    } else {
        println!("SELFTEST FAILED");
    }
    st
}

fn write_fixture(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    let mut f = fs::File::create(&path).expect("fixture create");
    f.write_all(body.as_bytes()).expect("fixture write");
}

fn tempfile_dir(prefix: &str) -> std::io::Result<PathBuf> {
    let base = std::env::var_os("TMPDIR").unwrap_or_else(|| "/tmp".into());
    let path = PathBuf::from(base).join(format!("{prefix}.{}.{}", std::process::id(), uuidish()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn uuidish() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        ^ (std::process::id() as u64) << 32
}

struct TempDirGuard(PathBuf);
impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCEN: &str = "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf";

    fn good() -> String {
        format!(
            "DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{SCEN}'.\n\
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'\n\
SCRIPT    (E): [TBD] missionId not configured — cannot load mission.\n"
        )
    }

    #[test]
    fn good_log_passes() {
        assert!(assess_quiet(&good(), SCEN));
    }

    #[test]
    fn bad_missing_rejects_exit_equiv_1() {
        let log = format!(
            "DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{SCEN}'.\n\
SCRIPT    (E): string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=MISSING'\n"
        );
        assert!(
            !assess_quiet(&log, SCEN),
            "bad-missing must be rejected (assess exit 1); exit 0 = hollow"
        );
    }

    #[test]
    fn bad_unknown_class_rejects() {
        let log = format!(
            "DEFAULT      : [SaveGameManager] Starting new playthrough nr.0 '' for mission '{SCEN}'.\n\
WORLD     (E): Unknown class 'TBD_ThisComponentDoesNotExist' at offset 530(0x212)\n\
SCRIPT       : string line = '[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok'\n"
        );
        assert!(!assess_quiet(&log, SCEN));
    }

    #[test]
    fn selftest_harness_ok() {
        assert_eq!(cmd_selftest(), 0);
    }
}
