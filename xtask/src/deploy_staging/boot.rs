//! T-607 — THE BOOT VERDICT (bash lines 681–1070).
//!
//! ── WHAT WAS BROKEN ──────────────────────────────────────────────────────────────────────────
//!
//! This script built two ExecStarts and neither gave a server that was both correct and joinable:
//!
//! | mode | flags | outcome |
//! |------|-------|---------|
//! | config | `-config`, no `-addonsDir` | registers a room, resolves the mod from the WORKSHOP — not from the checkout it just rsynced |
//! | addons | `-addonsDir` + `-addons` + `-server` | loads the checkout, registers NO backend room |
//!
//! The first is the expensive one and it is this program's signature defect wearing the engine's
//! clothes: **staging was validating a build it did not deploy.** `tbd-framework` is published
//! unlisted under the SAME id as the local gproj GUID (`B2C3D4E5F6A78901`), so `game.mods[]` is
//! satisfiable from the Workshop and the engine quietly does that. The deploy rsyncs a checkout to
//! the host, symlinks it into `$TBD_ADDONS_STAGING`, and then launches a server that never looks
//! at it. Every "staging is green" verdict since the June publish was a true statement about the
//! WRONG code.
//!
//! THE FIX is T-604's: `-addonsDir <dir>` **plus** `-config <json>` does both at once. The
//! 2026-06-14 "mutually exclusive" finding was measured on `-addons`, which really is fatal with
//! `-config`; `-addonsDir` is a different flag and combines with it fine.
//!
//! ⚠ THE FORMAT CHECK NO LONGER DISCRIMINATES HERE. `cargo xtask mod remote-logs` separates builds
//! by counting `[TBD][` lines — stale Workshop 1.0.1 emits 0, any current build emits many. That
//! was sound while the Workshop copy was June's. The operator re-published on 2026-07-31, so the
//! Workshop now serves **1.0.2**, which is current-format: measured on a real `-config`-only boot
//! (2026-08-01 00:12, profile pak 570,489 B) that log carries **154** `[TBD][` lines and would
//! sail through the format threshold while running a pak the deploy never produced. The format
//! check answers "is this build ancient", which is a different question from "is this build the
//! one I just deployed". Only the gproj PATH answers the second. Do not "simplify" it into one —
//! [`selftest`] asserts the non-redundancy as an executable statement.
//!
//! Pure functions over a log FILE on purpose: the deploy half needs ssh and a live host, and a
//! check that can only run during a real deploy is a check nobody runs. `--verify-boot` and
//! `--verify-boot-selftest` exercise every line of this logic with no ssh, no `deploy.env` and no
//! staging host.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use regex::Regex;

use super::Paths;

/// Output sink. `None` writes through to the real streams; `Some(buf)` accumulates BOTH streams
/// into one string, which is what the bash's `out="$(verify_boot_log … 2>&1)"` did.
///
/// This exists so [`selftest`] can assert on the reporter's own text without re-executing the
/// process. The bash could only do that by shelling out to itself; capturing here keeps the
/// verdict a pure function of (log, guid, addons_dir, admin_count, profile_dir, rival_bytes).
pub struct Out {
    buf: Option<String>,
}

impl Out {
    pub fn streams() -> Out {
        Out { buf: None }
    }
    pub fn captured() -> Out {
        Out {
            buf: Some(String::new()),
        }
    }
    pub fn text(&self) -> &str {
        self.buf.as_deref().unwrap_or("")
    }
    /// stdout
    fn o(&mut self, line: impl AsRef<str>) {
        match self.buf {
            Some(ref mut b) => {
                let _ = writeln!(b, "{}", line.as_ref());
            }
            None => println!("{}", line.as_ref()),
        }
    }
    /// stderr
    fn e(&mut self, line: impl AsRef<str>) {
        match self.buf {
            Some(ref mut b) => {
                let _ = writeln!(b, "{}", line.as_ref());
            }
            None => eprintln!("{}", line.as_ref()),
        }
    }
}

/// `read_addon_guid` — the addon GUID, read from the gproj rather than trusted from `deploy.env`.
///
/// A literal drifts from the source silently; the playtest runner and world-boot both read it the
/// same way. Bash was
/// `grep -oE '^[[:space:]]*GUID[[:space:]]+"[0-9A-Fa-f]+"' | grep -oE '[0-9A-Fa-f]{8,}' | head -1`.
///
/// ODDITY PRESERVED: a gproj that exists but carries no GUID line yields `Some("")`, not `None` —
/// the bash pipeline's exit status came from `head`, which succeeds over empty input. Only a
/// MISSING gproj was a failure. The caller's `|| echo <default>` therefore fires for the missing
/// file and NOT for the malformed one, and that asymmetry is load-bearing at the cross-check in
/// `config.rs`: an empty guid there compares unequal to `TBD_ADDON_GUID` only if the gproj was
/// readable, which is the case worth aborting on.
pub fn read_addon_guid(mono_root: &Path) -> Option<String> {
    let gproj = mono_root.join("apps/mod/tbd-framework/addon.gproj");
    let text = fs::read_to_string(&gproj).ok()?;
    let line_re = Regex::new(r#"(?m)^[[:space:]]*GUID[[:space:]]+"[0-9A-Fa-f]+""#).expect("static");
    let hex_re = Regex::new("[0-9A-Fa-f]{8,}").expect("static");
    for m in line_re.find_iter(&text) {
        if let Some(hex) = hex_re.find(m.as_str()) {
            return Some(hex.as_str().to_string());
        }
    }
    Some(String::new())
}

/// `grep -A8 <needle>` over `text`, returning the matched lines and their 8 followers.
///
/// GNU grep resets the after-context window on every new match and merges overlapping windows;
/// both fall out of the single counter below. The `--` group separators grep emits between
/// non-adjacent blocks are NOT produced, because the only consumer greps the result again and
/// `--` never matched `guid: '…'`.
fn grep_after(text: &str, needle: &str, after: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut remaining = 0usize;
    for line in text.lines() {
        if line.contains(needle) {
            out.push(line.to_string());
            remaining = after;
        } else if remaining > 0 {
            out.push(line.to_string());
            remaining -= 1;
        }
    }
    out
}

fn read_log(log: &Path) -> Option<String> {
    if !log.is_file() {
        return None;
    }
    fs::read_to_string(log).ok()
}

/// THE HARD GATE. Did the addon we deployed win, or a packed copy from the Workshop?
///
/// The discriminator is the gproj path the engine reports under `Loaded addons:` for OUR guid:
///
/// ```text
/// deployed checkout won   gproj: '<addonsDir>/tbd-framework/addon.gproj' guid: '<GUID>'
/// Workshop copy won       gproj: '<profile>/addons/TBDFramework_<GUID>/addon.gproj' guid: '<GUID>'
/// ```
///
/// Both are `guid: '<GUID>'` and both look healthy. Only the path differs, so only the path is
/// checked. The LAST block wins: the engine prints `Loaded addons:` more than once per boot (once
/// before and once after addon resolution — measured, 2 blocks on both a passing and a failing
/// log) and the final one is the one that ran.
pub fn assert_local_addon_won(out: &mut Out, log: &Path, guid: &str, addons_dir: &str) -> i32 {
    let want = format!("{addons_dir}/tbd-framework/addon.gproj");

    let Some(text) = read_log(log) else {
        out.e(format!("FAIL: boot log not found: {}", log.display()));
        out.e("      The check did NOT run. This is not a pass.");
        return 1;
    };

    // -F in the bash: the path may contain regex metacharacters, and `guid: '...'` is a literal.
    let needle = format!("guid: '{guid}'");
    // `| tail -1`: the LAST matching line, which is the last `Loaded addons:` block's entry.
    let loaded = grep_after(&text, "Loaded addons:", 8)
        .into_iter()
        .rfind(|l| l.contains(&needle));

    let Some(loaded) = loaded else {
        out.e(format!(
            "FAIL: the engine never reported loading addon {guid} at all."
        ));
        out.e("      Neither copy won, so the mod is simply not running.");
        // `grep -nE "Loaded addons:|gproj:" "$log" | head -20 >&2`
        let re = Regex::new("Loaded addons:|gproj:").expect("static");
        for (n, line) in text
            .lines()
            .enumerate()
            .filter(|(_, l)| re.is_match(l))
            .take(20)
        {
            out.e(format!("{}:{}", n + 1, line));
        }
        return 1;
    };

    if loaded.contains(&want) {
        out.o(format!("  PASS  deployed checkout won: {want}"));
        return 0;
    }

    out.e("FAIL: STAGING IS VALIDATING A BUILD IT DID NOT DEPLOY.");
    out.e(format!("  loaded: {}", loaded.trim_start()));
    out.e(format!("  wanted: {want}"));
    out.e("");
    out.e("  tbd-framework is published to the Workshop unlisted under the SAME id as the local");
    out.e("  gproj GUID, so the engine can satisfy game.mods[] without ever reading the checkout");
    out.e("  this deploy just rsynced. Every log line after this one would be a true statement");
    out.e("  about the wrong code.");
    out.e("");
    out.e("  Cause is almost always a missing -addonsDir on the ExecStart. Check the unit:");
    out.e("      systemctl --user cat tbd-reforger.service | grep ExecStart");
    out.e("  It must carry BOTH -addonsDir and -config (T-604).");
    1
}

/// The other half of the ticket: addons mode loads the right code and registers no room, so a
/// server can be running the correct build and still be unjoinable. Assert the room, by the
/// engine's own line, not by inference from a healthy-looking log.
pub fn assert_room_registered(out: &mut Out, log: &Path) -> i32 {
    let Some(text) = read_log(log) else {
        out.e(format!("FAIL: boot log not found: {}", log.display()));
        return 1;
    };
    // `grep -F 'Server registered with address:' | tail -1` — the LAST registration wins, because
    // a unit that restarted mid-log registers more than once and only the final one is live.
    let reg = text
        .lines()
        .rfind(|l| l.contains("Server registered with address:"));
    let Some(reg) = reg else {
        out.e("FAIL: no backend room registered — the server is NOT joinable.");
        out.e(format!(
            "      Zero 'Server registered with address:' lines in {}.",
            log.display()
        ));
        out.e(
            "      A healthy log is not a joinable server: -addonsDir + -addons + -server reaches",
        );
        out.e("      LOBBY with the mod loaded and never registers a room. Direct Join answers");
        out.e("      'No server found'. Joinable needs -config, alongside -addonsDir (T-604).");
        return 1;
    };
    // `sed 's/.*Server registered/Server registered/'` — drop the engine's timestamp prefix.
    let trimmed = match reg.rfind("Server registered") {
        Some(i) => &reg[i..],
        None => reg,
    };
    out.o(format!("  PASS  backend room registered: {trimmed}"));
    0
}

/// `#tbd` resolves admins from vanilla's `SCR_PlayerListedAdminManagerComponent`, which is
/// populated ONLY from `game.admins[]` in the server config — `TBD_AdminService.IsAdmin()` defers
/// to it. addons mode has no config at all, so it can never have an admin; that is the second half
/// of "the two modes break different halves of the acceptance criteria". `passwordAdmin` is a
/// DIFFERENT mechanism and does not feed that list.
///
/// What a log CAN prove is that the engine accepted the config carrying them. Whether a given id
/// maps to the human who connects is only observable when they connect, and this says so rather
/// than implying otherwise.
pub fn assert_admins_configured(out: &mut Out, log: &Path, want_count: &str) -> i32 {
    let Some(text) = read_log(log) else {
        out.e(format!("FAIL: boot log not found: {}", log.display()));
        return 1;
    };
    if !text.contains("Server config loaded.") {
        out.e("FAIL: the engine never loaded a server config — game.admins[] cannot exist.");
        out.e(
            "      '#tbd' will answer 'TBD: admin only.' for everyone, whatever deploy.env says.",
        );
        return 1;
    }
    if !text.contains("JSON is Valid") {
        out.e("FAIL: the engine did not report the server config as schema-valid.");
        let re = Regex::new("JSON Schema Validation|RegEx Pattern|errors in server config")
            .expect("static");
        for (n, line) in text
            .lines()
            .enumerate()
            .filter(|(_, l)| re.is_match(l))
            .take(10)
        {
            out.e(format!("{}:{}", n + 1, line));
        }
        return 1;
    }
    // ODDITY PRESERVED: bash `[ "$want_count" -eq 0 ]` over a NON-NUMERIC value errors, `[`
    // returns 2, and because it sits in an `if` CONDITION `set -e` does not fire — so the else
    // branch runs and the junk value is printed verbatim in the PASS line. `unwrap_or(1)` below
    // reproduces exactly that: unparseable is treated as "not zero".
    if want_count.trim().parse::<i64>().unwrap_or(1) == 0 {
        out.o(
            "  WARN  config accepted, but game.admins[] is EMPTY (TBD_ADMIN_IDENTITY_IDS unset).",
        );
        out.o("        Every '#tbd' command will answer 'TBD: admin only.' Set it in deploy.env.");
        return 0;
    }
    out.o(format!(
        "  PASS  server config accepted by the engine, carrying {want_count} admin id(s)"
    ));
    out.o("        (that the ENGINE took them; whether an id is the human who connects is only");
    out.o("         observable when they connect — check '#tbd' in chat)");
    0
}

/// The whole verdict over one log. Returns 1 if any half failed.
///
/// `profile_dir` is the `-profile` dir, so the rival check can look at the DISK and not just the
/// log. `rival_bytes` is a rival pak size ALREADY MEASURED by the caller: the deploy path uses it
/// because `profile_dir` there is a path on the STAGING HOST, and a local `[ -f ]` against a remote
/// path silently answers "absent" — which would downgrade a real contest to WEAK EVIDENCE on every
/// real deploy. `None` = not measured, `Some("0")` = measured and absent.
#[allow(clippy::too_many_arguments)]
pub fn verify_boot_log(
    out: &mut Out,
    log: &Path,
    guid: &str,
    addons_dir: &str,
    admin_count: &str,
    profile_dir: &str,
    rival_bytes: Option<&str>,
) -> i32 {
    let mut rc = 0;
    out.o(format!("==> boot verdict: {}", log.display()));
    if assert_local_addon_won(out, log, guid, addons_dir) != 0 {
        rc = 1;
    }
    if assert_room_registered(out, log) != 0 {
        rc = 1;
    }
    if assert_admins_configured(out, log, admin_count) != 0 {
        rc = 1;
    }

    // ── NON-VACUITY, measured rather than assumed ───────────────────────────────────────────
    //
    // "The checkout won a contest" and "the checkout was the only candidate on the machine" print
    // the same PASS above and mean very different things. The second proves almost nothing, and an
    // assertion that passes because the alternative does not exist on disk is precisely the defect
    // this program keeps finding. So say which one happened.
    //
    // THE LOG ALONE IS NOT ENOUGH, and getting this wrong once is why this block reads the disk:
    // when -addonsDir wins, the engine never mounts the packed copy, so a log-only check reports
    // "no rival" on exactly the runs that pass. Measured on this boot — a 570,489 B version-1.0.2
    // pak sat in <profile>/addons/ throughout and the console log never mentions it.
    let pak = format!("{profile_dir}/addons/TBDFramework_{guid}/data.pak");
    let text = read_log(log).unwrap_or_default();
    let mounted = Regex::new(&format!("Adding package '[^']*TBDFramework_{guid}/'"))
        .map(|re| re.is_match(&text))
        .unwrap_or(false);
    let downloaded = Regex::new(&format!("Downloading {guid} version"))
        .map(|re| re.is_match(&text))
        .unwrap_or(false);
    // `[ "$rival_bytes" -gt 0 ] 2>/dev/null` — a non-numeric value makes `[` fail, which reads as
    // false. `parse().unwrap_or(0) > 0` is the same answer without the stderr noise bash hid.
    let rival_positive = rival_bytes
        .map(|b| b.trim().parse::<i64>().unwrap_or(0) > 0)
        .unwrap_or(false);

    if mounted {
        out.o("  NOTE  non-vacuous: a packed Workshop copy was MOUNTED this boot (per the log).");
    } else if downloaded {
        out.o(
            "  NOTE  non-vacuous: the engine downloaded the Workshop copy this boot (per the log).",
        );
    } else if rival_positive {
        let bytes = rival_bytes.unwrap_or("");
        out.o("  NOTE  non-vacuous: a Workshop copy was on the server's disk and did NOT win —");
        out.o(format!("        {pak} ({bytes} bytes)"));
    } else if rival_bytes.is_none() && !profile_dir.is_empty() && Path::new(&pak).is_file() {
        let n = fs::metadata(&pak).map(|m| m.len()).unwrap_or(0);
        out.o("  NOTE  non-vacuous: a Workshop copy was on disk and did NOT win —");
        out.o(format!("        {pak} ({n} bytes)"));
    } else if rival_bytes.is_some() || !profile_dir.is_empty() {
        out.o("  NOTE  WEAK EVIDENCE: no Workshop copy in the log and none at");
        out.o(format!("        {pak}"));
        out.o(
            "        so the addon-path assertion had nothing to beat. To make it a real contest,",
        );
        out.o(
            "        boot once with -config and NO -addonsDir to populate that path, then re-run.",
        );
    } else {
        out.o("  NOTE  rival unknown — no profile dir given, so this could not check whether a");
        out.o(
            "        Workshop copy even exists. Pass the -profile dir to strengthen the verdict.",
        );
    }

    if rc != 0 {
        out.o("BOOT VERDICT: FAILED");
    } else {
        out.o("BOOT VERDICT: PASS");
    }
    rc
}

/// `--verify-boot <console.log>`.
///
/// Deliberately does NOT read `deploy.env`: the point is to run against a log you already have, on
/// a machine with no staging credentials.
pub fn verify_boot_cli(paths: &Paths, log: &Path) -> u8 {
    let guid = match std::env::var("TBD_ADDON_GUID") {
        Ok(v) if !v.is_empty() => v,
        _ => read_addon_guid(&paths.mono_root).unwrap_or_else(|| "B2C3D4E5F6A78901".to_string()),
    };
    let staging = std::env::var("TBD_ADDONS_STAGING").unwrap_or_default();
    if staging.is_empty() {
        eprintln!(
            "--verify-boot needs TBD_ADDONS_STAGING (the -addonsDir the server was launched with),"
        );
        eprintln!("so it knows which path counts as 'the checkout we deployed'. Export it, e.g.");
        eprintln!(
            "  TBD_ADDONS_STAGING=/home/sam/tbd/addons bash scripts/mod/deploy-staging.sh --verify-boot <log>"
        );
        return 2;
    }
    let admin_count = match std::env::var("TBD_ADMIN_COUNT") {
        Ok(v) if !v.is_empty() => v,
        _ => "0".to_string(),
    };
    let profile = std::env::var("TBD_PROFILE_DIR").unwrap_or_default();
    let mut out = Out::streams();
    verify_boot_log(&mut out, log, &guid, &staging, &admin_count, &profile, None) as u8
}

// ── the selftest ────────────────────────────────────────────────────────────────────────────

/// `--verify-boot-selftest`: prove the verdict can FAIL. A gate that has never been observed
/// failing is not a gate. Every fixture here is a log the engine really can produce.
pub fn selftest(_paths: &Paths) -> u8 {
    let d = std::env::temp_dir().join(format!(
        "tbd-verify-boot.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|t| t.subsec_nanos())
            .unwrap_or(0)
    ));
    if fs::create_dir_all(&d).is_err() {
        eprintln!("FAIL: could not create a scratch dir at {}", d.display());
        return 1;
    }
    let guid = "B2C3D4E5F6A78901";
    let staging = "/home/sam/tbd/addons";
    let mut pass = 0u32;
    let mut fail = 0u32;

    // (a) THE DEFECT: -config only. Room registers, config valid, mod loads — from the profile
    //     pak. Byte-shape copied from a real 2026-08-01 boot on this machine.
    let config_only = format!(
        "00:12:47.281 BACKEND      : Addon Download started {guid} - TBD Framework\n\
         00:12:47.281 BACKEND      : Downloading {guid} version 1.0.2\n\
         00:12:51.113 ENGINE       : FileSystem: Adding package '/home/sam/tbd/profile/addons/TBDFramework_{guid}/' (pak count: 1) to filesystem under name TBD_Framework\n\
         00:12:51.285  ENGINE       : Loaded addons:\n\
         00:12:51.285   ENGINE       : gproj: './addons/core/core.gproj' guid: '5614BBCCBB55ED1C'\n\
         00:12:51.285   ENGINE       : gproj: '/home/sam/tbd/profile/addons/TBDFramework_{guid}/addon.gproj' guid: '{guid}'\n\
         00:12:28.401  BACKEND      : Server config loaded.\n\
         00:12:28.401   BACKEND      : JSON is Valid\n\
         00:12:58.689 BACKEND      : Server registered with address: 192.168.0.140:2001\n\
         00:12:58.689 SCRIPT       : [TBD][Stage] LOADING -> LOBBY\n"
    );
    // (b) THE FIX: -addonsDir + -config. Same two healthy lines, different gproj path.
    let both_flags = format!(
        "00:20:30.385 ENGINE       : FileSystem: Adding relative directory '/home/sam/tbd/apps/mod/tbd-framework' to filesystem under name TBD_Framework\n\
         00:20:30.564  ENGINE       : Loaded addons:\n\
         00:20:30.564   ENGINE       : gproj: './addons/core/core.gproj' guid: '5614BBCCBB55ED1C'\n\
         00:20:30.564   ENGINE       : gproj: '{staging}/tbd-framework/addon.gproj' guid: '{guid}'\n\
         00:20:28.401  BACKEND      : Server config loaded.\n\
         00:20:28.401   BACKEND      : JSON is Valid\n\
         00:20:58.689 BACKEND      : Server registered with address: 192.168.0.140:2001\n"
    );
    // (c) addons mode: right code, no room. The other broken half.
    let addons_only = format!(
        "21:52:30.564  ENGINE       : Loaded addons:\n\
         21:52:30.564   ENGINE       : gproj: '{staging}/tbd-framework/addon.gproj' guid: '{guid}'\n\
         21:52:36.933 SCRIPT       : [TBD][Validate] mission result=PASS errors=0 warnings=5\n\
         21:52:40.000 SCRIPT       : [TBD][Stage] LOADING -> LOBBY\n"
    );
    // (d) mod absent entirely.
    let no_mod = "00:30:30.564  ENGINE       : Loaded addons:\n\
         00:30:30.564   ENGINE       : gproj: './addons/core/core.gproj' guid: '5614BBCCBB55ED1C'\n\
         00:30:28.401  BACKEND      : Server config loaded.\n\
         00:30:28.401   BACKEND      : JSON is Valid\n\
         00:30:58.689 BACKEND      : Server registered with address: 192.168.0.140:2001\n"
        .to_string();

    for (name, body) in [
        ("config-only.log", &config_only),
        ("both-flags.log", &both_flags),
        ("addons-only.log", &addons_only),
        ("no-mod.log", &no_mod),
    ] {
        if fs::write(d.join(name), body).is_err() {
            eprintln!("FAIL: could not write the {name} fixture");
            return 1;
        }
    }

    // name | file | fn | expected rc
    // One row per bash `cases=()` entry — kept as a TABLE so the two can be diffed by eye.
    #[rustfmt::skip]
    let cases: [(&str, &str, &str, i32); 10] = [
        ("-config only: WORKSHOP copy won -> must FAIL", "config-only.log", "addon", 1),
        ("-addonsDir + -config: checkout won -> must PASS", "both-flags.log", "addon", 0),
        ("addons mode: checkout won -> addon check PASSES", "addons-only.log", "addon", 0),
        ("addons mode: no room -> must FAIL", "addons-only.log", "room", 1),
        ("-config only: room registered -> room check PASSES", "config-only.log", "room", 0),
        ("mod never loaded at all -> must FAIL", "no-mod.log", "addon", 1),
        ("missing log file -> must FAIL (check did not run)", "ABSENT.log", "addon", 1),
        ("missing log file -> room check must FAIL too", "ABSENT.log", "room", 1),
        ("addons mode has no config -> admin check must FAIL", "addons-only.log", "admin", 1),
        ("config accepted -> admin check PASSES", "both-flags.log", "admin", 0),
    ];
    for (name, file, which, want) in cases {
        let path = d.join(file);
        // Every case is run with its output swallowed, exactly as the bash's `>/dev/null 2>&1`
        // did: the selftest is asserting the RETURN CODE, and letting the assertion text through
        // would bury the PASS/FAIL lines it prints.
        let mut sink = Out::captured();
        let got = match which {
            "addon" => assert_local_addon_won(&mut sink, &path, guid, staging),
            "room" => assert_room_registered(&mut sink, &path),
            _ => assert_admins_configured(&mut sink, &path, "1"),
        };
        if got == want {
            println!("  PASS  {name}");
            pass += 1;
        } else {
            println!("  FAIL  {name} (wanted rc={want}, got rc={got})");
            fail += 1;
        }
    }

    // The two directions must not agree. If the same log both passes and fails the addon check,
    // the check is reading nothing. This is the guard against a pattern that matches everything
    // (or nothing) still printing ten green lines above.
    let mut s1 = Out::captured();
    let mut s2 = Out::captured();
    let good = assert_local_addon_won(&mut s1, &d.join("both-flags.log"), guid, staging) == 0;
    let bad = assert_local_addon_won(&mut s2, &d.join("config-only.log"), guid, staging) != 0;
    if good && bad {
        println!("  PASS  the addon check DISCRIMINATES (passes one log, fails the other)");
        pass += 1;
    } else {
        println!("  FAIL  the addon check does not discriminate — it is vacuous.");
        fail += 1;
    }

    // The format check that USED to be sufficient is not, and this proves it on the spot: the
    // -config-only log is the stale SOURCE, yet a current-format Workshop build makes it
    // indistinguishable by line count. Kept as an executable statement so nobody re-derives the
    // format check as a substitute for the path check.
    let tagged = config_only.lines().filter(|l| l.contains("[TBD][")).count();
    if tagged > 0 {
        println!(
            "  PASS  format check alone would MISS this (log has {tagged} '[TBD][' lines yet loaded"
        );
        println!("        the Workshop copy) — proves the path check is not redundant with it");
        pass += 1;
    } else {
        println!("  FAIL  fixture (a) should carry current-format tagged lines");
        fail += 1;
    }

    // ── the non-vacuity reporter itself ─────────────────────────────────────────────────────
    // It got this wrong once (log-only, so it cried "no rival" on exactly the passing runs).
    // Pin all three ways it can learn about the rival, or the next edit reintroduces that.
    let rival_dir = d.join(format!("profile/addons/TBDFramework_{guid}"));
    let _ = fs::create_dir_all(&rival_dir);
    let _ = fs::write(rival_dir.join("data.pak"), vec![0u8; 4096]);
    let both = d.join("both-flags.log");
    let profile = d.join("profile");

    // (i) rival on DISK, log silent about it — the shape a passing -addonsDir boot really has
    let mut sink = Out::captured();
    verify_boot_log(
        &mut sink,
        &both,
        guid,
        staging,
        "1",
        &profile.display().to_string(),
        None,
    );
    if sink
        .text()
        .contains("non-vacuous: a Workshop copy was on disk and did NOT win")
    {
        println!("  PASS  rival found on DISK when the log never mentions it");
        pass += 1;
    } else {
        println!("  FAIL  rival on disk not reported: {}", sink.text());
        fail += 1;
    }
    // (ii) caller pre-measured it (the remote-deploy path, where a local stat cannot work)
    let mut sink = Out::captured();
    verify_boot_log(
        &mut sink,
        &both,
        guid,
        staging,
        "1",
        "/nonexistent/remote",
        Some("570489"),
    );
    if sink.text().contains("on the server's disk and did NOT win")
        && sink.text().contains("570489")
    {
        println!("  PASS  caller-measured rival size is trusted over a local stat");
        pass += 1;
    } else {
        println!("  FAIL  pre-measured rival not reported: {}", sink.text());
        fail += 1;
    }
    // (iii) genuinely no rival -> must say the evidence is WEAK, not print a clean pass
    let mut sink = Out::captured();
    verify_boot_log(
        &mut sink,
        &both,
        guid,
        staging,
        "1",
        "/nonexistent/remote",
        Some("0"),
    );
    if sink.text().contains("WEAK EVIDENCE") {
        println!("  PASS  absent rival is reported as WEAK EVIDENCE, not as a clean win");
        pass += 1;
    } else {
        println!("  FAIL  absent rival not flagged weak: {}", sink.text());
        fail += 1;
    }

    let _ = fs::remove_dir_all(&d);
    println!();
    if fail != 0 {
        println!("BOOT VERDICT SELFTEST: {pass} passed, {fail} FAILED");
        return 1;
    }
    println!("BOOT VERDICT SELFTEST: {pass} passed, 0 failed");
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tbd-t853-boot-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn grep_after_resets_the_window_on_each_match() {
        let text = "M\n1\n2\nx\nM\na\nb\n";
        assert_eq!(grep_after(text, "M", 2), vec!["M", "1", "2", "M", "a", "b"]);
        // A second match INSIDE the window merges rather than duplicating.
        assert_eq!(grep_after("M\nM\nz\n", "M", 1), vec!["M", "M", "z"]);
        // Window exhausted -> later lines are dropped.
        assert_eq!(grep_after("M\n1\n2\n", "M", 1), vec!["M", "1"]);
    }

    #[test]
    fn addon_check_discriminates_on_path_not_guid() {
        // ANTI-VACUITY, as a unit test rather than only inside the selftest: both logs carry the
        // SAME guid and both look healthy. Only the gproj path differs.
        let d = tmp("addon");
        let guid = "B2C3D4E5F6A78901";
        let win = d.join("win.log");
        let lose = d.join("lose.log");
        fs::write(
            &win,
            format!(
                "ENGINE : Loaded addons:\n ENGINE : gproj: '/home/sam/tbd/addons/tbd-framework/addon.gproj' guid: '{guid}'\n"
            ),
        )
        .unwrap();
        fs::write(
            &lose,
            format!(
                "ENGINE : Loaded addons:\n ENGINE : gproj: '/home/sam/tbd/profile/addons/TBDFramework_{guid}/addon.gproj' guid: '{guid}'\n"
            ),
        )
        .unwrap();
        let mut o = Out::captured();
        assert_eq!(
            assert_local_addon_won(&mut o, &win, guid, "/home/sam/tbd/addons"),
            0
        );
        let mut o = Out::captured();
        assert_eq!(
            assert_local_addon_won(&mut o, &lose, guid, "/home/sam/tbd/addons"),
            1
        );
        assert!(
            o.text()
                .contains("STAGING IS VALIDATING A BUILD IT DID NOT DEPLOY")
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn last_loaded_addons_block_wins() {
        // The engine prints `Loaded addons:` twice per boot; the FINAL block is the one that ran.
        // A first block naming the Workshop copy must not save a run whose second block does too,
        // and — the case that matters — a first block naming the Workshop copy must not SINK a run
        // whose second block names the checkout.
        let d = tmp("last");
        let guid = "B2C3D4E5F6A78901";
        let log = d.join("two.log");
        fs::write(
            &log,
            format!(
                "ENGINE : Loaded addons:\n ENGINE : gproj: '/profile/addons/TBDFramework_{guid}/addon.gproj' guid: '{guid}'\n\
                 ENGINE : Loaded addons:\n ENGINE : gproj: '/home/sam/tbd/addons/tbd-framework/addon.gproj' guid: '{guid}'\n"
            ),
        )
        .unwrap();
        let mut o = Out::captured();
        assert_eq!(
            assert_local_addon_won(&mut o, &log, guid, "/home/sam/tbd/addons"),
            0
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn missing_log_is_not_a_pass() {
        // The whole point of the four-outcome posture: a check that could not read its input must
        // FAIL, not print green.
        let missing = Path::new("/nonexistent/tbd-t853/console.log");
        let mut o = Out::captured();
        assert_eq!(assert_local_addon_won(&mut o, missing, "X", "/a"), 1);
        assert!(
            o.text()
                .contains("The check did NOT run. This is not a pass.")
        );
        let mut o = Out::captured();
        assert_eq!(assert_room_registered(&mut o, missing), 1);
        let mut o = Out::captured();
        assert_eq!(assert_admins_configured(&mut o, missing, "1"), 1);
    }

    #[test]
    fn non_numeric_admin_count_takes_the_else_branch() {
        // ODDITY PINNED: bash `[ "$x" -eq 0 ]` errors, returns 2, and inside an `if` condition
        // `set -e` stays quiet, so junk read as "not zero" and got printed verbatim.
        let d = tmp("admin");
        let log = d.join("ok.log");
        fs::write(
            &log,
            "BACKEND : Server config loaded.\nBACKEND : JSON is Valid\n",
        )
        .unwrap();
        let mut o = Out::captured();
        assert_eq!(assert_admins_configured(&mut o, &log, "banana"), 0);
        assert!(
            o.text().contains("carrying banana admin id(s)"),
            "{}",
            o.text()
        );
        let mut o = Out::captured();
        assert_eq!(assert_admins_configured(&mut o, &log, "0"), 0);
        assert!(
            o.text()
                .contains("WARN  config accepted, but game.admins[] is EMPTY")
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn rival_precedence_matches_the_bash_if_chain() {
        let d = tmp("rival");
        let guid = "B2C3D4E5F6A78901";
        let log = d.join("b.log");
        fs::write(
            &log,
            format!(
                "ENGINE : Loaded addons:\n ENGINE : gproj: '/a/tbd-framework/addon.gproj' guid: '{guid}'\n\
                 BACKEND : Server config loaded.\nBACKEND : JSON is Valid\n\
                 BACKEND : Server registered with address: 1.2.3.4:2001\n"
            ),
        )
        .unwrap();
        // A measured size wins over the local stat, even when profile_dir is unreachable.
        let mut o = Out::captured();
        verify_boot_log(
            &mut o,
            &log,
            guid,
            "/a",
            "1",
            "/nonexistent",
            Some("570489"),
        );
        assert!(o.text().contains("570489 bytes"));
        // "" profile AND no measurement -> the weakest message of the four.
        let mut o = Out::captured();
        verify_boot_log(&mut o, &log, guid, "/a", "1", "", None);
        assert!(o.text().contains("rival unknown"));
        // Measured ZERO is not "unknown" — it is WEAK EVIDENCE, which is a different claim.
        let mut o = Out::captured();
        verify_boot_log(&mut o, &log, guid, "/a", "1", "", Some("0"));
        assert!(o.text().contains("WEAK EVIDENCE"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn guid_reads_out_of_the_real_gproj() {
        // Not a fixture: the live gproj, because the whole point of read_addon_guid is that a
        // literal drifts from the source silently.
        let root = crate::root::find_repo_root().unwrap();
        if root.join("apps/mod/tbd-framework/addon.gproj").is_file() {
            let g = read_addon_guid(&root).expect("gproj present");
            assert!(
                g.len() >= 8 && g.chars().all(|c| c.is_ascii_hexdigit()),
                "guid={g}"
            );
        }
    }
}
