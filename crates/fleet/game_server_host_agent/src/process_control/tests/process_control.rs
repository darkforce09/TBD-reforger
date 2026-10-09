use super::*;

#[test]
fn service_unit_names_are_accepted() {
    for name in [
        "tbd-reforger.service",
        "arma_reforger@staging-2.service",
        "reforger:eu\\x2d1.service",
    ] {
        assert_eq!(SystemdUnitName::parse(name).unwrap().as_str(), name);
    }
}

#[test]
fn names_that_are_not_service_units_or_could_read_as_options_are_refused() {
    let too_long = format!("{}.service", "a".repeat(250));
    for name in [
        "",
        ".service",
        "--user.service",
        "-reforger.service",
        ".hidden.service",
        "tbd-reforger",
        "tbd-reforger.timer",
        "tbd reforger.service",
        "tbd-reforger.service;reboot",
        "tbd/reforger.service",
        "réforger.service",
        too_long.as_str(),
    ] {
        assert!(SystemdUnitName::parse(name).is_err(), "{name:?}");
    }
}

#[test]
fn each_action_names_its_verb_and_intended_state() {
    assert_eq!(
        [
            ProcessAction::Start,
            ProcessAction::Stop,
            ProcessAction::Restart
        ]
        .map(|action| (action.systemctl_verb(), action.intended_active_state())),
        [
            ("start", "active"),
            ("stop", "inactive"),
            ("restart", "active")
        ]
    );
    assert!(ProcessAction::Start.reads_state_after_dwell());
    assert!(ProcessAction::Restart.reads_state_after_dwell());
    assert!(!ProcessAction::Stop.reads_state_after_dwell());
}

#[test]
fn a_state_value_is_one_lowercase_word() {
    assert_eq!(state_value(b"active\n"), Some("active".to_owned()));
    assert_eq!(state_value(b"not-found\n"), Some("not-found".to_owned()));
    for printed in [
        &b""[..],
        b"\n",
        b"ActiveState=active\n",
        b"active\ninactive\n",
        b"\xff\xfe",
    ] {
        assert_eq!(state_value(printed), None, "{printed:?}");
    }
}
