//! Unit pins for the counters contract: nested `counters` is authoritative, flat top-level keys
//! fold into a complete scoreline only when it is absent, and an identity-only row writes nothing.

use super::*;
use serde_json::json;

/// A minimal decodable player line — identity and role only — merged with whatever extra keys
/// the case under test wants to state.
fn core_player(extra: serde_json::Value) -> PlayerStatInput {
    let mut v = json!({
        "arma_id": "a1",
        "role_played": "SL",
        "source_event_id": "e1",
    });
    if let Some(obj) = v.as_object_mut()
        && let Some(extra_obj) = extra.as_object()
    {
        for (k, val) in extra_obj {
            obj.insert(k.clone(), val.clone());
        }
    }
    serde_json::from_value(v).expect("PlayerStatInput decodes")
}

/// a lone top-level `deaths` folds into a complete scoreline.
#[test]
fn flat_deaths_folds_when_nested_absent() {
    let p = core_player(json!({ "deaths": 3 }));
    let c = p.effective_counters().expect("flat deaths must fold");
    assert_eq!(c.deaths, 3);
    assert_eq!(c.kills, 0);
    assert!(!c.is_command);
    assert_eq!(c.command_win, None);
}

/// a full flat scoreline folds every field, including kills.
#[test]
fn flat_kills_fold_into_nested() {
    let p = core_player(json!({
        "kills": 17,
        "deaths": 3,
        "team_kills": 1,
        "longest_kill_m": 842,
        "vehicles_destroyed": 4,
        "is_command": true,
        "command_win": true
    }));
    let c = p.effective_counters().expect("flat scoreline must fold");
    assert_eq!(c.kills, 17);
    assert_eq!(c.deaths, 3);
    assert_eq!(c.team_kills, 1);
    assert_eq!(c.longest_kill_m, 842);
    assert_eq!(c.vehicles_destroyed, 4);
    assert!(c.is_command);
    assert_eq!(c.command_win, Some(true));
}

/// nested wins when both shapes are present (no double count).
#[test]
fn nested_counters_win_over_conflicting_flat() {
    let p = core_player(json!({
        "kills": 99,
        "deaths": 99,
        "counters": {
            "kills": 1,
            "deaths": 0,
            "team_kills": 0,
            "longest_kill_m": 0,
            "vehicles_destroyed": 0,
            "is_command": false,
            "command_win": true
        }
    }));
    let c = p.effective_counters().expect("nested must win");
    assert_eq!(c.kills, 1);
    assert_eq!(c.deaths, 0);
    assert_eq!(c.command_win, Some(true));
}

/// identity-only (no nested, no flat keys) still writes nothing.
#[test]
fn identity_only_does_not_fold() {
    let p = core_player(json!({}));
    assert!(p.effective_counters().is_none());
    assert!(p.counters.is_none());
}
