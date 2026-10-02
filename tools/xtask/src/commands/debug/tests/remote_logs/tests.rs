use super::remote_fetch::{FLEET_HOST_EXIT, LogSource, newest_console_log_script};
use super::*;
use crate::commands::debug::staging_fleet_instance::select_fleet_instance;
use crate::core::deploy_environment::DeployEnvironment;

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

/// Runs a newest-log script in bash with `HOME` at `home`.
fn run_script(script: String, home: &Path) -> std::process::Output {
    std::process::Command::new("bash")
        .arg("-c")
        .arg(script)
        .env("HOME", home)
        .output()
        .expect("bash runs")
}

/// A run folder holding a `console.log`, under `profile`'s `logs/`.
fn console_log(profile: &Path, run: &str) -> PathBuf {
    let folder = profile.join("logs").join(run);
    fs::create_dir_all(&folder).expect("run folder");
    write_log(&folder.join("console.log"), &["line"]);
    folder.join("console.log")
}

/// The newest-log script runs in bash against a scratch profile whose name needs quoting: it
/// skips a newer run without a `console.log`, prints the newest one that has it and exits 0, and
/// exits 1 when no run has one.
#[test]
fn newest_console_log_script_prints_the_newest_log_and_exits_zero() {
    let root = tempfile_dir("t-newest-console-log").expect("scratch dir");
    let profile = root.join("it's profile");
    let older = console_log(&profile, "logs_2026-01-01_10-00-00");
    fs::create_dir_all(profile.join("logs/logs_2026-01-01_11-00-00")).expect("newer run");
    let single = |profile: &Path| {
        newest_console_log_script(&LogSource::SingleServer(
            profile.to_str().expect("utf-8").to_string(),
        ))
    };

    let found = run_script(single(&profile), &root);
    assert_eq!(found.status.code(), Some(0), "{found:?}");
    assert_eq!(
        String::from_utf8_lossy(&found.stdout).trim(),
        older.to_str().expect("utf-8")
    );

    let empty = root.join("empty profile");
    fs::create_dir_all(&empty).expect("empty profile");
    let none = run_script(single(&empty), &root);
    assert_eq!(none.status.code(), Some(1), "{none:?}");
    fs::remove_dir_all(&root).expect("clean up");
}

/// `--instance N` reads instance N's own profile under the deploy user's home, never another
/// instance's newer log and never the single server's folder.
#[test]
fn remote_logs_instance_reads_that_instances_profile() {
    let home = tempfile_dir("t-remote-logs-instance").expect("scratch home");
    let second = console_log(
        &home.join("tbd/fleet/instance-2/profile"),
        "logs_2026-01-01_10-00-00",
    );
    console_log(
        &home.join("tbd/fleet/instance-3/profile"),
        "logs_2026-01-01_11-00-00",
    );
    let settings = DeployEnvironment::from_text(Path::new("/nonexistent/deploy.env"), Some(""), [])
        .expect("empty settings");
    let instance = |number| {
        LogSource::FleetInstance(select_fleet_instance(&settings, number).expect("in range"))
    };

    let found = run_script(newest_console_log_script(&instance(2)), &home);
    assert_eq!(found.status.code(), Some(0), "{found:?}");
    assert_eq!(
        String::from_utf8_lossy(&found.stdout).trim(),
        second.to_str().expect("utf-8")
    );
    let absent = run_script(newest_console_log_script(&instance(4)), &home);
    assert_eq!(absent.status.code(), Some(1), "{absent:?}");
    fs::remove_dir_all(&home).expect("clean up");
}

/// Without `--instance`, a host that holds `~/tbd/fleet` is refused with the fleet exit, even
/// when the single server's profile still holds a log.
#[test]
fn remote_logs_single_server_on_a_fleet_host_is_refused() {
    let home = tempfile_dir("t-remote-logs-fleet-host").expect("scratch home");
    let profile = home.join("tbd/profile");
    console_log(&profile, "logs_2026-01-01_10-00-00");
    let script = newest_console_log_script(&LogSource::SingleServer(
        profile.to_str().expect("utf-8").to_string(),
    ));
    assert_eq!(run_script(script.clone(), &home).status.code(), Some(0));
    fs::create_dir_all(home.join("tbd/fleet")).expect("fleet folder");
    let refused = run_script(script, &home);
    assert_eq!(refused.status.code(), Some(FLEET_HOST_EXIT), "{refused:?}");
    assert!(refused.stdout.is_empty(), "{refused:?}");
    fs::remove_dir_all(&home).expect("clean up");
}

/// `--instance` names a server to fetch from, so with a local file or the self-test it is a bad
/// flag: ENVIRONMENT, never a verdict.
#[test]
fn remote_logs_instance_with_file_or_selftest_is_environment() {
    let p = fixture(
        "instance-with-file",
        &["SCRIPT : [TBD][Mission] loaded id=msn_x name='N' slots=7 source=profile"],
    );
    assert_eq!(super::run(Some(p.clone()), false, Some(1)).unwrap(), 3);
    assert_eq!(super::run(None, true, Some(1)).unwrap(), 3);
    let _ = fs::remove_file(&p);
}
