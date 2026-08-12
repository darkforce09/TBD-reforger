//! T-856 — spawn/equip determinism gate
//!
//! Live Workbench path lives in [`live`] (SIZE-1 split; soft cap 600). (T-853 port of `scripts/mod/tbd-spawn-determinism.sh`).
//!
//! CLI: `cargo xtask mod spawn-determinism` with bash-compatible modes `--preflight`,
//! `--selftest`, and `[N-runs] [world]`.
//!
//! `gate_probe_file` semantics come from [`tbd_gate::gate::probe_files`] (T-556 four-outcome).
//! Fail-opens closed or pinned inline where noted.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use regex::Regex;
use tbd_gate::{Pattern, gate, proc};

#[path = "gate_tbd_spawn_determinism_live.rs"]
mod live;

/// Entry: mirrors bash argv after the script name.
pub fn run(
    repo_root: &Path,
    preflight_only: bool,
    selftest: bool,
    runs: u32,
    world: &str,
) -> Result<u8> {
    if preflight_only {
        return Ok(preflight());
    }
    if selftest {
        return Ok(run_selftest());
    }
    // Live path always preflights first (bash: `preflight || exit $?`).
    let pf = preflight();
    if pf != 0 {
        return Ok(pf);
    }
    live::run_live(repo_root, runs, world)
}

pub(crate) fn wb_port() -> String {
    std::env::var("ENFUSION_WORKBENCH_PORT").unwrap_or_else(|_| "5775".into())
}

pub(crate) fn det_timeout() -> u64 {
    std::env::var("TBD_DET_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120)
}

pub(crate) fn keep_snapshots() -> bool {
    std::env::var("TBD_DET_KEEP")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// bash: `ss -tln 2>/dev/null | grep -q ":${WB_PORT} "`
///
/// FAIL-OPEN PIN: ss absent or non-zero ⇒ "not open" (bash pipeline collapses to false).
/// Reproduced deliberately — preflight then fail-fast with the actionable message.
pub(crate) fn port_open() -> bool {
    let needle = format!(":{} ", wb_port());
    match proc::Run::new("ss").arg("-tln").output() {
        Ok(o) if o.code == 0 => o.stdout.lines().any(|l| l.contains(&needle)),
        _ => false,
    }
}

fn preflight() -> u8 {
    let port = wb_port();
    if port_open() {
        println!("preflight: Workbench Net API listening on :{port}");
        return 0;
    }
    // Keep the two-space indent on continuation lines (bash heredoc contract).
    eprintln!(
        "FATAL: Workbench Net API not listening on :{port} — spawn-determinism cannot run.\n  Prerequisite: Arma Reforger Workbench with Net API enabled on this host.\n  Start Workbench (e.g. steam -applaunch 1874910), wait until :{port} is up,\n  then: cargo xtask mod spawn-determinism\n  Docs: docs/mod/SPAWN_DETERMINISM.md\n  This gate is NOT headless and is NOT part of cargo xtask ci ci-local / wave.sh gates.\n  Offline MCP (no Workbench): cargo xtask mcp selftest"
    );
    2
}

pub(crate) fn normalize(input: &str) -> String {
    // sed -E chain from the bash script, then LC_ALL=C sort -u.
    let rules: &[(Regex, &str)] = &[
        (Regex::new(r"^[0-9:.]+[[:space:]]+").unwrap(), ""),
        (
            Regex::new(r"SCRIPT[[:space:]]*(\((E|W)\))?[[:space:]]*:[[:space:]]*").unwrap(),
            "",
        ),
        (Regex::new(r"0x[0-9A-Fa-f]+").unwrap(), "0xID"),
        (Regex::new(r"ent=[^ ]+").unwrap(), "ent=ID"),
        (Regex::new(r"weapon=[^ ]+").unwrap(), "weapon=ID"),
        (Regex::new(r"<[-0-9., ]+>").unwrap(), "<POS>"),
        (Regex::new(r"\([-0-9.,]+\)").unwrap(), "(POS)"),
        (
            Regex::new(r"(feetY|surfaceY|groundDelta|yaw|Y|delta)=[-0-9.e]+").unwrap(),
            "${1}=N",
        ),
        (Regex::new(r"took: [0-9.]+ ms").unwrap(), "took: N ms"),
        (
            Regex::new(
                r"^(\[TBD\]\[Loadout\]\[[A-Za-z]+\]) ([a-z]+) (swap-skipped \(already worn\)|equip OK) (\{[0-9A-F]+\}[^ ]+).*",
            )
            .unwrap(),
            "$1 GEAR-ENSURED $2 $4",
        ),
    ];
    let mut out_lines: Vec<String> = Vec::new();
    for line in input.lines() {
        let mut s = line.to_string();
        for (re, rep) in rules {
            s = re.replace_all(&s, *rep).into_owned();
        }
        out_lines.push(s);
    }
    out_lines.sort();
    out_lines.dedup();
    let mut out = out_lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn extract(text: &str) -> String {
    let keep = Regex::new(
        r"\[TBD\]\[Spawn\]|\[TBD\]\[Slots\]|\[TBD\]\[Loadout\]|\[TBD\]\[Audit\]|\[TBD\]\[Mission\] loaded id=|\[TBD\]\[Stage\]|\[TBD\] Stage |\[TBD\] Roster|bound player|assigned slot",
    )
    .expect("extract keep");
    let drop =
        Regex::new(r"application cancelled|deployed player=|swapped area=").expect("extract drop");
    let mut out = String::new();
    for line in text.lines() {
        if keep.is_match(line) && !drop.is_match(line) {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Map [`gate::probe_files`] Err → the numeric grep status bash printed in assess_run messages.
fn probe_status_num(err: &tbd_gate::NotRun) -> i32 {
    match err {
        tbd_gate::NotRun::ToolAbsent(_) => 127,
        _ => 2,
    }
}

const HEALTHY: &str = "\
21:12:01.100 SCRIPT       : [TBD][Mission] loaded id=msn_x name='N' slots=2 source=profile\n\
21:12:01.200 SCRIPT       : [TBD][Slots] Slot-1 blufor:Alpha:SL:0 (blufor:Alpha:SL:0) kit kit:rifleman_m16 at <4870, 135, 7760>\n\
21:12:01.300 SCRIPT       : [TBD][Slots] materialized 2/2 bodies — 1 with a JSON loadout, 1 kit-only, 0 failed\n\
21:12:01.400 SCRIPT       : [TBD][Loadout][Slot] slot=blufor:Alpha:SL:0 primary equip OK {ABC}Rifle_M16A2.et\n\
21:12:01.500 SCRIPT       : [TBD][Stage] LOADING -> LOBBY\n\
21:12:02.000 SCRIPT       : [TBD] SpawnManager: assigned slot blufor:Alpha:SL:0 to player 1 at (4870,7760)\n\
21:12:02.100 SCRIPT       : [TBD] SpawnManager: bound player 1 to slot blufor:Alpha:SL:0 body (kit kit:rifleman_m16)\n\
21:12:03.000 SCRIPT       : [TBD][Audit] characters=2 bodies=2 players=1\n";

const STALE: &str = "\
SCRIPT       : [TBD] Mission loaded from backend: Bridgehead at Levie\n\
SCRIPT       : [TBD] SpawnManager: built slot spawn blufor:Alpha:SL:0\n\
SCRIPT       : [TBD] Stage → LOBBY\n\
SCRIPT       : [TBD] SpawnManager: assigned slot blufor:Alpha:SL:0\n\
SCRIPT       : [TBD] SpawnManager: spawn requested\n";

const INVALID: &str = "\
SCRIPT    (E): [TBD] Mission loaded but invalid — staying in LOADING.\n\
SCRIPT    (E): [TBD][Validate] mission result=FAIL errors=3 warnings=0\n";

fn healthy_fixture() -> String {
    // Harness overlay for broken-arm proofs (not a public CLI flag).
    if let Ok(p) = std::env::var("TBD_DET_SELFTEST_HEALTHY") {
        return fs::read_to_string(&p).unwrap_or_else(|e| {
            panic!("TBD_DET_SELFTEST_HEALTHY={p}: {e}");
        });
    }
    HEALTHY.to_string()
}

fn run_selftest() -> u8 {
    let tmp = tempfile_dir("tbd-det-selftest");
    let healthy = tmp.join("healthy.log");
    let stale = tmp.join("stale.log");
    let invalid = tmp.join("invalid.log");
    fs::write(&healthy, healthy_fixture()).expect("write healthy");
    fs::write(&stale, STALE).expect("write stale");
    fs::write(&invalid, INVALID).expect("write invalid");

    let mut st: u8 = 0;
    st |= expect_run("healthy-run-passes", 0, &healthy);
    st |= expect_run("stale-strings-must-fail", 1, &stale);
    st |= expect_run("mission-invalid-must-fail", 1, &invalid);

    let extracted_h = extract(&fs::read_to_string(&healthy).unwrap());
    if extracted_h.contains("loaded id=") {
        println!("ok   selftest extract-captures-healthy-mission-line");
    } else {
        println!(
            "FAIL selftest extract-captures-healthy-mission-line — digest is blind to mission identity again"
        );
        st = 1;
    }
    let extracted_i = extract(&fs::read_to_string(&invalid).unwrap());
    if extracted_i.contains("Mission loaded but invalid") {
        println!("FAIL selftest extract-excludes-load-failure-line — error-only capture is back");
        st = 1;
    } else {
        println!("ok   selftest extract-excludes-load-failure-line");
    }

    if st == 0 {
        println!("SELFTEST: PASS");
    } else {
        println!("SELFTEST: FAIL");
    }
    let _ = fs::remove_dir_all(&tmp);
    st
}

fn expect_run(name: &str, want: u8, file: &Path) -> u8 {
    // bash: `assess_run … >/dev/null 2>&1` — only the ok/FAIL line is user-visible.
    let rc = assess_run_silent(file, "selftest");
    if rc == want {
        println!("ok   selftest {name} -> {rc}");
        0
    } else {
        println!("FAIL selftest {name} -> {rc} (expected {want})");
        1
    }
}

fn assess_run_silent(raw: &Path, label: &str) -> u8 {
    assess_run_to(raw, label, &mut std::io::sink())
}

fn assess_run_to(raw: &Path, label: &str, out: &mut dyn Write) -> u8 {
    let mut rfail: u8 = 0;
    let fallthrough = Pattern::literal("path=vanilla-fallthrough");
    match gate::probe_files(&fallthrough, &[raw]) {
        Ok(true) => {
            let _ = writeln!(out, "FAIL run {label}: vanilla fall-through");
            rfail = 1;
        }
        Ok(false) => {}
        Err(e) => {
            let _ = writeln!(
                out,
                "FAIL run {label}: vanilla fall-through check did not execute (grep exited {} on {})",
                probe_status_num(&e),
                raw.display()
            );
            rfail = 1;
        }
    }

    let script_err =
        Pattern::regex(r"SCRIPT[[:space:]]*\(E\)|Virtual Machine Exception").expect("script_err");
    match gate::probe_files(&script_err, &[raw]) {
        Ok(true) => {
            let _ = writeln!(out, "FAIL run {label}: script error lines:");
            if let Ok(text) = fs::read_to_string(raw) {
                for (i, line) in text.lines().filter(|l| script_err.is_match(l)).enumerate() {
                    if i >= 5 {
                        break;
                    }
                    let _ = writeln!(out, "{line}");
                }
            }
            rfail = 1;
        }
        Ok(false) => {}
        Err(e) => {
            let _ = writeln!(
                out,
                "FAIL run {label}: script-error check did not execute (grep exited {} on {})",
                probe_status_num(&e),
                raw.display()
            );
            rfail = 1;
        }
    }

    let text = match fs::read_to_string(raw) {
        Ok(t) => t,
        Err(_) => {
            let _ = writeln!(
                out,
                "FAIL run {label}: could not read raw log {}",
                raw.display()
            );
            return 1;
        }
    };

    let churn = text
        .lines()
        .filter(|l| l.contains("has switched from faction"))
        .count();
    if churn > 3 {
        let _ = writeln!(
            out,
            "FAIL run {label}: faction churn ({churn} switch lines)"
        );
        rfail = 1;
    }
    if let Some(pos) = text.find("[TBD][Audit]")
        && text[pos..].contains("has switched from faction")
    {
        let _ = writeln!(
            out,
            "FAIL run {label}: faction switch AFTER census — churn loop alive"
        );
        rfail = 1;
    }

    let bind_re = Regex::new(r"bound player [0-9]+").expect("bind");
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for m in bind_re.find_iter(&text) {
        *counts.entry(m.as_str().to_string()).or_default() += 1;
    }
    let dups: Vec<_> = counts
        .iter()
        .filter(|(_, c)| **c > 1)
        .map(|(k, c)| format!("{c:>7} {k}"))
        .take(3)
        .collect();
    if !dups.is_empty() {
        let _ = writeln!(out, "FAIL run {label}: duplicate binds:");
        for d in &dups {
            let _ = writeln!(out, "{d}");
        }
        rfail = 1;
    }

    let bad_gear_re = Regex::new(r"\[TBD\]\[Loadout\].*(FAILED|not worn)").expect("bad_gear");
    let bad: Vec<_> = text
        .lines()
        .filter(|l| bad_gear_re.is_match(l))
        .take(3)
        .collect();
    if !bad.is_empty() {
        let _ = writeln!(out, "FAIL run {label}: gear failures:");
        for b in &bad {
            let _ = writeln!(out, "{b}");
        }
        rfail = 1;
    }

    let census_re = Regex::new(r"\[TBD\]\[Audit\] characters=[0-9]+ bodies=[0-9]+ players=[0-9]+")
        .expect("census");
    // bash: grep -oE '…[TBD][Audit]…' | tail -1 — extract match only, not the full log line
    let census = census_re
        .find_iter(&text)
        .map(|m| m.as_str().to_string())
        .last();
    match census {
        Some(census) => {
            let c = Regex::new(r"characters=([0-9]+)")
                .unwrap()
                .captures(&census)
                .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
                .unwrap_or_default();
            let b = Regex::new(r"bodies=([0-9]+)")
                .unwrap()
                .captures(&census)
                .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
                .unwrap_or_default();
            if c != b {
                let _ = writeln!(
                    out,
                    "FAIL run {label}: census mismatch {census} (stray/missing bodies?)"
                );
                rfail = 1;
            }
        }
        None => {
            let _ = writeln!(out, "FAIL run {label}: no census line");
            rfail = 1;
        }
    }

    let mat = Pattern::regex(r"\[TBD\]\[Slots\] materialized [1-9]").expect("mat");
    if !mat.is_match(&text) {
        let _ = writeln!(out, "FAIL run {label}: no materialization line");
        rfail = 1;
    }
    rfail
}

// Live assess_run prints to stdout (operator-facing during Workbench runs).
pub(crate) fn assess_run(raw: &Path, label: &str) -> u8 {
    assess_run_to(raw, label, &mut std::io::stdout())
}

pub(crate) fn tempfile_dir(prefix: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "{prefix}.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&p).expect("tmpdir");
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_keeps_healthy_mission_line() {
        assert!(extract(HEALTHY).contains("loaded id="));
    }

    #[test]
    fn extract_drops_mission_load_failure() {
        assert!(!extract(INVALID).contains("Mission loaded but invalid"));
    }

    #[test]
    fn assess_healthy_passes() {
        let d = tempfile_dir("tbd-det-test-h");
        let p = d.join("h.log");
        fs::write(&p, HEALTHY).unwrap();
        assert_eq!(assess_run_silent(&p, "t"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn assess_stale_fails() {
        let d = tempfile_dir("tbd-det-test-s");
        let p = d.join("s.log");
        fs::write(&p, STALE).unwrap();
        assert_eq!(assess_run_silent(&p, "t"), 1);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn assess_fallthrough_fails() {
        let d = tempfile_dir("tbd-det-test-f");
        let p = d.join("f.log");
        let mut body = HEALTHY.to_string();
        body.push_str("SCRIPT       : path=vanilla-fallthrough\n");
        fs::write(&p, body).unwrap();
        assert_eq!(assess_run_silent(&p, "t"), 1);
        let _ = fs::remove_dir_all(&d);
    }

    /// Wave-218 REJECT pin: census mismatch must print grep -oE extract, not the full log line.
    #[test]
    fn assess_census_mismatch_extracts_audit_only() {
        let d = tempfile_dir("tbd-det-test-census");
        let p = d.join("c.log");
        let mut body = HEALTHY.to_string();
        body = body.replace(
            "[TBD][Audit] characters=2 bodies=2 players=1",
            "[TBD][Audit] characters=5 bodies=2 players=1",
        );
        fs::write(&p, &body).unwrap();
        let mut buf = Vec::new();
        assert_eq!(assess_run_to(&p, "novel", &mut buf), 1);
        let out = String::from_utf8(buf).unwrap();
        assert!(
            out.contains(
                "FAIL run novel: census mismatch [TBD][Audit] characters=5 bodies=2 players=1 (stray/missing bodies?)"
            ),
            "got: {out:?}"
        );
        assert!(
            !out.contains("21:12:03.000"),
            "timestamp prefix leaked into census mismatch: {out:?}"
        );
        let _ = fs::remove_dir_all(&d);
    }

    /// Wave-218 REJECT pin: duplicate binds must use GNU uniq -c 7-wide count field.
    #[test]
    fn assess_duplicate_binds_uniq_c_padding() {
        let d = tempfile_dir("tbd-det-test-dup");
        let p = d.join("d.log");
        let mut body = HEALTHY.to_string();
        body = body.replace(
            "bound player 1 to slot blufor:Alpha:SL:0 body (kit kit:rifleman_m16)\n",
            "bound player 1 to slot blufor:Alpha:SL:0 body (kit kit:rifleman_m16)\n21:12:02.200 SCRIPT       : [TBD] SpawnManager: bound player 1 to slot blufor:Alpha:SL:0 body (kit kit:rifleman_m16)\n",
        );
        fs::write(&p, &body).unwrap();
        let mut buf = Vec::new();
        assert_eq!(assess_run_to(&p, "novel", &mut buf), 1);
        let out = String::from_utf8(buf).unwrap();
        assert!(
            out.contains("FAIL run novel: duplicate binds:\n      2 bound player 1\n"),
            "uniq -c padding mismatch, got: {out:?}"
        );
        let _ = fs::remove_dir_all(&d);
    }
}
