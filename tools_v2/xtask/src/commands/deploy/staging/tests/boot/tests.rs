use super::*;
use std::path::PathBuf;

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("tbd-t853-boot-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn grep_after_resets_the_window_on_each_match() {
    let text = "M\n1\n2\nx\nM\na\nb\n";
    assert_eq!(grep_after(text, "M", 2), vec!["M", "1", "2", "M", "a", "b"]);
    // A second match INSIDE the window merges rather than duplicating.
    assert_eq!(grep_after("M\nM\nz\n", "M", 1), vec!["M", "M", "z"]);
    // Window exhausted -> later lines are dropped.
    assert_eq!(grep_after("M\n1\n2\n", "M", 1), vec!["M", "1"]);
}

#[test]
fn addon_check_discriminates_on_path_not_guid() {
    // ANTI-VACUITY, as a unit test rather than only inside the selftest: both logs carry the
    // SAME guid and both look healthy. Only the gproj path differs.
    let d = tmp("addon");
    let guid = "B2C3D4E5F6A78901";
    let win = d.join("win.log");
    let lose = d.join("lose.log");
    fs::write(
        &win,
        format!(
            "ENGINE : Loaded addons:\n ENGINE : gproj: '/home/sam/tbd/addons/tbd-framework/addon.gproj' guid: '{guid}'\n"
        ),
    )
    .unwrap();
    fs::write(
        &lose,
        format!(
            "ENGINE : Loaded addons:\n ENGINE : gproj: '/home/sam/tbd/profile/addons/TBDFramework_{guid}/addon.gproj' guid: '{guid}'\n"
        ),
    )
    .unwrap();
    let mut o = Out::captured();
    assert_eq!(
        assert_local_addon_won(&mut o, &win, guid, "/home/sam/tbd/addons"),
        0
    );
    let mut o = Out::captured();
    assert_eq!(
        assert_local_addon_won(&mut o, &lose, guid, "/home/sam/tbd/addons"),
        1
    );
    assert!(
        o.text()
            .contains("STAGING IS VALIDATING A BUILD IT DID NOT DEPLOY")
    );
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn last_loaded_addons_block_wins() {
    // The engine prints `Loaded addons:` twice per boot; the FINAL block is the one that ran.
    // A first block naming the Workshop copy must not save a run whose second block does too,
    // and — the case that matters — a first block naming the Workshop copy must not SINK a run
    // whose second block names the checkout.
    let d = tmp("last");
    let guid = "B2C3D4E5F6A78901";
    let log = d.join("two.log");
    fs::write(
        &log,
        format!(
            "ENGINE : Loaded addons:\n ENGINE : gproj: '/profile/addons/TBDFramework_{guid}/addon.gproj' guid: '{guid}'\n\
             ENGINE : Loaded addons:\n ENGINE : gproj: '/home/sam/tbd/addons/tbd-framework/addon.gproj' guid: '{guid}'\n"
        ),
    )
    .unwrap();
    let mut o = Out::captured();
    assert_eq!(
        assert_local_addon_won(&mut o, &log, guid, "/home/sam/tbd/addons"),
        0
    );
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn missing_log_is_not_a_pass() {
    // The whole point of the four-outcome posture: a check that could not read its input must
    // FAIL, not print green.
    let missing = Path::new("/nonexistent/tbd-t853/console.log");
    let mut o = Out::captured();
    assert_eq!(assert_local_addon_won(&mut o, missing, "X", "/a"), 1);
    assert!(
        o.text()
            .contains("The check did NOT run. This is not a pass.")
    );
    let mut o = Out::captured();
    assert_eq!(assert_room_registered(&mut o, missing), 1);
    let mut o = Out::captured();
    assert_eq!(assert_admins_configured(&mut o, missing, "1"), 1);
}

#[test]
fn non_numeric_admin_count_takes_the_else_branch() {
    // ODDITY PINNED: bash `[ "$x" -eq 0 ]` errors, returns 2, and inside an `if` condition
    // `set -e` stays quiet, so junk read as "not zero" and got printed verbatim.
    let d = tmp("admin");
    let log = d.join("ok.log");
    fs::write(
        &log,
        "BACKEND : Server config loaded.\nBACKEND : JSON is Valid\n",
    )
    .unwrap();
    let mut o = Out::captured();
    assert_eq!(assert_admins_configured(&mut o, &log, "banana"), 0);
    assert!(
        o.text().contains("carrying banana admin id(s)"),
        "{}",
        o.text()
    );
    let mut o = Out::captured();
    assert_eq!(assert_admins_configured(&mut o, &log, "0"), 0);
    assert!(
        o.text()
            .contains("WARN  config accepted, but game.admins[] is EMPTY")
    );
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn rival_precedence_matches_the_bash_if_chain() {
    let d = tmp("rival");
    let guid = "B2C3D4E5F6A78901";
    let log = d.join("b.log");
    fs::write(
        &log,
        format!(
            "ENGINE : Loaded addons:\n ENGINE : gproj: '/a/tbd-framework/addon.gproj' guid: '{guid}'\n\
             BACKEND : Server config loaded.\nBACKEND : JSON is Valid\n\
             BACKEND : Server registered with address: 1.2.3.4:2001\n"
        ),
    )
    .unwrap();
    // A measured size wins over the local stat, even when profile_dir is unreachable.
    let mut o = Out::captured();
    verify_boot_log(
        &mut o,
        &log,
        guid,
        "/a",
        "1",
        "/nonexistent",
        Some("570489"),
    );
    assert!(o.text().contains("570489 bytes"));
    // "" profile AND no measurement -> the weakest message of the four.
    let mut o = Out::captured();
    verify_boot_log(&mut o, &log, guid, "/a", "1", "", None);
    assert!(o.text().contains("rival unknown"));
    // Measured ZERO is not "unknown" — it is WEAK EVIDENCE, which is a different claim.
    let mut o = Out::captured();
    verify_boot_log(&mut o, &log, guid, "/a", "1", "", Some("0"));
    assert!(o.text().contains("WEAK EVIDENCE"));
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn guid_reads_out_of_the_real_gproj() {
    // Not a fixture: the live gproj, because the whole point of read_addon_guid is that a
    // literal drifts from the source silently.
    let root = crate::core::repository_root::test_repo_root();
    if root.join("apps/mod/tbd-framework/addon.gproj").is_file() {
        let g = read_addon_guid(&root).expect("gproj present");
        assert!(
            g.len() >= 8 && g.chars().all(|c| c.is_ascii_hexdigit()),
            "guid={g}"
        );
    }
}
