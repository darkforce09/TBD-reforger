//! The console command's arguments and outcome keep their exact JSON and refuse unknown keys.

use super::*;

#[test]
fn arguments_round_trip_byte_for_byte() {
    let arguments = ConsoleCommandArguments {
        line: "#restart".to_owned(),
    };
    let golden = r##"{"line":"#restart"}"##;
    assert_eq!(serde_json::to_string(&arguments).unwrap(), golden);
    assert_eq!(
        serde_json::from_str::<ConsoleCommandArguments>(golden).unwrap(),
        arguments
    );
}

#[test]
fn outcome_round_trips_byte_for_byte() {
    let outcome = ConsoleCommandOutcome {
        response: "Players: 3".to_owned(),
        response_truncated: false,
    };
    let golden = r#"{"response":"Players: 3","response_truncated":false}"#;
    assert_eq!(serde_json::to_string(&outcome).unwrap(), golden);
    assert_eq!(
        serde_json::from_str::<ConsoleCommandOutcome>(golden).unwrap(),
        outcome
    );
}

#[test]
fn unknown_or_missing_keys_are_refused() {
    for invalid in [r##"{"line":"#kick 1","target":"x"}"##, "{}"] {
        assert!(
            serde_json::from_str::<ConsoleCommandArguments>(invalid).is_err(),
            "{invalid}"
        );
    }
    for invalid in [
        r#"{"response":"ok","response_truncated":false,"extra":1}"#,
        r#"{"response":"ok"}"#,
    ] {
        assert!(
            serde_json::from_str::<ConsoleCommandOutcome>(invalid).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn limits_match_the_schema() {
    assert_eq!(ConsoleCommandArguments::LINE_MAX_BYTES, 256);
    assert_eq!(ConsoleCommandOutcome::RESPONSE_MAX_BYTES, 4096);
}
