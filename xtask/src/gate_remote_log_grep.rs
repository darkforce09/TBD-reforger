//! T-855 — port of `scripts/mod/remote-log-grep.sh` → `cargo xtask mod remote-logs`.
//!
//! Four outcomes (preserved exactly): 0 HEALTHY · 1 FAIL · 2 PARTIAL · 3 ENVIRONMENT.
//! Usage / bad flags go to 3 (not 2) so a mistype cannot read as "booted, nobody joined".
//!
//! Fail-opens closed vs bash:
//! - `grep -c PAT 2>/dev/null || true` on the tagged-line count collapsed a read/pattern
//!   error into `0` (STALE BUILD). We count after a successful read; an unreadable log is
//!   ENVIRONMENT (3), same as a missing file.
//! - Display / error extract used `2>/dev/null`; we print matching lines from the same
//!   in-memory text used for the verdict (no silent empty extract on a read error).
//!
//! Preserved oddity: unset `TBD_SSH_HOST` after optional `deploy.env` source exits **1**
//! (bash `${VAR:?…}`), not ENVIRONMENT 3 — pin that, do not "fix" it to 3.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tbd_gate::NotRun;
use tbd_gate::gate::probe_str;
use tbd_gate::pattern::Pattern;
use tbd_gate::proc::{self, Run};

use crate::root::find_repo_root;

const PAT_TAGGED: &str = r"\[TBD\]\[";
const PAT_MISSION: &str = r"\[TBD\]\[Mission\] loaded id=";
const PAT_SLOTS: &str = r"\[TBD\]\[Slots\] Slot-";
const PAT_LOBBY: &str = r"\[TBD\]\[Stage\].*LOBBY|\[TBD\] Stage .*LOBBY";
const PAT_LOADOUT: &str = r"\[TBD\]\[Loadout\]\[Slot\]";
const PAT_ASSIGNED: &str = r"\[TBD\]\[Spawn\].*assigned|\[TBD\] SpawnManager: assigned";
const PAT_ERRORS: &str = r"Can.t compile|Unknown class .TBD_|RequestSpawn failed";
const PAT_EXTRACT: &str = r"\[TBD\]|assigned slot|Can.t compile|RequestSpawn failed|Unknown class";

/// Entry for `xtask mod remote-logs`.
pub fn run(file: Option<PathBuf>, selftest: bool) -> Result<u8> {
    if selftest {
        return Ok(cmd_selftest());
    }
    if let Some(path) = file {
        return Ok(check_log(&path));
    }
    cmd_remote()
}

fn min_tagged() -> u32 {
    std::env::var("TBD_MIN_TAGGED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20)
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

/// Local verdict over one log file. Remote path fetches once, then calls this.
fn check_log(log: &Path) -> u8 {
    let text = match fs::read_to_string(log) {
        Ok(t) => t,
        Err(_) if !log.is_file() => {
            return env_fail(&format!("no such log file: {}", log.display()));
        }
        Err(e) => {
            // Closed fail-open: bash `grep -c … 2>/dev/null || true` would have treated this
            // as tagged=0 / STALE. An unreadable log was never examined → ENVIRONMENT.
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
        println!(
            "      or use scripts/mod/run-playtest-server.sh which asserts the local addon won."
        );
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

fn cmd_selftest() -> u8 {
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
    let pat_lobby = Pattern::regex(PAT_LOBBY).unwrap();
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

fn cmd_remote() -> Result<u8> {
    let root = find_repo_root()?;
    let env_file = root.join("scripts/deploy/deploy.env");
    let mut host = std::env::var("TBD_SSH_HOST").ok();
    let mut profile = std::env::var("TBD_PROFILE_DIR").ok();
    let mut ssh_pass = std::env::var("TBD_SSH_PASS").ok();
    let mut ssh_ident = std::env::var("TBD_SSH_IDENTITY_FILE").ok();

    if env_file.is_file() {
        let parsed = parse_deploy_env(&env_file)?;
        host = host.or_else(|| parsed.get("TBD_SSH_HOST").cloned());
        profile = profile.or_else(|| parsed.get("TBD_PROFILE_DIR").cloned());
        ssh_pass = ssh_pass.or_else(|| parsed.get("TBD_SSH_PASS").cloned());
        ssh_ident = ssh_ident.or_else(|| parsed.get("TBD_SSH_IDENTITY_FILE").cloned());
    }

    // Preserved oddity: bash `: "${TBD_SSH_HOST:?…}"` exits 1, not ENVIRONMENT 3.
    let host = match host.filter(|s| !s.is_empty()) {
        Some(h) => h,
        None => {
            eprintln!(
                "scripts/mod/remote-log-grep.sh: TBD_SSH_HOST: Set TBD_SSH_HOST in scripts/deploy/deploy.env"
            );
            return Ok(1);
        }
    };
    let profile = match profile.filter(|s| !s.is_empty()) {
        Some(p) => p,
        None => {
            eprintln!(
                "scripts/mod/remote-log-grep.sh: TBD_PROFILE_DIR: Set TBD_PROFILE_DIR in scripts/deploy/deploy.env"
            );
            return Ok(1);
        }
    };

    let find_log = format!(
        "\nls -td '{profile}'/logs/logs_* '{profile}'/profile/logs/logs_* 2>/dev/null | while read -r d; do\n  [ -f \"$d/console.log\" ] && echo \"$d/console.log\" && exit 0\ndone\nexit 1\n"
    );

    let remote_log = match ssh_cmd(
        &host,
        ssh_pass.as_deref(),
        ssh_ident.as_deref(),
        &["bash", "-lc", &shell_quote(&find_log)],
    ) {
        Ok(out) if out.code == 0 => out.stdout.trim().to_string(),
        // bash: `REMOTE_LOG="$(ssh_cmd … 2>/dev/null || true)"` — any SSH failure → empty → env_fail.
        // Closed: we do not treat SSH failure as a log verdict; ENVIRONMENT below.
        _ => String::new(),
    };

    if remote_log.is_empty() {
        return Ok(env_fail(&format!(
            "no console.log found under {profile} (logs/ or profile/logs/) on {host}"
        )));
    }

    let local_copy = {
        let mut p = std::env::temp_dir();
        p.push(format!("tbd-remote-log.{}", std::process::id()));
        p
    };

    let cat = ssh_cmd(
        &host,
        ssh_pass.as_deref(),
        ssh_ident.as_deref(),
        &["cat", &remote_log],
    );
    match cat {
        Ok(out) if out.code == 0 => {
            if out.stdout.is_empty() {
                let _ = fs::remove_file(&local_copy);
                return Ok(env_fail(&format!("{remote_log} on {host} is empty")));
            }
            fs::write(&local_copy, &out.stdout).context("write local log copy")?;
        }
        _ => {
            let _ = fs::remove_file(&local_copy);
            return Ok(env_fail(&format!(
                "could not read {remote_log} from {host}"
            )));
        }
    }

    println!("Remote log: {host}:{remote_log}");
    let rc = check_log(&local_copy);
    let _ = fs::remove_file(&local_copy);
    Ok(rc)
}

struct SshOut {
    code: i32,
    stdout: String,
}

fn ssh_cmd(
    host: &str,
    pass: Option<&str>,
    ident: Option<&str>,
    remote_args: &[&str],
) -> Result<SshOut, NotRun> {
    let mut args: Vec<String> = Vec::new();
    let program;
    if let Some(p) = pass.filter(|s| !s.is_empty()) {
        program = "sshpass".to_string();
        args.push("-p".into());
        args.push(p.into());
        args.push("ssh".into());
        args.push("-o".into());
        args.push("StrictHostKeyChecking=no".into());
        args.push(host.into());
        for a in remote_args {
            args.push((*a).into());
        }
    } else if let Some(id) = ident.filter(|s| !s.is_empty()) {
        program = "ssh".to_string();
        args.push("-i".into());
        args.push(id.into());
        args.push("-o".into());
        args.push("StrictHostKeyChecking=no".into());
        args.push(host.into());
        for a in remote_args {
            args.push((*a).into());
        }
    } else {
        program = "ssh".to_string();
        args.push("-o".into());
        args.push("StrictHostKeyChecking=no".into());
        args.push(host.into());
        for a in remote_args {
            args.push((*a).into());
        }
    }

    // Prefer tbd-gate Run so ToolAbsent / Signalled stay NotRun (not a FAIL verdict).
    let _ = proc::which(&program)?;
    let mut run = Run::new(&program);
    for a in &args {
        run = run.arg(a);
    }
    let out = run.output()?;
    Ok(SshOut {
        code: out.code,
        stdout: out.stdout,
    })
}

fn shell_quote(s: &str) -> String {
    // bash printf '%q' for the FIND_LOG payload — enough for our fixed string (no single quotes).
    if s.bytes().all(|b| {
        b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'/' | b'.' | b'_' | b'-' | b'=' | b':' | b'@' | b'+' | b','
            )
    }) {
        return s.to_string();
    }
    let mut out = String::from("'");
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn parse_deploy_env(path: &Path) -> Result<std::collections::HashMap<String, String>> {
    let mut map = std::collections::HashMap::new();
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
            map.insert(k.trim().to_string(), v);
        }
    }
    Ok(map)
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

fn append_line(path: &Path, line: &str) {
    let mut f = fs::OpenOptions::new()
        .append(true)
        .open(path)
        .expect("append log");
    writeln!(f, "{line}").expect("append");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str, lines: &[&str]) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("t855-{name}-{}", std::process::id()));
        write_log(&p, lines);
        p
    }

    #[test]
    fn selftest_pass() {
        assert_eq!(cmd_selftest(), 0);
    }

    #[test]
    fn healthy_is_partial() {
        let p = fixture(
            "healthy",
            &[
                "SCRIPT : [TBD][Mission] loaded id=msn_x name='N' slots=7 source=profile",
                "SCRIPT : [TBD][Slots] Slot-1 s (a:b:c:0) kit kit:x at <1, 2, 3>",
                "SCRIPT : [TBD][Loadout][Slot] slot=a:b:c:0 loadout pass complete gear=1/1 cargo=0/0",
                "SCRIPT : [TBD][Stage] LOADING -> LOBBY",
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
                "SCRIPT : [TBD] Mission loaded from backend: something",
                "SCRIPT : [TBD] SpawnManager: built slot spawn",
                "SCRIPT : [TBD] Stage → LOBBY",
            ],
        );
        assert_eq!(check_log_quiet(&p), 1);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn missing_file_is_environment() {
        let p = PathBuf::from("/tmp/t855-no-such-log-file-ever");
        assert_eq!(check_log(&p), 3);
    }

    #[test]
    fn errors_present_fail() {
        let p = fixture(
            "errs",
            &[
                "SCRIPT : [TBD][Mission] loaded id=msn_x name='N' slots=7 source=profile",
                "SCRIPT : [TBD][Slots] Slot-1 s (a:b:c:0) kit kit:x at <1, 2, 3>",
                "SCRIPT : [TBD][Stage] LOADING -> LOBBY",
                "SCRIPT : Can't compile SomeClass",
            ],
        );
        assert_eq!(check_log_quiet(&p), 1);
        let _ = fs::remove_file(&p);
    }
}
