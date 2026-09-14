//! Captured-response round trips for mission cards, dossiers and the approvals queue.

use super::*;

#[test]
fn mission_detail() {
    // `json_payload` is the editor superset, deliberately opaque (`Value`).
    assert_golden::<MissionDetail>(
        golden!("GET__missions__512d8658-7025-4a70-94e9-a1b44a7aa155.json"),
        &["current_version/json_payload"],
    );
}

/// The golden above is a draft, so all three review-stamp fields are absent from it. Against an
/// option that is skipped when empty, absent round-trips to `None` and back to absent, which
/// asserts nothing at all about them. This golden was captured off a mission driven through the
/// real submit-then-reject path, so every one of the three is present and non-empty on the wire
/// and the round trip has something to be wrong about. The two tests are a matched pair: the
/// absent case above, the present case here.
#[test]
fn mission_detail_rejected_carries_the_review_stamp() {
    const G: &str = golden!("GET__missions__82b937fc-c88e-4bb9-abb3-0bef67379398.json");
    assert_golden::<MissionDetail>(G, &["current_version/json_payload"]);
    // Belt-and-braces on the round-trip: assert the golden really is the present case, so this
    // test cannot quietly decay into a second copy of the absent one if the fixture is
    // recaptured off a draft.
    let d: MissionDetail = serde_json::from_str(G).unwrap();
    assert_eq!(d.status, "rejected");
    assert!(
        d.rejection_reason.as_deref().is_some_and(|r| !r.is_empty()),
        "the rejected golden must carry a non-empty rejection_reason"
    );
    assert!(d.reviewed_by.is_some(), "and the reviewer");
    assert!(d.reviewed_at.is_some(), "and the review timestamp");
}

/// The row half of the editor's export, checked against a real captured row.
///
/// `MissionDetail::compiled_meta` feeds `flatten_mod_document_json`, whose output is pinned
/// byte-identical to `GET /missions/:id/compiled` by
/// `website-api`'s `client_twin_is_byte_identical_to_the_compiled_route`. That test supplies its
/// own meta, so it proves the *compiler* agrees; this one proves the *editor* hands it the same
/// row the server would have read. Both halves or the preview is only half-checked.
///
/// The golden is used rather than a hand-built struct on purpose: a literal fixture would be
/// written from the same misreading as the code it checks.
#[test]
fn compiled_meta_is_the_row_the_server_compiles_from() {
    const G: &str = golden!("GET__missions__512d8658-7025-4a70-94e9-a1b44a7aa155.json");
    let d: MissionDetail = serde_json::from_str(G).unwrap();
    let meta = d.compiled_meta();

    // `services::mission_compile::flatten_to_mod_document` reads `m.author_id`. The golden's
    // two author fields differ, so picking `author_name` here fails rather than coinciding.
    assert_ne!(
        d.author_id, d.author_name,
        "this golden can no longer tell author_id from author_name — recapture one that can",
    );
    assert_eq!(
        meta.author, d.author_id,
        "author is the Discord id, not the display name"
    );

    assert_eq!(meta.id, d.id);
    assert_eq!(meta.title, d.title);
    assert_eq!(meta.terrain, d.terrain);
    assert_eq!(meta.max_players, d.max_players);
    assert_eq!(meta.time_of_day, d.time_of_day);
    assert_eq!(meta.weather_preset, d.weather);
    assert_eq!(
        meta.custom_terrain_name,
        d.custom_terrain_name.clone().unwrap_or_default(),
        "an absent custom terrain is the empty string the row column holds, not a literal null",
    );

    // Nothing load-bearing may be silently empty: an all-`Default` meta would satisfy several
    // of the equalities above if the golden itself went blank.
    assert!(!meta.id.is_empty() && !meta.title.is_empty() && !meta.terrain.is_empty());
    assert!(
        meta.max_players > 0,
        "playerRange upper bound comes from here"
    );

    // And the wire round trip the wasm caller actually performs: serialize → the camelCase
    // bytes `flatten_mod_document_json` parses → back. A rename on either side breaks this.
    let json = serde_json::to_string(&meta).unwrap();
    let back: map_engine_core::mission::flatten::MissionMeta = serde_json::from_str(&json).unwrap();
    assert_eq!(
        back.max_players, meta.max_players,
        "maxPlayers survives the round trip"
    );
    assert_eq!(
        back.time_of_day, meta.time_of_day,
        "timeOfDay survives the round trip"
    );
    assert_eq!(back.weather_preset, meta.weather_preset);
    assert_eq!(back.custom_terrain_name, meta.custom_terrain_name);
    assert_eq!(back.author, meta.author);
}

/// The four `GET /missions` row keys `MissionCard` does not name — they ride the `extra`
/// catch-all, and this list is what stands between that catch-all and going blind: the moment a
/// named field stops being named, its key joins this set and both listing tests go red. Shared by
/// the two goldens so they cannot drift apart.
const MISSION_CARD_EXTRA: &[&str] = &[
    "data/*/bookmarked",
    "data/*/created_at",
    "data/*/current_version_id",
    "data/*/updated_at",
];

/// Typed as the library reads it. An untyped page body round-trips anything and proves nothing.
#[test]
fn missions_envelope() {
    assert_golden::<Paginated<MissionCard>>(golden!("GET__missions.json"), MISSION_CARD_EXTRA);
}

/// The listing golden above has three approved rows and one draft, so it pins the reviewer and
/// review time but no row carries a rejection reason — which is the one field the library card
/// renders to an author. This fixture was captured as the author after a real rejection.
///
/// The capture tooling strips query strings when it names a fixture, so a scoped listing resolves
/// to the plain listing file and this one can never shadow it. It is read by this test alone,
/// deliberately, so recapturing it cannot move any other frozen expectation.
#[test]
fn missions_envelope_rejected_card_carries_its_reason() {
    const G: &str = golden!("GET__missions__scope-mine-rejected.json");
    assert_golden::<Paginated<MissionCard>>(G, MISSION_CARD_EXTRA);
    let page: Paginated<MissionCard> = serde_json::from_str(G).unwrap();
    let rejected: Vec<&MissionCard> = page
        .data
        .iter()
        .filter(|m| m.status == "rejected")
        .collect();
    assert!(
        !rejected.is_empty(),
        "this golden exists to cover the rejected card; recapture it off a rejected mission"
    );
    for m in rejected {
        assert!(
            m.rejection_reason.as_deref().is_some_and(|r| !r.is_empty()),
            "a rejected card must carry the reason the page renders"
        );
        // The field must be a named one rather than swept into the catch-all — that is the
        // difference between proving the wire and giving the page something it can read.
        assert!(
            !m.extra.contains_key("rejection_reason"),
            "rejection_reason must be a named field, not absorbed by the `extra` catch-all"
        );
    }
}

/// Typed as the approvals queue reads it, rather than as an untyped page body.
#[test]
fn approvals_envelope() {
    assert_golden::<Paginated<ApprovalRow>>(golden!("GET__approvals.json"), &[]);
}
