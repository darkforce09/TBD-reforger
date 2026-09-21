use super::*;

fn clean_source() -> String {
    let mut s = String::from("//! header\n");
    for pin in PINS {
        s.push_str(&format!("//! {pin}\n"));
    }
    s
}

#[test]
fn a_clean_source_holds() {
    assert!(assert_contract(&clean_source(), "t").unwrap().is_empty());
}

#[test]
fn every_ban_is_discriminating() {
    // The T-556 defect: a ban that compares nothing reports clean. Each lie must break it.
    for lie in LIES {
        let src = format!("{}//! {lie}\n", clean_source());
        assert!(
            !assert_contract(&src, "t").unwrap().is_empty(),
            "ban did not catch the reintroduced lie: {lie}"
        );
    }
}

#[test]
fn every_pin_is_discriminating() {
    for pin in PINS {
        let src: String = clean_source()
            .lines()
            .filter(|l| !l.contains(pin))
            .map(|l| format!("{l}\n"))
            .collect();
        assert!(
            !assert_contract(&src, "t").unwrap().is_empty(),
            "pin removal went unnoticed: {pin}"
        );
    }
}

#[test]
fn a_missing_target_does_not_read_as_pass() {
    assert_eq!(
        verify_player_identity_comments(Path::new("/nonexistent/tbd-853")).unwrap(),
        1
    );
}
