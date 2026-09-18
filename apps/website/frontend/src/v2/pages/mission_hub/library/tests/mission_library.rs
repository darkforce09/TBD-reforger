//! The guards on the mission library: the featured briefing, the maker affordance, the
//! bookmark control and the thumbnail sinks.

use super::{
    author_avatar_img_src, bookmark_api_path, card_is_bookmarked, featured_briefing_text,
    mission_art_url, FEATURED_BRIEFING_FALLBACK, PLACEHOLDER_ART,
};
use crate::v2::core::api::dto::MissionCard;
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

/// Source ratchet — ban the emptiness check without a trim, which left whitespace-only
/// briefings on the featured hero. Pin the trim-aware helper arm.
#[test]
fn featured_briefing_source_ratchet_requires_trim() {
    let src = crate::v2::core::test_support::pins::mission_library_source();
    assert!(
        src.contains("featured_briefing_text(f.briefing.as_deref())"),
        "featured hero must route briefing through featured_briefing_text"
    );
    // concat! so this test body does not match itself.
    let old_filter = concat!(".filter(|b| !b.", "is_empty())");
    assert!(
        !src.contains(old_filter),
        "is_empty-only briefing filter must not return — whitespace-only briefings \
             would keep the empty string on the featured hero instead of the fallback"
    );
    let old_arm = concat!("Some(b) if !b.", "is_empty()");
    assert!(
        !src.contains(old_arm),
        "match-arm !b.is_empty() without trim must not return on featured briefing paths"
    );
    let trim_arm = concat!("Some(b) if !b.trim().", "is_empty()");
    assert!(
        src.contains(trim_arm),
        "featured_briefing_text must keep the trim-aware match arm"
    );
}

/// New Mission must not use the browse-mode role check, which treats a page that has
/// not bootstrapped as permitted. Goes red if the one-shot store read returns or the
/// authenticated helper and memo wiring is dropped.
#[test]
fn maker_affordance_uses_authed_reactive_role() {
    let src = crate::v2::core::test_support::pins::mission_library_source();
    assert!(
        src.contains("has_min_role_authed"),
        "Mission Library maker gate must use has_min_role_authed (not browse-mode None=>true)"
    );
    assert!(
        src.contains("Memo::new(move |_|")
            && src.contains(
                "has_min_role_authed(store.user.get().map(|u| u.role), Role::MissionMaker)"
            ),
        "is_maker must be a Memo that re-reads AuthStore.user after bootstrap"
    );
    // The old one-shot browse-mode path must stay gone. Split the needle so this assert's
    // own source text cannot false-red the include_str scan.
    let one_shot = format!("store.has_min_role({}::MissionMaker)", "Role");
    assert!(
        !src.contains(&one_shot),
        "one-shot store.has_min_role(MissionMaker) freezes pre-bootstrap None as maker"
    );
}

/// The Bookmarked tab is dead unless the library both renders a control and calls the
/// bookmark route. The guards go red under perturbation: delete the path helper, the
/// test identifiers, or either of the two request arms.
#[test]
fn bookmark_control_and_handlers_are_wired() {
    let src = crate::v2::core::test_support::pins::mission_library_source();
    assert!(
        src.contains("data-testid=\"mission-bookmark-toggle\""),
        "bookmark control must be present on the Mission Library surface \
             (perturbation: remove data-testid=\"mission-bookmark-toggle\")"
    );
    assert!(
        src.contains("Bookmark mission"),
        "bookmark control needs an accessible name when off"
    );
    assert!(
        src.contains("Remove bookmark"),
        "bookmark control needs an accessible name when on"
    );
    assert!(
        src.contains(r#"name="bookmark""#),
        "bookmark control must render the bookmark Material icon"
    );
    // POST when off — must not be a toast-only stub.
    assert!(
        src.contains("api_post_ok(store, &path, serde_json::json!({}))")
            && src.contains("bookmark_api_path"),
        "toggling on must POST via api_post_ok(bookmark_api_path(...))"
    );
    // DELETE when on.
    assert!(
        src.contains("api_delete(store, &path)"),
        "toggling off must DELETE via api_delete"
    );
    assert_eq!(
        bookmark_api_path("abc"),
        "/missions/abc/bookmark",
        "path helper must match Axum POST|DELETE /missions/{{id}}/bookmark"
    );
    // The route really is registered on the live router, not only called from here. Named
    // `route_tables` rather than `src`, which this test already binds to the page source.
    let route_tables = crate::v2::core::test_support::fixtures::api_route_source();
    let route_tables: &str = &route_tables;
    assert!(
        route_tables.contains(r#""/missions/{id}/bookmark""#),
        "the api_v2 domain route tables must still register /missions/{{id}}/bookmark"
    );
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

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../shared/is_http_url_cases.rs"
));

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

#[test]
fn author_avatar_emits_src_only_for_http_urls() {
    let mut wrong = Vec::new();
    for (input, should_img) in IS_HTTP_URL_CASES {
        match (author_avatar_img_src(input), should_img) {
            (Some(_), false) => wrong.push(format!("  RENDERED AN IMG FOR {input:?}")),
            (None, true) => wrong.push(format!("  refused a legitimate avatar {input:?}")),
            _ => {}
        }
    }
    assert!(
        wrong.is_empty(),
        "mission author avatar sink wrong on {} of {} cases:\n{}",
        wrong.len(),
        IS_HTTP_URL_CASES.len(),
        wrong.join("\n")
    );
}
