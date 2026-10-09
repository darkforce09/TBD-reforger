//! The guards on the mission library: the featured briefing fallback, the bookmark flag and the
//! thumbnail sink.

use super::{
    FEATURED_BRIEFING_FALLBACK, PLACEHOLDER_ART, card_is_bookmarked, featured_briefing_text,
    mission_art_url,
};
use frontend_api_dtos::MissionCard;
use http_url_guard::cases::IS_HTTP_URL_CASES;
use serde_json::json;
/// Whitespace-only featured briefings take the cinematic fallback; the rule is
/// trim-aware, the same one the overview and the event dossier use.
#[test]
fn featured_briefing_trims_whitespace_only_to_fallback() {
    for cleared in [None, Some(""), Some("   \n\n  "), Some("\t")] {
        assert_eq!(
            featured_briefing_text(cleared),
            FEATURED_BRIEFING_FALLBACK,
            "whitespace-only briefing must take the featured fallback ({cleared:?})"
        );
    }
    let authored = "Hold the ridge.\n\nSecond wave at H+20.";
    assert_eq!(featured_briefing_text(Some(authored)), authored);
    // Leading/trailing space alone is not emptiness — only all-whitespace is.
    assert_eq!(featured_briefing_text(Some(" Hold. ")), " Hold. ");
}

#[test]
fn card_is_bookmarked_reads_wire_extra_bool() {
    // MissionCard.bookmarked lives in `extra` until a dto promotion; the card control must
    // still see the same bool the Bookmarked scope filter uses.
    let on: MissionCard = serde_json::from_value(json!({
        "id": "1",
        "title": "t",
        "author_id": "a",
        "terrain": "everon",
        "game_mode": "pve_coop",
        "weather": "clear",
        "time_of_day": "dawn",
        "max_players": 16,
        "status": "live",
        "author_name": "x",
        "author_avatar": "",
        "bookmarked": true
    }))
    .unwrap();
    let off: MissionCard = serde_json::from_value(json!({
        "id": "1",
        "title": "t",
        "author_id": "a",
        "terrain": "everon",
        "game_mode": "pve_coop",
        "weather": "clear",
        "time_of_day": "dawn",
        "max_players": 16,
        "status": "live",
        "author_name": "x",
        "author_avatar": "",
        "bookmarked": false
    }))
    .unwrap();
    assert!(card_is_bookmarked(&on));
    assert!(!card_is_bookmarked(&off));
    assert!(
        on.extra.get("bookmarked").and_then(|v| v.as_bool()) == Some(true),
        "bookmarked must remain on the extra catch-all for MissionCard (dto owns naming)"
    );
}

#[test]
fn mission_art_falls_back_for_non_http_thumbnails() {
    let mut wrong = Vec::new();
    for (input, ok) in IS_HTTP_URL_CASES {
        let got = mission_art_url(Some(input));
        if *ok {
            if got != *input {
                wrong.push(format!("  dropped a legitimate thumb {input:?}"));
            }
        } else if got != PLACEHOLDER_ART {
            wrong.push(format!("  kept a non-http thumb {input:?} (got {got:?})"));
        }
    }
    assert!(
        wrong.is_empty(),
        "mission art sink wrong on {} of {} cases:\n{}",
        wrong.len(),
        IS_HTTP_URL_CASES.len(),
        wrong.join("\n")
    );
    assert_eq!(mission_art_url(None), PLACEHOLDER_ART);
    assert_eq!(mission_art_url(Some("")), PLACEHOLDER_ART);
}
