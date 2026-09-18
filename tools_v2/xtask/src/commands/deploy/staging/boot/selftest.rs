use super::*;

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
