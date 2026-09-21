use super::*;

#[test]
fn the_launcher_argv_is_the_one_the_engine_needs() {
    // NEVER EXECUTED BY A TEST MACHINE WITHOUT THE ENGINE, so the argv itself is the contract.
    // Flag ORDER matches the staging deploy's deliberately.
    let s = launcher_script("");
    assert!(s.contains("echo $$ > \"$1/server.pid\""));
    assert!(
        s.contains("exec  ./ArmaReforgerServer"),
        "the empty-prefix double space is bash's: {s}"
    );
    assert!(s.contains("-addonsDir \"$1/addons\""));
    assert!(s.contains("-config \"$1/server.json\""));
    assert!(s.contains("-profile \"$1/profile\""));
    assert!(s.contains("-maxFPS 60 -logStats 30000 -nothrow"));
    // BOTH flags together. That combination is the entire finding: `-addonsDir` alone
    // registers no room, `-config` alone silently runs the stale Workshop pak.
    assert!(s.contains("-addonsDir") && s.contains("-config"));
}

#[test]
fn a_timeout_becomes_a_far_side_timeout_prefix() {
    assert!(
        launcher_script("timeout -s TERM 30")
            .contains("exec timeout -s TERM 30 ./ArmaReforgerServer")
    );
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
