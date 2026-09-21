//! Unit and source pins for the shared ingest parsers: the terrain and UUID helpers, the
//! two-state COALESCE string, the blank `role_played` guard, and the foreign-key mapping.
//!
//! Several assertions run against handler *source* rather than only the helpers, because the
//! dangerous edit is a call site that stops calling: a helper-only suite stays green while the
//! heartbeat binds a raw `as_deref()` again or the 23503 mapping is widened to every error. The
//! source is stripped of `//` and `/* */` first so a comment naming the behaviour cannot green a
//! deleted code path.

use super::*;
use axum::http::StatusCode;

const HEARTBEAT_SRC: &str = include_str!("../server_heartbeat.rs");
const UPSERT_SRC: &str = include_str!("../match_upsert.rs");
const PARSING_SRC: &str = include_str!("../ingest_parsing.rs");

/// Drop `//` and `/* */` comments so every source pin asserts on live code only.
///
/// Shared with the sibling test modules for `server_heartbeat`, `match_upsert` and
/// `match_results`, which window their own files the same way.
pub(crate) fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < bytes.len() {
                i += 2;
            } else {
                i = bytes.len();
            }
            continue;
        }
        out.push(char::from(bytes[i]));
        i += 1;
    }
    out
}

/// Everything ahead of the file's `#[cfg(test)]` sibling-module declaration.
pub(crate) fn production_half(source: &str) -> &str {
    source
        .split("#[cfg(test)]")
        .next()
        .expect("source must carry a #[cfg(test)] tests declaration")
}

/// Collapse all runs of whitespace so a pin survives rustfmt re-wrapping.
pub(crate) fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// known terrains still map (do not break everon/arland/custom).
#[test]
fn terrain_known_pins() {
    assert_eq!(
        parse_terrain_opt(&Some("everon".into())),
        Some(TerrainType::Everon)
    );
    assert_eq!(
        parse_terrain_opt(&Some(" arland ".into())),
        Some(TerrainType::Arland)
    );
    assert_eq!(
        parse_terrain_opt(&Some("custom".into())),
        Some(TerrainType::Custom)
    );
}

/// community / unknown terrain soft-fails to None — does not 400.
#[test]
fn terrain_community_degrades_to_none() {
    assert_eq!(parse_terrain_opt(&Some("kolguyev".into())), None);
    assert_eq!(parse_terrain_opt(&Some("anizay".into())), None);
    assert_eq!(parse_terrain_opt(&Some("  ".into())), None);
    assert_eq!(parse_terrain_opt(&None), None);
}

/// present-and-blank `role_played` is a 400 — mirrors `source_match_key` / outcome
/// blank rejects. `''` and whitespace both reject.
#[test]
fn blank_role_played_is_rejected() {
    for blank in ["", "   ", "\t", "\n", " \t\n "] {
        let err = require_role_played(blank).expect_err("blank must 400");
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert!(
            err.message.contains("role_played must not be blank"),
            "unexpected message for {blank:?}: {:?}",
            err.message
        );
    }
}

/// non-blank role is accepted; trimmed form is what binds.
#[test]
fn non_blank_role_played_ok() {
    assert_eq!(require_role_played("SL").unwrap(), "SL");
    assert_eq!(
        require_role_played("  Squad Leader  ").unwrap(),
        "Squad Leader"
    );
    assert_eq!(require_role_played("Rifleman").unwrap(), "Rifleman");
}

/// malformed non-empty event_id / mission_id → 400 (not silent None).
#[test]
fn malformed_event_or_mission_id_is_bad_request() {
    for field in ["event_id", "mission_id"] {
        for junk in ["not-a-uuid", "123", "garb age", "0", "{bad}"] {
            let err = parse_uuid_opt_strict(field, &Some(junk.into()))
                .expect_err("unparseable non-empty must 400");
            assert_eq!(err.status, StatusCode::BAD_REQUEST);
            assert!(
                err.message.contains(field),
                "message must name {field}: {:?}",
                err.message
            );
        }
    }
}

/// absent / blank / valid still Ok — blank keeps (not three-state clear).
#[test]
fn event_mission_id_absent_blank_valid_ok() {
    let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    assert_eq!(parse_uuid_opt_strict("event_id", &None).unwrap(), None);
    assert_eq!(
        parse_uuid_opt_strict("event_id", &Some("".into())).unwrap(),
        None
    );
    assert_eq!(
        parse_uuid_opt_strict("mission_id", &Some("   ".into())).unwrap(),
        None
    );
    assert_eq!(
        parse_uuid_opt_strict(
            "event_id",
            &Some("550e8400-e29b-41d4-a716-446655440000".into())
        )
        .unwrap(),
        Some(id)
    );
    assert_eq!(
        parse_uuid_opt_strict(
            "mission_id",
            &Some("  550e8400-e29b-41d4-a716-446655440000  ".into())
        )
        .unwrap(),
        Some(id)
    );
}

/// `current_match_id` keeps soft three-state via `parse_uuid_opt`.
/// Absent → None (caller treats as keep); "" / whitespace → None (clear); uuid → Some;
/// unparseable present → None (clear, **not** 400 — do not tighten this helper globally).
#[test]
fn current_match_id_three_state_soft_parse() {
    let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    assert_eq!(parse_uuid_opt(&None), None);
    assert_eq!(parse_uuid_opt(&Some("".into())), None);
    assert_eq!(parse_uuid_opt(&Some("   ".into())), None);
    assert_eq!(
        parse_uuid_opt(&Some("550e8400-e29b-41d4-a716-446655440000".into())),
        Some(id)
    );
    assert_eq!(
        parse_uuid_opt(&Some("  550e8400-e29b-41d4-a716-446655440000  ".into())),
        Some(id)
    );
    // Soft degrade — same as blank clear when `set_match_id` is true at the call site.
    assert_eq!(parse_uuid_opt(&Some("not-a-uuid".into())), None);
    assert_eq!(parse_uuid_opt(&Some("123".into())), None);
}

/// COALESCE keep/clear is two-state. Whitespace must clear as `""`, never bind as a
/// third non-NULL value. `None` stays keep — do not collapse blank to `None` (that would break
/// the deliberate `""` clear).
#[test]
fn coalesce_str_two_state_no_whitespace_third() {
    assert_eq!(coalesce_str(&None), None);
    assert_eq!(coalesce_str(&Some(String::new())), Some(""));
    for blank in ["", "   ", "\t", "\n", " \t\n "] {
        assert_eq!(
            coalesce_str(&Some(blank.into())),
            Some(""),
            "whitespace {blank:?} must clear, not land as a third COALESCE state"
        );
    }
    assert_eq!(coalesce_str(&Some("  BLUFOR  ".into())), Some("BLUFOR"));
    assert_eq!(coalesce_str(&Some("Clear".into())), Some("Clear"));
    // Perturbation RED: collapsing blank → None would make this fail the clear pin.
    assert_ne!(coalesce_str(&Some("   ".into())), None);
}

/// call sites must use `coalesce_str`, not raw `as_deref()` — otherwise helper-only
/// tests stay green while COALESCE still admits `Some("   ")`.
#[test]
fn coalesce_str_bound_at_heartbeat_and_match_writes() {
    let production = production_half(HEARTBEAT_SRC);

    let hb_start = production
        .find("pub async fn ingest_server_status")
        .expect("ingest_server_status must exist");
    let hb_after = &production[hb_start..];
    let hb_end = hb_after[1..]
        .find("\npub async fn ")
        .map(|i| i + 1)
        .unwrap_or(hb_after.len());
    let hb = strip_rust_comments(&hb_after[..hb_end]);
    let hb_collapsed = collapse_ws(&hb);
    assert!(
        hb_collapsed.contains("coalesce_str(&input.ingame_time)"),
        "heartbeat must bind coalesce_str(&input.ingame_time) (fails with: as_deref)"
    );
    assert!(
        hb_collapsed.contains("coalesce_str(&input.ingame_weather)"),
        "heartbeat must bind coalesce_str(&input.ingame_weather) (fails with: as_deref)"
    );
    assert!(
        !hb_collapsed.contains("input.ingame_time.as_deref()"),
        "ingame_time must not bind raw as_deref — that reopens the whitespace third state"
    );
    assert!(
        !hb_collapsed.contains("input.ingame_weather.as_deref()"),
        "ingame_weather must not bind raw as_deref — that reopens the whitespace third state"
    );

    let upsert_production = production_half(UPSERT_SRC);
    let up_start = upsert_production
        .find("async fn upsert_match")
        .expect("upsert_match must exist");
    let up_after = &upsert_production[up_start..];
    let up_end = up_after[1..]
        .find("\nasync fn ")
        .or_else(|| up_after[1..].find("\npub(super) async fn "))
        .map(|i| i + 1)
        .unwrap_or(up_after.len());
    let up = strip_rust_comments(&up_after[..up_end]);
    let up_collapsed = collapse_ws(&up);
    assert_eq!(
        up_collapsed
            .matches("coalesce_str(&m.winning_faction)")
            .count(),
        2,
        "UPDATE + INSERT must each bind coalesce_str(&m.winning_faction)"
    );
    assert!(
        !up_collapsed.contains("m.winning_faction.as_deref()"),
        "winning_faction must not bind raw as_deref — COALESCE third-state regression"
    );
}

/// the FK constraint names this module branches on must follow `0018`'s
/// `<table>_<column>_fkey` convention.
///
/// Three of the four are for constraints that do not exist yet, so nothing at runtime can
/// catch a typo in them — a misspelled name simply never matches and the endpoint quietly
/// goes back to 500 the day the migration lands. That is the failure this pins: derive the
/// expected name from the table and column and compare, so the constant cannot drift from
/// the convention the migration will use.
#[test]
fn fk_constant_names_follow_migration_convention() {
    for (table, column, actual) in [
        ("server_statuses", "server_id", FK_STATUS_SERVER),
        ("server_statuses", "current_match_id", FK_STATUS_MATCH),
        ("matches", "event_id", FK_MATCH_EVENT),
        ("matches", "mission_id", FK_MATCH_MISSION),
    ] {
        assert_eq!(
            actual,
            format!("{table}_{column}_fkey"),
            "0018 names every one of its 25 foreign keys <table>_<column>_fkey; a constant \
             that disagrees matches nothing and silently restores the 500"
        );
    }
}

/// the mapping must **discriminate**, not blanket-4xx the database.
///
/// `foreign_key_error` is the only thing standing between "a foreign key was violated" and
/// "every `sqlx::Error` is the caller's fault". Asserted here on the source rather than only
/// over HTTP, because the dangerous edit — widening the guard to `is_foreign_key_violation`
/// alone, or worse to any `Err` — leaves every happy-path test green while turning connection
/// resets and NOT NULL breaches into 400s that tell a game-server bridge to stop retrying.
/// Perturbation: delete the `_ => return None` arm and this goes red.
#[test]
fn foreign_key_error_falls_through_to_500() {
    let production = production_half(PARSING_SRC);
    let start = production
        .find("fn foreign_key_error")
        .expect("foreign_key_error must exist");
    let after = &production[start..];
    let end = after[1..]
        .find("\n/// ")
        .or_else(|| after[1..].find("\npub async fn "))
        .map(|i| i + 1)
        .unwrap_or(after.len());
    let body = strip_rust_comments(&after[..end]);
    let collapsed = collapse_ws(&body);

    assert!(
        collapsed.contains("if !is_foreign_key_violation(e) { return None; }"),
        "must return None for any SQLSTATE that is not 23503 — a 4xx for a connection \
         reset or a NOT NULL breach is worse than the 500 it replaced"
    );
    assert!(
        collapsed.contains("_ => return None,"),
        "must return None for a 23503 raised by an unrecognised constraint — the message \
         names a parent, and it cannot name one it did not identify"
    );
    assert!(
        collapsed.contains("ApiError::bad_request"),
        "a named foreign-key violation maps to 400"
    );
    assert!(
        !collapsed.contains("ApiError::internal"),
        "the 500 must come from the caller's untouched `e.into()`, not be re-minted here"
    );
}
