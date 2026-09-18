//! Source pins for the attach path: the join-key refusal must run before any slot is written,
//! and the writer must store `faction` byte-for-byte.

use website_map_engine::data::scenario::orbat::validate_faction_join_key;

const ATTACHMENT: &str = include_str!("../event_mission_attachment.rs");

/// `add_event_mission` must call the join-key guard, and `materialize_slots` must bind
/// `&sq.faction` verbatim (never `.trim()`).
#[test]
fn add_event_mission_refuses_and_materialize_stores_verbatim() {
    let production = ATTACHMENT
        .split("#[cfg(test)]")
        .next()
        .expect("production source before tests module");

    let add = production
        .split("pub async fn add_event_mission")
        .nth(1)
        .expect("add_event_mission")
        .split("\npub async fn ")
        .next()
        .expect("handler body");
    assert!(
        add.contains("validate_faction_join_key"),
        "add_event_mission must refuse bad orbat[].faction before materialize"
    );

    let mat = production
        .split("async fn materialize_slots")
        .nth(1)
        .expect("materialize_slots")
        .split("\nasync fn ")
        .next()
        .expect("fn body");
    let collapsed: String = mat.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        collapsed.contains(".bind(&sq.faction)"),
        "materialize_slots must bind faction verbatim"
    );
    assert!(
        !collapsed.contains("sq.faction.trim()") && !collapsed.contains(".trim()"),
        "materialize_slots must not trim faction (one-sided join bug)"
    );
}

/// The shared join-key guard: non-empty after trim, and equal to its own trimmed form.
#[test]
fn join_key_helper_matches_the_shared_join_key_shape() {
    assert!(validate_faction_join_key("").is_err());
    assert!(validate_faction_join_key("\t").is_err());
    assert!(validate_faction_join_key("  USA  ").is_err());
    assert!(validate_faction_join_key("USA").is_ok());
    assert!(validate_faction_join_key("US Army").is_ok());
}
