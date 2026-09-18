use super::*;

/// The minimum source that satisfies the contract: every pin present, no lie anywhere.
///
/// Built from `PINS` rather than copied from the real file so that adding a pin cannot leave
/// these tests quietly asserting against a stale fixture.
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
    // The T-556/T-620 defect in test form: a ban that compares nothing reports clean. Each
    // lie must break the contract, or the ban guarding it is decoration.
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

/// A rewrite that deletes the banner outright removes the lies too. Bans alone would call
/// that clean; the pins are the half that does not.
#[test]
fn an_empty_source_fails_on_pins_not_bans() {
    let broken = assert_contract("", "t").unwrap();
    assert_eq!(
        broken.len(),
        PINS.len(),
        "expected exactly the pins to fail"
    );
}

/// The fail-closed direction: a moved or deleted target is not a pass.
#[test]
fn a_missing_target_does_not_read_as_pass() {
    assert_eq!(verify_t296(Path::new("/nonexistent/tbd-853")).unwrap(), 1);
}
