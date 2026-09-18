//! The mission armory write, which is destroy-then-rewrite, and the Event Hub dossier it
//! feeds. Every case here is a way a malformed body could have emptied a faction's armory.
//!
//! Skips without `TEST_DATABASE_URL`.

use axum::http::StatusCode;

mod common;
mod missions_support;

use missions_support::*;

/// `PUT /missions/:id/armory` is destroy-then-rewrite: the transaction opens with an
/// unconditional `DELETE FROM mission_armories WHERE mission_id = $1`. So every way the body can
/// be wrong is a way to lose the whole armory, and the armory is not versioned with the mission —
/// there is nothing to roll back to.
///
/// `#[serde(default)]` on `items` made `{}` decode as `items: []`, which is not "the caller said
/// nothing", it is "the caller said the new armory is empty". Four real rows were deleted, nothing
/// was inserted, and the answer was **200**. On base this test fails at the first `{}` assertion
/// with `200 {"data":[]}` and a row count of 0.
///
/// The three sibling vectors (no body, wrong `Content-Type`, malformed JSON) were already safe —
/// this handler kept its `map_err` — and they are asserted here so a future `.ok().unwrap_or_default()`
/// cannot quietly reopen them.
#[tokio::test]
async fn armory_survives_a_body_that_never_mentions_it() {
    let Some((app, tok)) = app_and_token("mission_maker").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = tok.as_str();
    let (_, url) = mission_with_armory(&app, t).await;
    assert_eq!(armory_len(&app, &url, t).await, 4, "seeded");

    // The headline case: a body that simply never mentions the armory.
    let (st, b) = call_ct(&app, "PUT", &url, t, Some("application/json"), Some("{}")).await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "`{{}}` must not be a wholesale delete: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(
        json(&b)["error"],
        "items is required, and every item needs a faction and an item_name"
    );
    assert_eq!(armory_len(&app, &url, t).await, 4, "`{{}}` kept the rows");

    // No body at all.
    let (st, _) = call_ct(&app, "PUT", &url, t, None, None).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "missing body");
    assert_eq!(
        armory_len(&app, &url, t).await,
        4,
        "missing body kept the rows"
    );

    // A well-formed body the extractor refuses because the header is wrong.
    let (st, _) = call_ct(
        &app,
        "PUT",
        &url,
        t,
        Some("text/plain"),
        Some(r#"{"items":[]}"#),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "wrong Content-Type");
    assert_eq!(
        armory_len(&app, &url, t).await,
        4,
        "wrong Content-Type kept the rows"
    );

    // Truncated JSON — the shape a dropped connection or a hand-built request produces.
    let (st, _) = call_ct(
        &app,
        "PUT",
        &url,
        t,
        Some("application/json"),
        Some(r#"{"items":["#),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "malformed JSON");
    assert_eq!(
        armory_len(&app, &url, t).await,
        4,
        "malformed kept the rows"
    );

    // The other half: clearing the armory is a legitimate thing to ask for, it just has to be
    // said out loud. Requiring the field must not cost the author the ability to empty it.
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(r#"{"items":[]}"#)).await;
    assert_eq!(
        st,
        StatusCode::OK,
        "an explicit empty armory is legitimate: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(armory_len(&app, &url, t).await, 0, "explicit clear applied");
}

/// The same mistake one level down. `item_name` was defaulted too, so `{"items":[{}]}`
/// deleted four real rows and inserted a nameless, factionless one: a blank line in the faction
/// dossier that cannot be identified or removed except by replacing the whole armory again.
/// Measured **200** on the pre-fix binary.
///
/// The guard runs before the transaction opens, so a rejected item never reaches the DELETE at
/// all rather than relying on the rollback, and it trims — otherwise `" "` is refused while
/// `" M4A1 "` is stored with its padding and never matches anything.
#[tokio::test]
async fn armory_item_without_a_name_is_refused_before_the_delete() {
    let Some((app, tok)) = app_and_token("mission_maker").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = tok.as_str();
    let (_, url) = mission_with_armory(&app, t).await;

    // An item with no fields at all fails to decode — `item_name` and `faction` are both
    // required at the type level — so this one is caught by the extractor, not the positional
    // guard below.
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(r#"{"items":[{}]}"#)).await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "a nameless item must not replace the armory: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(
        json(&b)["error"],
        "items is required, and every item needs a faction and an item_name"
    );
    assert_eq!(armory_len(&app, &url, t).await, 4, "rows untouched");

    // A whitespace-only name decodes fine and is the same lie, so the runtime guard catches it —
    // and names which item is at fault, because a 30-item armory rejected as one opaque 400 is a
    // bug report, not a diagnostic.
    let padded =
        r#"{"items":[{"faction":"USA","item_name":"M4A1"},{"faction":"USA","item_name":"   "}]}"#;
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(padded)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "blank name");
    assert_eq!(json(&b)["error"], "items[1].item_name is required");
    assert_eq!(armory_len(&app, &url, t).await, 4, "rows untouched");

    // A real name that arrived with padding is accepted and stored trimmed, so the stored value
    // agrees with the value the guard tested.
    let ok = r#"{"items":[{"faction":"USA","category":"rifle","item_name":"  M4A1  ","quantity":2,"sort_order":0}]}"#;
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(ok)).await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json(&b)["data"][0]["item_name"], "M4A1");
}

/// `faction` is the Event Hub's **join key**, not a presentation hint, and it was both
/// `#[serde(default)]` and bound untrimmed two lines above the correctly-trimmed `item_name`.
///
/// The harm is not "a column has a space in it". `get_event` groups the armory by `faction`
/// (`events.rs:796`) and the SPA matches those groups against the mission's `orbat_slots` factions
/// by exact equality (`event_hub.rs:415`) — a different table. A `faction` that does not match one
/// byte-for-byte renders a dossier card with **no items**: the author gets 200 and their own value
/// echoed back, the players get an empty armory.
///
/// Measured on the pre-fix binary, ORBAT declaring `USA`, four seeded rows:
/// - `faction: "  USA  "` → **200**, stored `"  USA  "`, the USA card rendered **0** items.
/// - `{"items":[{"item_name":"M4A1"}]}` → **200**, stored `""`, the USA card rendered **0** items.
///
/// The second needs **no whitespace at all** — it is the `#[serde(default)]`, so a trim-only fix
/// would have looked like it worked while leaving that half fully broken.
///
/// **A padded `faction` is refused, not trimmed**, and that is the whole point. The other side of
/// the join is written verbatim (`events.rs:391` ← `orbat.rs:23-25`, itself defaulted and
/// untrimmed), so trimming only here would make the two sites *disagree*: measured on the pre-fix
/// binary, an ORBAT declaring `"  USA  "` with an armory row `"  USA  "` renders **correctly
/// today**, and a unilateral trim turns that into 0 items. Refusing keeps the stored bytes
/// exactly what the caller sent, which agrees with the other side whether or not it ever
/// starts trimming.
#[tokio::test]
async fn armory_faction_is_the_event_hub_join_key() {
    let Some((app, tok)) = app_and_token("admin").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = tok.as_str();
    let (url, eid) = mission_in_event_with_orbat_faction(&app, t, "USA").await;

    // The measurement is only meaningful if the join works when both sides agree.
    assert_eq!(
        event_hub_cards(&dossier(&app, &eid, t).await),
        vec![("USA".to_string(), 2)],
        "baseline: the USA card renders its two seeded rows"
    );

    // Half one — the `#[serde(default)]`, reachable with no whitespace anywhere. Pre-fix this
    // answered 200, stored `""`, and emptied the card.
    let defaulted = r#"{"items":[{"category":"rifle","item_name":"M4A1","sort_order":0}]}"#;
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(defaulted)).await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "an item that never names a faction must not replace the armory: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(
        json(&b)["error"],
        "items is required, and every item needs a faction and an item_name"
    );
    assert_eq!(armory_len(&app, &url, t).await, 4, "rows untouched");

    // Half two — padding. Refused rather than canonicalised, so the stored bytes never diverge
    // from what `orbat_slots` holds.
    let padded =
        r#"{"items":[{"faction":"  USA  ","category":"rifle","item_name":"M4A1","sort_order":0}]}"#;
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(padded)).await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "a padded faction must not be stored: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(
        json(&b)["error"],
        "items[0].faction must not have leading or trailing whitespace"
    );
    assert_eq!(armory_len(&app, &url, t).await, 4, "rows untouched");

    // Whitespace-only decodes fine and is the same lie as no faction, so the runtime guard takes
    // it — and names which item, because a 30-item armory rejected as one opaque 400 is a bug
    // report, not a diagnostic.
    let blank =
        r#"{"items":[{"faction":"USA","item_name":"M4A1"},{"faction":"   ","item_name":"AT4"}]}"#;
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(blank)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "blank faction");
    assert_eq!(json(&b)["error"], "items[1].faction is required");
    assert_eq!(armory_len(&app, &url, t).await, 4, "rows untouched");

    // The dossier still populates for a body that states the faction the ORBAT actually declares —
    // requiring the field must not cost the author a working armory.
    let ok = r#"{"items":[
        {"faction":"USA","category":"rifle","item_name":"  M4A1  ","quantity":24,"sort_order":0},
        {"faction":"USA","category":"launcher","item_name":"AT4","quantity":6,"sort_order":1},
        {"faction":"USSR","category":"rifle","item_name":"AK-74","quantity":30,"sort_order":2}]}"#;
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(ok)).await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json(&b)["data"][0]["faction"], "USA", "stored verbatim");
    assert_eq!(json(&b)["data"][0]["item_name"], "M4A1", "name trimmed");
    assert_eq!(
        event_hub_cards(&dossier(&app, &eid, t).await),
        vec![("USA".to_string(), 2)],
        "the USA card renders its two rows again, and does not absorb USSR's"
    );

    // And the guard is `!= trim()`, not "contains a space": a faction whose name legitimately has
    // interior whitespace is stored byte-identical. An over-strict fix fails here.
    let interior =
        r#"{"items":[{"faction":"US Army","category":"rifle","item_name":"M4A1","sort_order":0}]}"#;
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(interior)).await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json(&b)["data"][0]["faction"], "US Army");
}
