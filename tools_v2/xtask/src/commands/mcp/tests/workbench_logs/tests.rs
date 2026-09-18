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
fn file_equals_empty_is_environment_rc3() {
    // Pins `--file=` (empty path) → ENVIRONMENT 3, not clap rc=2 / usage.
    let code = run(Some(PathBuf::new()), false, false, None).unwrap();
    assert_eq!(code, 3);
}

#[test]
fn file_missing_sentinel_is_usage_rc3() {
    let code = run(Some(PathBuf::from("__MISSING__")), false, false, None).unwrap();
    assert_eq!(code, 3);
}

#[test]
fn preprocess_rewrites_file_empty_arg_to_missing_sentinel() {
    let args = preprocess_cli_args(
        ["xtask", "mcp", "wb-logs", "--file", ""]
            .into_iter()
            .map(OsString::from)
            .collect(),
    );
    assert_eq!(args[4], "__MISSING__");
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
