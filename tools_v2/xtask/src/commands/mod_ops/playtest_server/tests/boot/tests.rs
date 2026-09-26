use super::*;

#[test]
fn the_launcher_argv_is_the_one_the_engine_needs() {
    // NEVER EXECUTED BY A TEST MACHINE WITHOUT THE ENGINE, so the argv itself is the contract.
    // Flag ORDER matches the staging deploy's deliberately.
    let s = launcher_script();
    assert!(s.contains("echo $$ > \"$1/server.pid\""));
    // The engine replaces the shell, so the recorded process group is the engine's; no far-side
    // `timeout` sits in front of it (the run deadline is kept in process).
    assert!(s.contains("exec ./ArmaReforgerServer"), "{s}");
    assert!(!s.contains("timeout"), "{s}");
    assert!(s.contains("-addonsDir \"$1/addons\""));
    assert!(s.contains("-config \"$1/server.json\""));
    assert!(s.contains("-profile \"$1/profile\""));
    assert!(s.contains("-maxFPS 60 -logStats 30000 -nothrow"));
    // BOTH flags together. That combination is the entire finding: `-addonsDir` alone
    // registers no room, `-config` alone silently runs the stale Workshop pak.
    assert!(s.contains("-addonsDir") && s.contains("-config"));
}

#[test]
fn the_deadline_expires_at_its_limit_and_never_naps_past_it() {
    use super::run_deadline::{DeadlineCheck, deadline_check};
    let poll = Duration::from_secs(5);
    // No --timeout: the loop keeps its usual pause forever.
    assert_eq!(
        deadline_check(None, Duration::from_secs(9999), poll),
        DeadlineCheck::NapFor(poll)
    );
    let limit = Some(Duration::from_secs(420));
    assert_eq!(
        deadline_check(limit, Duration::from_secs(100), poll),
        DeadlineCheck::NapFor(poll)
    );
    // Close to the limit the nap shrinks to what is left.
    assert_eq!(
        deadline_check(limit, Duration::from_secs(418), poll),
        DeadlineCheck::NapFor(Duration::from_secs(2))
    );
    assert_eq!(
        deadline_check(limit, Duration::from_secs(420), poll),
        DeadlineCheck::Expired
    );
    assert_eq!(
        deadline_check(limit, Duration::from_secs(421), poll),
        DeadlineCheck::Expired
    );
}

#[test]
fn timeout_values_read_the_way_timeout_1_reads_them() {
    use super::run_deadline::parse_run_timeout;
    assert_eq!(parse_run_timeout(""), Ok(None));
    assert_eq!(
        parse_run_timeout("0"),
        Ok(None),
        "zero disables the deadline"
    );
    assert_eq!(parse_run_timeout("420"), Ok(Some(Duration::from_secs(420))));
    assert_eq!(parse_run_timeout("30s"), Ok(Some(Duration::from_secs(30))));
    assert_eq!(parse_run_timeout("5m"), Ok(Some(Duration::from_secs(300))));
    assert_eq!(
        parse_run_timeout("1.5h"),
        Ok(Some(Duration::from_secs(5400)))
    );
    assert_eq!(
        parse_run_timeout("1d"),
        Ok(Some(Duration::from_secs(86400)))
    );
    for bad in ["abc", "-5", "5x", "m", "inf"] {
        assert!(parse_run_timeout(bad).is_err(), "{bad}");
    }
}

#[test]
fn the_join_details_are_scraped_out_of_the_engines_lines() {
    let reg = "BACKEND  : Server registered with address: 192.168.0.117:2001";
    assert_eq!(
        super::super::grep_o(r"[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+:[0-9]+", reg)[0],
        "192.168.0.117:2001"
    );
    assert_eq!(
        super::super::grep_o("[0-9]{6,}", "BACKEND  : Direct Join Code: 0207990185")[0],
        "0207990185"
    );
}
