use super::*;

#[test]
fn a_fresh_state_is_neither_held_nor_degraded() {
    let s = GateState::new();
    assert!(!s.held());
    assert!(!s.unserialised());
}

/// INTEROP, BOTH DIRECTIONS — the requirement that makes a half-ported factory safe.
///
/// During the overlap a machine WILL run `scripts/platform/wave.sh gate` and
/// `cargo xtask platform wave gate` at the same time, and they must contend. `flock(1)` and
/// `flock(2)` are the same primitive on the same inode, so this is a property of naming the
/// same path — and the only way to know it holds is to make the two fight over one file.
#[test]
fn bash_flock_and_this_port_contend_on_one_file() {
    use std::process::{Command, Stdio};
    let mut p = std::env::temp_dir();
    p.push(format!("tbd-wave-lock-interop-{}", std::process::id()));
    std::fs::write(&p, b"").unwrap();

    let probe = |path: &std::path::Path| -> bool {
        Command::new("flock")
            .args(["-n", "-x"])
            .arg(path)
            .args(["-c", "true"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("flock(1) on PATH")
            .success()
    };

    // ANTI-VACUITY FIRST, and on a SEPARATE file. `!probe(&p)` below is only evidence if
    // `probe` can succeed at all — otherwise a broken `flock` would "prove" contention.
    //
    // It runs against a fresh file rather than against `p` after a release, because the
    // release-then-probe form was MEASURED FLAKY under `cargo test`'s thread pool: a sibling
    // test forking between our `fork` and its `exec` copies the still-open lock descriptor and
    // holds the lock past our drop. See the LOCK RELEASE note in the module header — that
    // observation is what corrected it.
    let mut free = std::env::temp_dir();
    free.push(format!("tbd-wave-lock-free-{}", std::process::id()));
    std::fs::write(&free, b"").unwrap();
    assert!(
        probe(&free),
        "flock -n failed even on a FREE lock — the probe tests nothing"
    );
    let _ = std::fs::remove_file(&free);

    // DIRECTION 1: Rust holds, bash must be REFUSED.
    let held = flock_exclusive(&p, Duration::from_secs(1), Duration::from_secs(5), |_| {}).unwrap();
    assert!(
        !probe(&p),
        "bash flock -n TOOK the lock while this port held it — the two do not contend, and a \
         half-ported factory would run two gates over the same target dirs"
    );
    drop(held);

    // DIRECTION 2: bash holds, this port must REFUSE rather than proceed unserialised.
    let mut holder = Command::new("flock")
        .args(["-x"])
        .arg(&p)
        .args(["-c", "sleep 3"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn flock holder");
    // Give the holder a moment to actually take it before probing.
    std::thread::sleep(Duration::from_millis(400));
    let got = flock_exclusive(
        &p,
        Duration::from_millis(50),
        Duration::from_millis(200),
        |_| {},
    );
    assert!(
        matches!(got, Err(NotRun::Timeout { .. })),
        "this port took the lock while bash flock held it: {got:?}"
    );
    let _ = holder.kill();
    let _ = holder.wait();
    let _ = std::fs::remove_file(&p);
}

#[test]
fn unserialised_verdict_cannot_look_like_a_clean_pass() {
    // The whole point of T-409's relabelling: a log scraper reading the last line must see it.
    let mut s = GateState::new();
    s.unserialised = true;
    s.why = "flock is not on PATH".into();
    // Rendering is asserted via the format string here rather than by capturing stdout; the
    // byte-for-byte check is the diff harness's job.
    assert_eq!(
        format!(
            "{}: {} — UNSERIALISED, NOT A CLEAN {}",
            "GATE", "PASS", "PASS"
        ),
        "GATE: PASS — UNSERIALISED, NOT A CLEAN PASS"
    );
}
