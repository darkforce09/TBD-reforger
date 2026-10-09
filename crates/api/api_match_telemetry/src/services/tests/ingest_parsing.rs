//! Unit coverage for the shared ingest parsers: the terrain and UUID helpers, the two-state
//! COALESCE string and the blank `role_played` guard.

use super::*;

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
