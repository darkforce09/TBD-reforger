//! The executor kind keeps its snake_case wire spelling in both directions.

use super::*;

#[test]
fn executor_kinds_serialise_as_their_spelling_and_parse_back() {
    for (kind, spelling) in [
        (ExecutorKind::HostAgent, "host_agent"),
        (ExecutorKind::ModRuntime, "mod_runtime"),
    ] {
        assert_eq!(kind.as_str(), spelling);
        assert_eq!(ExecutorKind::parse(spelling), Some(kind));
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, format!("\"{spelling}\""));
        assert_eq!(serde_json::from_str::<ExecutorKind>(&json).unwrap(), kind);
    }
}

#[test]
fn unknown_executor_kinds_are_refused() {
    for unknown in ["", "HostAgent", "host-agent", "mod runtime", "agent"] {
        assert_eq!(ExecutorKind::parse(unknown), None, "{unknown:?}");
    }
    assert!(serde_json::from_str::<ExecutorKind>(r#""game_runtime""#).is_err());
}
