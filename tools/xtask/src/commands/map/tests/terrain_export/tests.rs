use super::*;
use std::sync::Mutex;

// parse_args reads/writes process env (`TERRAIN`) — serialize these tests.
static TERRAIN_ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_terrain_env<R>(f: impl FnOnce() -> R) -> R {
    let _guard = TERRAIN_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev = env::var_os("TERRAIN");
    let out = f();
    match prev {
        Some(v) => unsafe { env::set_var("TERRAIN", v) },
        None => unsafe { env::remove_var("TERRAIN") },
    }
    out
}

#[test]
fn parse_usage_when_no_terrain() {
    with_terrain_env(|| {
        unsafe { env::remove_var("TERRAIN") };
        assert!(matches!(parse_args(&[]), Parse::Usage));
    });
}

#[test]
fn parse_unknown_arg() {
    let args = vec![
        "everon".into(), // E2c-allow
        "--bogus".into(),
    ];
    match parse_args(&args) {
        Parse::Unknown(a) => assert_eq!(a, "--bogus"),
        Parse::Usage | Parse::Ok { .. } => panic!("expected Unknown"),
    }
}

#[test]
fn parse_phase_and_default() {
    with_terrain_env(|| {
        unsafe { env::remove_var("TERRAIN") };
        let args_default = vec![
            "everon".into(), // E2c-allow
        ];
        match parse_args(&args_default) {
            Parse::Ok { terrain, phase } => {
                assert_eq!(terrain, "everon"); // E2c-allow
                assert_eq!(phase, "P1_buildings");
            }
            _ => panic!("expected Ok"),
        }
        let args_phase = vec![
            "arland".into(), // E2c-allow
            "--phase".into(),
            "P2_trees".into(),
        ];
        match parse_args(&args_phase) {
            Parse::Ok { terrain, phase } => {
                assert_eq!(terrain, "arland"); // E2c-allow
                assert_eq!(phase, "P2_trees");
            }
            _ => panic!("expected Ok"),
        }
    });
}

#[test]
fn parse_terrain_from_env_when_no_positional() {
    with_terrain_env(|| {
        unsafe { env::set_var("TERRAIN", "everon") }; // E2c-allow
        // bash: $1=--phase wins over env → terrain="--phase", then "P1_buildings" is unknown.
        match parse_args(&["--phase".into(), "P1_buildings".into()]) {
            Parse::Unknown(a) => assert_eq!(a, "P1_buildings"),
            Parse::Ok { terrain, .. } => panic!("unexpected Ok terrain={terrain}"),
            Parse::Usage => panic!("unexpected Usage"),
        }
        match parse_args(&[]) {
            Parse::Ok { terrain, phase } => {
                assert_eq!(terrain, "everon"); // E2c-allow
                assert_eq!(phase, "P1_buildings");
            }
            _ => panic!("expected env terrain"),
        }
    });
}
