use super::*;

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
pub(super) fn grep_after(text: &str, needle: &str, after: usize) -> Vec<String> {
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

pub(super) fn read_log(log: &Path) -> Option<String> {
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
    out.e("  It must carry BOTH -addonsDir and -config.");
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
        out.e("      'No server found'. Joinable needs -config, alongside -addonsDir.");
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
            "  TBD_ADDONS_STAGING=/home/sam/tbd/addons cargo xtask deploy staging --verify-boot <log>"
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
