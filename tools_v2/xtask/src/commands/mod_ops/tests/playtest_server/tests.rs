use super::*;

fn opts(args: &[&str]) -> Opts {
    let v: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    match parse(&v, "/home/u") {
        Parsed::Opts(o) => *o,
        Parsed::Help => panic!("expected Opts, got Help"),
        Parsed::Unknown(u) => panic!("expected Opts, got Unknown({u})"),
    }
}

#[test]
fn defaults_match_the_bash_variable_block() {
    let o = opts(&[]);
    assert_eq!(o.backend_url, "http://127.0.0.1:8080");
    assert_eq!(o.game_port, "2001");
    assert_eq!(o.a2s_port, "17777");
    assert_eq!(o.max_players, "8");
    assert_eq!(o.run_dir, "/home/u/tbd-playtest");
    assert!(!o.dry_run && !o.selftest && o.admins.is_empty());
}

#[test]
fn help_position_decides_the_exit_code() {
    // THE ODDITY. bash acts on tokens in order, so which of these two wins is positional.
    assert!(matches!(
        parse(&["--bogus".into(), "--help".into()], "/h"),
        Parsed::Unknown(_)
    ));
    assert!(matches!(
        parse(&["--help".into(), "--bogus".into()], "/h"),
        Parsed::Help
    ));
}

#[test]
fn values_may_contain_equals_signs() {
    // bash `${arg#*=}` strips through the FIRST `=` only.
    assert_eq!(opts(&["--name=a=b=c"]).server_name, "a=b=c");
}

#[test]
fn empty_values_are_carried_not_dropped() {
    // `--admin=` must reach validation and be REJECTED, not silently skipped (baseline a15).
    assert_eq!(opts(&["--admin="]).admins, vec![""]);
    assert!(!admin_id_is_valid(""));
    // `--mission=` must reach the required check.
    assert!(opts(&["--mission="]).mission.is_empty());
}

#[test]
fn admins_are_repeatable_and_ordered() {
    assert_eq!(
        opts(&["--admin=a", "--admin=b", "--admin=c"]).admins,
        ["a", "b", "c"]
    );
}

#[test]
fn bare_and_lone_dashes_are_unknown_arguments() {
    // `--` is NOT a separator here; bash's case had no arm for it (baselines a17/a18/a20).
    assert!(matches!(parse(&["--".into()], "/h"), Parsed::Unknown(_)));
    assert!(matches!(parse(&["-".into()], "/h"), Parsed::Unknown(_)));
    assert!(matches!(parse(&["".into()], "/h"), Parsed::Unknown(_)));
}

#[test]
fn admin_schema_matches_the_engines_two_patterns() {
    assert!(admin_id_is_valid("b2c3d4e5-f6a7-8901-b2c3-d4e5f6a78901"));
    assert!(admin_id_is_valid("76561198000000000"));
    // Uppercase hex is REJECTED — the engine's pattern is lowercase-only (baseline a12).
    assert!(!admin_id_is_valid("B2C3D4E5-F6A7-8901-B2C3-D4E5F6A78901"));
    assert!(!admin_id_is_valid("1234567890123456")); // 16
    assert!(!admin_id_is_valid("123456789012345678")); // 18
    assert!(!admin_id_is_valid("nope"));
    assert!(!admin_id_is_valid(""));
}

#[test]
fn admin_newline_widening_is_preserved() {
    // bash piped the value into `grep`, which anchors per LINE, so an embedded newline let a
    // junk value through as long as ONE line matched. `Pattern` is multi_line for exactly this
    // compatibility reason. Pinned, not fixed: see `admin_id_is_valid`.
    assert!(admin_id_is_valid(
        "junk\n00000000-0000-0000-0000-000000000000"
    ));
}

#[test]
fn guid_is_read_out_of_a_real_gproj_shape() {
    assert_eq!(
        read_addon_guid("Project {\n  GUID \"B2C3D4E5F6A78901\"\n  Title \"TBD\"\n}\n"),
        "B2C3D4E5F6A78901"
    );
}

#[test]
fn a_gproj_without_a_guid_yields_empty_not_a_guess() {
    // The emptiness IS the rc 1 arm (baseline c01) — never a fabricated GUID.
    assert_eq!(read_addon_guid("Project {\n  Title \"x\"\n}\n"), "");
    assert_eq!(read_addon_guid(""), "");
}

#[test]
fn scenario_extraction_stops_at_the_comma_and_the_quote() {
    // The two-stage `grep -o | grep -o` the bash used, on the committed dev config's line.
    assert_eq!(
        read_scenario("    \"scenarioId\": \"{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf\",\n"),
        "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf"
    );
    // Baseline c02: a config with no scenarioId is rc 1, not a silent empty scenario.
    assert_eq!(
        read_scenario("{\n  \"game\": {\n    \"name\": \"x\"\n  }\n}"),
        ""
    );
}

#[test]
fn help_text_matches_the_options_we_parse() {
    // What the bash's `sed`-your-own-header trick was really buying: help and parser cannot
    // drift. Every long flag named in HELP must be one the loop accepts.
    for line in HELP.lines() {
        let t = line.trim_start();
        if !t.starts_with("--") {
            continue;
        }
        let flag = t.split([' ', '=']).next().unwrap();
        let probe = if t.contains('=') {
            format!("{flag}=v")
        } else {
            flag.to_string()
        };
        assert!(
            !matches!(
                parse(std::slice::from_ref(&probe), "/h"),
                Parsed::Unknown(_)
            ),
            "HELP advertises {probe} but the parser rejects it"
        );
    }
}

#[test]
fn help_opens_with_the_runnable_command_and_lists_every_option() {
    assert_eq!(HELP.lines().count(), 22);
    assert!(HELP.starts_with("Usage:\n  cargo xtask mod playtest"));
    assert!(HELP.ends_with("boots no game server\n"));
}
