use super::*;

fn tmp_lock(name: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "verification-core-lock-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&p);
    p
}

#[test]
fn acquires_when_free() {
    let p = tmp_lock("free");
    let got = flock_exclusive(&p, DEFAULT_POLL, Duration::from_secs(5), |_| {});
    assert!(got.is_ok());
    let _ = std::fs::remove_file(&p);
}

#[test]
fn a_second_holder_is_refused_not_granted() {
    // flock() treats separate open file descriptions independently even within one process,
    // so this is a genuine contention test.
    let p = tmp_lock("contend");
    let first = flock_exclusive(&p, DEFAULT_POLL, Duration::from_secs(5), |_| {}).unwrap();

    let second = flock_exclusive(
        &p,
        Duration::from_millis(20),
        Duration::from_millis(150),
        |_| {},
    );
    match second {
        Err(NotRun::Timeout { .. }) => {}
        Ok(_) => panic!("TWO HOLDERS AT ONCE — the lock serialised nothing"),
        Err(other) => panic!("expected Timeout, got {other:?}"),
    }
    drop(first);
    let _ = std::fs::remove_file(&p);
}

#[test]
fn dropping_releases_for_the_next_holder() {
    let p = tmp_lock("release");
    let first = flock_exclusive(&p, DEFAULT_POLL, Duration::from_secs(5), |_| {}).unwrap();
    drop(first);
    let second = flock_exclusive(
        &p,
        Duration::from_millis(20),
        Duration::from_secs(2),
        |_| {},
    );
    assert!(second.is_ok(), "a released lock must be re-acquirable");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn exhaustion_is_did_not_run_never_a_pass() {
    // Give up rather than run unserialised.
    let p = tmp_lock("refuse");
    let _held = flock_exclusive(&p, DEFAULT_POLL, Duration::from_secs(5), |_| {}).unwrap();
    let got = flock_exclusive(
        &p,
        Duration::from_millis(10),
        Duration::from_millis(60),
        |_| {},
    );
    let cause = got.expect_err("must not succeed");
    assert!(matches!(cause, NotRun::Timeout { .. }));
    let _ = std::fs::remove_file(&p);
}

#[test]
fn heartbeat_fires_while_blocked() {
    let p = tmp_lock("beat");
    let _held = flock_exclusive(&p, DEFAULT_POLL, Duration::from_secs(5), |_| {}).unwrap();
    let mut beats = 0;
    let _ = flock_exclusive(
        &p,
        Duration::from_millis(20),
        Duration::from_millis(150),
        |_| {
            beats += 1;
        },
    );
    assert!(
        beats >= 2,
        "a silent block is indistinguishable from a hang; got {beats} beats"
    );
    let _ = std::fs::remove_file(&p);
}

#[test]
fn interops_with_the_flock_command() {
    // Any process naming this path must contend with this lock. If `flock(1)` and this do not
    // contend, both "serialise" against nothing and the whole lock is decorative.
    let p = tmp_lock("interop");
    if crate::proc::which("flock").is_err() {
        eprintln!("skip: flock(1) not installed");
        return;
    }
    // Hold the lock from a shell process the ordinary way (fd 9 + flock 9).
    let script = format!("exec 9>{}; flock 9; sleep 3", p.display());
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(&script)
        .spawn()
        .unwrap();
    // Give the child time to actually take it before asserting contention.
    std::thread::sleep(Duration::from_millis(400));

    let got = flock_exclusive(
        &p,
        Duration::from_millis(20),
        Duration::from_millis(200),
        |_| {},
    );
    let refused = matches!(got, Err(NotRun::Timeout { .. }));

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&p);
    assert!(
        refused,
        "this crate did not contend with flock(1) — the shared lock is decorative"
    );
}
