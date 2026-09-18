use super::*;

fn tmp(tag: &str) -> RunPaths {
    let d = std::env::temp_dir().join(format!("tbd-rps-ut-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    RunPaths::new(d.to_str().unwrap())
}

/// A host that is in a container with no bridge — every probe answers `Unknown`.
fn unreachable() -> Host {
    Host::detect().broken()
}

#[test]
fn unknown_is_not_dead_and_has_no_bool_shortcut() {
    assert!(!Probe::Unknown.confirmed_gone());
    assert!(!Probe::Alive.confirmed_gone());
    assert!(Probe::Dead.confirmed_gone());
    // Zombie counts as gone — see `probe_group`: it holds no sockets, and folding it into
    // `alive` would make death permanently unconfirmable.
    assert!(Probe::Zombie.confirmed_gone());
}

#[test]
fn an_empty_pgid_is_unknown_never_dead() {
    assert_eq!(probe_group(&Host::detect(), ""), Probe::Unknown);
}

#[test]
fn a_broken_bridge_probes_unknown_not_dead() {
    // THE T-608 REGRESSION, at the unit level. `hostrun kill -0 … || return 0` produced `dead`
    // here, and that single misreading manufactured both halves of the orphan.
    assert_eq!(probe_group(&unreachable(), "424242"), Probe::Unknown);
}

#[test]
fn no_pidfile_is_a_clean_kill_run_but_not_a_claim_of_death() {
    let p = tmp("nopid");
    assert!(kill_run(&p, &unreachable()).is_ok());
    let _ = std::fs::remove_dir_all(&p.run_dir);
}

#[test]
fn kill_run_keeps_the_pidfile_when_it_cannot_confirm() {
    // The 15 s of polling this walks through is the price of pinning the property that matters
    // most: an unconfirmable death must not delete the only handle on the group.
    let p = tmp("unconfirmed");
    std::fs::write(&p.pidfile, "424242\n").unwrap();
    match kill_run(&p, &unreachable()) {
        Err(pgid) => assert_eq!(pgid, "424242"),
        Ok(()) => panic!("returned success over a group it never examined — the T-608 defect"),
    }
    assert!(
        Path::new(&p.pidfile).is_file(),
        "the pidfile is the only handle on a live group and must survive"
    );
    let _ = std::fs::remove_dir_all(&p.run_dir);
}

#[test]
fn unreachable_bridge_refuses_to_stage_rather_than_guessing() {
    // The `Probe::Unknown` arm of `assert_no_live_server`. NOT reachable from the CLI on this
    // machine — the `require_host` preflight fires first (baseline e06) — so it is only ever
    // exercised here. Named in the port's report as such.
    let p = tmp("unkstate");
    std::fs::write(&p.pidfile, "424242\n").unwrap();
    let o = Opts::defaults("/h");
    match check_no_live_server(&p, &unreachable(), &o) {
        LiveVerdict::Refuse { code, message } => {
            assert_eq!(code, 1);
            let text = message.join("\n");
            assert!(text.contains("could not reach the"), "{text}");
            assert!(
                text.contains("'I cannot tell' is not 'it is dead'"),
                "{text}"
            );
            assert!(text.contains("424242"), "{text}");
        }
        LiveVerdict::Clear => panic!("staged over a server it could not rule out"),
    }
    assert!(
        Path::new(&p.pidfile).is_file(),
        "an unknown state must not clear the pidfile"
    );
    let _ = std::fs::remove_dir_all(&p.run_dir);
}

#[test]
fn an_empty_pidfile_clears_the_way() {
    let p = tmp("emptypid");
    std::fs::write(&p.pidfile, "  \n").unwrap();
    assert!(matches!(
        check_no_live_server(&p, &unreachable(), &Opts::defaults("/h")),
        LiveVerdict::Clear
    ));
    let _ = std::fs::remove_dir_all(&p.run_dir);
}

#[test]
fn the_lock_is_exclusive_and_released_on_drop() {
    let p = tmp("lock");
    let o = Opts::defaults("/h");
    {
        let _g = claim_lock(&p, &o).expect("first claim");
        assert!(Path::new(&p.lockdir).is_dir());
        // A second claim by a LIVE owner (this very process) must refuse.
        match claim_lock(&p, &o) {
            Err(code) => assert_eq!(code, 1),
            Ok(_) => panic!("two instances took the same run dir — the F5 orphan bug"),
        }
    }
    assert!(
        !Path::new(&p.lockdir).exists(),
        "Drop must release the lock, as bash's `trap release_lock EXIT` did"
    );
    let _ = std::fs::remove_dir_all(&p.run_dir);
}

#[test]
fn a_stale_lock_is_taken_over() {
    let p = tmp("stale");
    std::fs::create_dir_all(&p.lockdir).unwrap();
    // pid 1 is alive but 999999 will not be; bash's own test used the same trick.
    std::fs::write(format!("{}/owner", p.lockdir), "999999\n").unwrap();
    let g = claim_lock(&p, &Opts::defaults("/h")).expect("stale lock must be taken over");
    let owner = std::fs::read_to_string(format!("{}/owner", p.lockdir)).unwrap();
    assert_eq!(owner.trim(), std::process::id().to_string());
    drop(g);
    let _ = std::fs::remove_dir_all(&p.run_dir);
}

#[test]
fn an_owner_file_that_never_fills_is_stale_not_a_refusal() {
    // The 20 x 0.1 s retry window: an empty owner after 2 s means the writer died mid-claim.
    let p = tmp("emptyowner");
    std::fs::create_dir_all(&p.lockdir).unwrap();
    std::fs::write(format!("{}/owner", p.lockdir), "").unwrap();
    let g = claim_lock(&p, &Opts::defaults("/h"));
    assert!(
        g.is_ok(),
        "an empty owner file must not deadlock the run dir"
    );
    let _ = std::fs::remove_dir_all(&p.run_dir);
}

#[test]
fn local_liveness_handles_junk_owners() {
    assert!(local_pid_is_alive(&std::process::id().to_string()));
    assert!(!local_pid_is_alive("abc"));
    assert!(!local_pid_is_alive(""));
    assert!(
        !local_pid_is_alive("0"),
        "pgid 0 means our own group — never a lock owner"
    );
    assert!(!local_pid_is_alive("-1"));
}

#[test]
fn stray_warning_names_the_group_the_ports_and_the_pidfile() {
    let p = RunPaths::new("/run/dir");
    let mut o = Opts::defaults("/h");
    o.game_port = "2001".into();
    o.a2s_port = "17777".into();
    let text = stray_warning(&p, &Host::detect(), &o, "31337").join("\n");
    assert!(text.contains("process group: 31337"));
    assert!(text.contains("kill -9 -- -31337"));
    assert!(text.contains("holds UDP 2001 / 17777"));
    assert!(text.contains("rm -f '/run/dir/server.pid'"));
    assert!(text.contains("pgrep -af '[A]rmaReforgerServer'"));
}

#[test]
fn read_pgid_strips_all_whitespace() {
    let p = tmp("readpgid");
    std::fs::write(&p.pidfile, " 12 34 \n").unwrap();
    assert_eq!(read_pgid(&p.pidfile), "1234");
    assert_eq!(read_pgid("/nonexistent/pid"), "");
    let _ = std::fs::remove_dir_all(&p.run_dir);
}
