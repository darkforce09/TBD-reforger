use super::*;

fn with_log(tag: &str, body: &str) -> RunPaths {
    let d = std::env::temp_dir().join(format!("tbd-rps-boot-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let p = RunPaths::new(d.to_str().unwrap());
    std::fs::write(&p.srv_out, body).unwrap();
    p
}
#[test]
fn boot_phase_reports_the_furthest_milestone_not_the_first() {
    // Order matters: a log that has BOTH must report LOBBY, the newest.
    let p = with_log(
        "phase",
        "Compiling Game scripts\nGameProject load\nGame::LoadEntities took 3s\n\
             NETWORK  : Starting RPL server, listening on address 0.0.0.0:2001\n\
             [TBD][Stage] LOADING -> LOBBY\n",
    );
    assert!(boot_phase(&p).starts_with("WORLD UP, mission already in LOBBY"));
    let _ = std::fs::remove_dir_all(&p.run_dir);
}

#[test]
fn the_lobby_marker_survives_a_reworded_arrow() {
    // T-606: `grep -F '[TBD][Stage] LOADING -> LOBBY'` dropped to ZERO matches when the arrow
    // changed, i.e. a server that WAS in LOBBY was reported as never having got there.
    let p = with_log("arrow", "[TBD][Stage] LOADING => LOBBY\n");
    assert!(boot_phase(&p).starts_with("WORLD UP, mission already in LOBBY"));
    assert!(world_is_up(&p));
    let _ = std::fs::remove_dir_all(&p.run_dir);
}

#[test]
fn an_absent_log_is_a_phase_not_a_crash() {
    let p = RunPaths::new("/nonexistent/run/dir");
    assert_eq!(boot_phase(&p), "engine has not written anything yet");
    assert!(!world_is_up(&p));
}

#[test]
fn phases_below_the_world_do_not_claim_the_world_is_up() {
    let p = with_log("early", "Compiling Game scripts\n");
    assert_eq!(boot_phase(&p), "compiling scripts");
    assert!(!world_is_up(&p));
    let _ = std::fs::remove_dir_all(&p.run_dir);
}

#[test]
fn the_local_addon_must_win_or_the_gate_fails() {
    let mut o = Opts::defaults("/h");
    o.run_dir = "/home/u/tbd-playtest".into();
    let local = "ENGINE   : Loaded addons:\n\
             ENGINE   :   gproj: '/home/u/tbd-playtest/addons/tbd-framework/addon.gproj' guid: 'B2C3D4E5F6A78901'\n";
    let p = with_log("won", local);
    assert!(assert_local_addon_won(&p, &o, "B2C3D4E5F6A78901"));
    let _ = std::fs::remove_dir_all(&p.run_dir);

    // THE TRAP: the packed Workshop copy answering for the same GUID.
    let stale = "ENGINE   : Loaded addons:\n\
             ENGINE   :   gproj: '/home/u/tbd-playtest/profile/addons/TBDFramework_B2C3D4E5F6A78901/addon.gproj' guid: 'B2C3D4E5F6A78901'\n";
    let p = with_log("stale", stale);
    assert!(
        !assert_local_addon_won(&p, &o, "B2C3D4E5F6A78901"),
        "the stale Workshop pak winning must be a HARD failure, never a warning"
    );
    let _ = std::fs::remove_dir_all(&p.run_dir);
}

#[test]
fn a_guid_outside_the_loaded_addons_window_does_not_count() {
    // `grep -A6` bounds the search to six lines after the header. A `guid:` line further down
    // (the "Available addons:" list, say) must not satisfy the gate.
    let far = format!(
        "ENGINE   : Loaded addons:\n{}ENGINE   :   gproj: '/r/addons/tbd-framework/addon.gproj' guid: 'G1'\n",
        "filler\n".repeat(7)
    );
    assert!(loaded_addon_line(&far, "G1").is_none());
    let near =
        "ENGINE   : Loaded addons:\nfiller\nENGINE   :   gproj: '/r/x/addon.gproj' guid: 'G1'\n";
    assert!(loaded_addon_line(near, "G1").is_some());
}

#[test]
fn loaded_addon_line_takes_the_last_match_like_tail_1() {
    let two = "Loaded addons:\n gproj: 'first' guid: 'G'\n gproj: 'second' guid: 'G'\n";
    assert!(loaded_addon_line(two, "G").unwrap().contains("second"));
}

#[test]
fn the_hard_gate_holds_on_a_real_engine_boot() {
    // NOT a synthetic log. Captured verbatim from `ArmaReforgerServer` 1.7.0.54 booted BY THIS
    // PORT on 2026-08-12 (`--run-dir=/tmp/t853/w-play/live --port=2031`), lines 164-169 of its
    // `server.out`. Two details only a real boot supplies, and both are load-bearing:
    //
    //   * the engine's own leading-space indent grows by one per nesting level, so the `gproj:`
    //     lines start with two spaces and would not match a `^ENGINE` anchor;
    //   * the wanted entry is the THIRD gproj under the header, three lines down — inside
    //     `grep -A6`'s window, but only just, and it is preceded by two vanilla addons that
    //     `tail -1` must not select.
    //
    // This is the T-604 finding in evidence: `-addonsDir` + `-config` together, and the LOCAL
    // checkout wins over the unlisted Workshop 1.0.1 published under the same GUID.
    let real = "\
ENGINE       : GameProject load
 ENGINE       : Loaded addons:
  ENGINE       : gproj: './addons/core/core.gproj' guid: '5614BBCCBB55ED1C'
  ENGINE       : gproj: './addons/data/ArmaReforger.gproj' guid: '58D0FB3206B6F859'
  ENGINE       : gproj: '/tmp/t853/w-play/live/addons/tbd-framework/addon.gproj' guid: 'B2C3D4E5F6A78901'
GUI          : Using default language (en_us)
";
    let line = loaded_addon_line(real, "B2C3D4E5F6A78901")
        .expect("the real boot's addon line must be found");
    assert!(line.contains("/tmp/t853/w-play/live/addons/tbd-framework/addon.gproj"));
    // And the gate itself agrees, against the run dir that boot actually used.
    let mut o = Opts::defaults("/h");
    o.run_dir = "/tmp/t853/w-play/live".into();
    let p = with_log("realboot", real);
    assert!(assert_local_addon_won(&p, &o, "B2C3D4E5F6A78901"));
    // Flip the run dir and the same log must now FAIL — proving the gate reads the path and is
    // not merely finding the GUID somewhere.
    o.run_dir = "/somewhere/else".into();
    assert!(
        !assert_local_addon_won(&p, &o, "B2C3D4E5F6A78901"),
        "the gate must compare the PATH, not just the GUID"
    );
    let _ = std::fs::remove_dir_all(&p.run_dir);
}

#[test]
fn grep_n_numbers_from_one_and_grep_c_counts_lines() {
    let t = "a\nDEFAULT   (E): noise\nb\nBACKEND (E): real\n";
    assert_eq!(
        grep_n(t, r"\((E|F)\):"),
        vec![
            "2:DEFAULT   (E): noise".to_string(),
            "4:BACKEND (E): real".to_string()
        ]
    );
    assert_eq!(
        grep_c(
            t,
            "^[[:space:]]*(DEFAULT|MATERIAL|RESOURCES)[[:space:]]*\\(E\\):"
        ),
        1
    );
}

#[test]
fn the_vanilla_error_floor_is_demoted_not_the_real_cause() {
    // The measured shape: 75 floor lines, four that matter. The floor must not crowd out the
    // signal inside the `head -20` window.
    let mut body = String::new();
    for i in 0..75 {
        body.push_str(&format!("DEFAULT   (E): Trying to register a signal {i}\n"));
    }
    body.push_str("BACKEND  (E): the actual cause\n");
    let p = with_log("floor", &body);
    let t = log(&p);
    let noise =
        Pattern::regex("^[0-9]+:[[:space:]]*(DEFAULT|MATERIAL|RESOURCES)[[:space:]]*\\(E\\):")
            .unwrap();
    let signal: Vec<String> = grep_n(&t, r"\((E|F)\):")
        .into_iter()
        .filter(|l| !noise.is_match(l))
        .take(20)
        .collect();
    assert_eq!(
        signal.len(),
        1,
        "the floor leaked into the signal: {signal:?}"
    );
    assert!(signal[0].ends_with("BACKEND  (E): the actual cause"));
    assert_eq!(
        grep_c(
            &t,
            "^[[:space:]]*(DEFAULT|MATERIAL|RESOURCES)[[:space:]]*\\(E\\):"
        ),
        75
    );
    let _ = std::fs::remove_dir_all(&p.run_dir);
}

#[test]
fn console_path_falls_back_to_the_literal_slash_console_log() {
    // PRESERVED ODDITY — see `console_log_path`.
    assert_eq!(console_log_path("/nonexistent/run"), "/console.log");
}

#[test]
fn console_path_takes_the_newest_logs_dir() {
    let d = std::env::temp_dir().join(format!("tbd-rps-console-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("profile/logs/logs_2026-01-01_00-00-00")).unwrap();
    std::fs::create_dir_all(d.join("profile/logs/logs_2026-08-12_09-00-00")).unwrap();
    std::fs::create_dir_all(d.join("profile/logs/not-a-log")).unwrap();
    let got = console_log_path(d.to_str().unwrap());
    assert!(
        got.ends_with("logs_2026-08-12_09-00-00/console.log"),
        "{got}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_registered_marker_and_the_fatals_are_the_engines_own_strings() {
    let t = "BACKEND  : Server registered with address: 192.168.0.117:2001\n";
    assert!(has(t, "Server registered with address:"));
    for fatal in [
        "There are errors in server config!",
        "Unable to initialize the game",
        "NETWORK (E): Unable to start replication",
    ] {
        assert!(
            has_re(
                fatal,
                "There are errors in server config!|Unable to initialize the game|Unable to start replication"
            ),
            "{fatal}"
        );
    }
}
