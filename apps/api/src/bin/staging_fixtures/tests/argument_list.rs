//! Unit tests for the flag splitting and taking of the staging fixtures command line.

use super::ArgumentList;
use crate::tool_failure::ToolFailure;

fn arguments(tokens: &[&str]) -> Result<ArgumentList, ToolFailure> {
    let tokens: Vec<String> = tokens.iter().map(|token| (*token).to_owned()).collect();
    ArgumentList::parse(&tokens)
}

fn refusal(result: Result<impl std::fmt::Debug, ToolFailure>) -> String {
    match result {
        Err(ToolFailure::Refused(reason)) => reason,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn staging_fixtures_arguments_take_values_switches_and_parsed_numbers() {
    let mut list =
        arguments(&["--instances", "5", "--apply", "--actor", "123", "--stage"]).unwrap();
    assert_eq!(
        list.required_parsed::<u32>("--instances", "a count")
            .unwrap(),
        5
    );
    assert!(list.switch("--apply").unwrap());
    assert!(!list.switch("--promote").unwrap());
    assert_eq!(list.required("--actor").unwrap(), "123");
    assert!(list.switch("--stage").unwrap());
    assert_eq!(list.optional("--secrets-root").unwrap(), None);
    list.finish().unwrap();
}

#[test]
fn staging_fixtures_arguments_refuse_stray_repeated_and_untaken_flags() {
    assert!(refusal(arguments(&["5"])).contains("unexpected argument `5`"));
    assert!(refusal(arguments(&["--apply", "--apply"])).contains("--apply is given twice"));
    let list = arguments(&["--instances", "5", "--bogus"]).unwrap();
    let reason = refusal(list.finish());
    assert!(
        reason.contains("--instances") && reason.contains("--bogus"),
        "{reason}"
    );
}

#[test]
fn staging_fixtures_arguments_refuse_misshapen_values() {
    let mut list = arguments(&["--apply", "yes", "--actor", "--instances", "five"]).unwrap();
    assert!(refusal(list.switch("--apply")).contains("takes no value"));
    assert!(refusal(list.required("--actor")).contains("--actor needs a value"));
    let reason = refusal(list.required_parsed::<u32>("--instances", "a whole number"));
    assert_eq!(reason, "--instances takes a whole number, not `five`");
    assert!(refusal(list.required("--secrets-root")).contains("--secrets-root is required"));
}
