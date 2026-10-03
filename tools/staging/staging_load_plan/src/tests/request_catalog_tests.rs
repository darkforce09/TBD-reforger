use serde_json::{Value, json};

use super::*;
use crate::identifiers::DiscordId;
use crate::sample_plans::{fixture_events, read_template, step};

/// Account 13 of ten fixture events: event 3, slot 1.
fn binding() -> AccountBinding {
    AccountBinding::for_account(
        13,
        &DiscordId::new("9100000000000000013"),
        &fixture_events(10, 2),
    )
}

fn write(id: &str, steps: Vec<RequestStep>) -> RequestTemplate {
    RequestTemplate {
        id: id.into(),
        class: RequestClass::JsonWrite,
        weight: 1,
        steps,
    }
}

fn refusal(mix: &[RequestTemplate]) -> String {
    format!(
        "{:#}",
        RequestCatalog::compile(mix).expect_err("the mix must be refused")
    )
}

#[test]
fn placeholders_bind_to_the_account_in_paths_and_body_strings_only() {
    let body = json!({
        "slot_id": "{slot_id}",
        "note": "account {account_index} of {discord_id}",
        "targets": [{ "event": "{event_id}", "mission": "{mission_id}" }],
        "{slot_id}": 2,
    });
    let register = step(
        HttpMethod::Post,
        "/api/v1/event-missions/{event_mission_id}/register",
        Some(body),
        &[200],
    );
    let resolved = resolve(&register, &binding());
    assert_eq!(resolved.path, "/api/v1/event-missions/em-3/register");
    let sent: Value = serde_json::from_slice(&resolved.body.expect("a body")).expect("JSON");
    assert_eq!(
        sent,
        json!({
            "slot_id": "slot-3-1",
            "note": "account 13 of 9100000000000000013",
            "targets": [{ "event": "event-3", "mission": "mission-3" }],
            "{slot_id}": 2,
        })
    );
    assert!(!resolved.conditional && !resolved.auth);
    assert_eq!(resolved.expected_statuses, vec![200]);
}

#[test]
fn a_step_expecting_304_is_conditional_and_auth_paths_are_flagged() {
    let read = step(
        HttpMethod::Get,
        "/api/v1/events?limit=20",
        None,
        &[200, 304],
    );
    let resolved = resolve(&read, &binding());
    assert!(resolved.conditional && resolved.body.is_none());
    let session = step(HttpMethod::Get, "/api/v1/auth/me", None, &[200]);
    assert!(resolve(&session, &binding()).auth);
}

#[test]
fn malformed_templates_are_refused_before_any_request() {
    let get = |path: &str| read_template("reads", path, &[200]);
    let cases: Vec<(Vec<RequestTemplate>, &str)> = vec![
        (vec![], "the request mix is empty"),
        (
            vec![get("/api/v1/events/{event}")],
            "unknown placeholder {event}",
        ),
        (vec![get("/api/v1/events/{event_id")], "never closed"),
        (vec![get("/api/v1/events/event_id}")], "closes nothing"),
        (vec![get("api/v1/events")], "must start with '/'"),
        (
            vec![get("/api/v1/game-runtime/sessions")],
            "is game traffic",
        ),
        (vec![get("/api/v1/ingest?since=0")], "is game traffic"),
        (vec![get("/api/v1/events?q=a b")], "no whitespace"),
        (
            vec![get("/api/v1/events"), get("/api/v1/me")],
            "declared twice",
        ),
        (
            vec![read_template("Reads", "/api/v1/events", &[200])],
            "lowercase",
        ),
        (
            vec![read_template("reads", "/api/v1/events", &[])],
            "at least one expected status",
        ),
        (
            vec![read_template("reads", "/api/v1/events", &[200, 200])],
            "listed twice",
        ),
        (
            vec![read_template("reads", "/api/v1/events", &[99])],
            "not an HTTP status",
        ),
        (vec![write("bookmark", vec![])], "at least one step"),
        (
            vec![write(
                "bookmark",
                vec![step(HttpMethod::Get, "/api/v1/me", None, &[200])],
            )],
            "cannot use Get",
        ),
        (
            vec![write(
                "bookmark",
                vec![step(HttpMethod::Post, "/x", None, &[304])],
            )],
            "only a JSON read",
        ),
        (
            vec![write(
                "save",
                vec![step(
                    HttpMethod::Post,
                    "/x",
                    Some(json!({ "a": ["{nope}"] })),
                    &[201],
                )],
            )],
            "{nope}",
        ),
    ];
    for (mix, expected) in cases {
        let error = refusal(&mix);
        assert!(error.contains(expected), "{expected}: {error}");
    }
    let mut weightless = get("/api/v1/events");
    weightless.weight = 0;
    assert!(refusal(&[weightless]).contains("weight"));
    let mut with_body = get("/api/v1/events");
    with_body.steps[0].body = Some(json!({}));
    assert!(refusal(&[with_body]).contains("takes no body"));
    let mut session = get("/api/v1/auth/refresh");
    session.class = RequestClass::Session;
    assert!(refusal(&[session]).contains("account rotation"));
}

#[test]
fn picks_follow_the_weights() {
    let mut light = read_template("light", "/api/v1/me", &[200]);
    light.weight = 1;
    let mut heavy = read_template("heavy", "/api/v1/events", &[200]);
    heavy.weight = 3;
    let catalog = RequestCatalog::compile(&[light, heavy]).expect("compiles");
    let draws = 4000u64;
    let stride = u64::MAX / draws;
    let mut counts = [0u64; 2];
    for draw in 0..draws {
        counts[catalog.pick(draw * stride)] += 1;
    }
    assert!(
        counts[0].abs_diff(1000) <= 2 && counts[1].abs_diff(3000) <= 2,
        "{counts:?}"
    );
    assert_eq!(catalog.pick(0), 0);
    assert_eq!(catalog.pick(u64::MAX), 1);
    assert_eq!(catalog.template_count(), 2);
}

#[test]
fn path_safe_values_are_ids() {
    assert!(is_path_safe("9f1c2e44-0b7a-4c55-9d0e-2a4b6c8d0e1f"));
    assert!(is_path_safe("slot_12"));
    for unsafe_value in ["", "a/b", "a b", "..", "a?b", "é"] {
        assert!(!is_path_safe(unsafe_value), "{unsafe_value:?}");
    }
}
