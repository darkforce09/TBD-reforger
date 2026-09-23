//! Validate actual /me responses against the schema, generated types, and frontend golden.
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use website_api::{
    core::{application_state::AppState, configuration::Config, database, http_router},
    identity_and_access::models::{
        current_profile::CurrentProfileResponse, generated::current_profile as generated,
    },
};
mod common;
const GOLDEN: &str = include_str!("../../frontend/tests/fixtures/api/GET__me.json");
const SCHEMA: &str =
    include_str!("../../../../contracts_v2/definitions/current-profile.schema.json");

#[tokio::test]
async fn current_profile_handler_schema_generated_types_and_frontend_golden_agree() {
    let url = common::require_test_database_url().unwrap();
    let pool = database::connect(&url).await.unwrap();
    database::migrate(&pool).await.unwrap();
    let state = AppState::new(pool, Config::for_tests(url, "current-profile-contract"));
    let expected: Value = serde_json::from_str(GOLDEN).unwrap();
    let user = &expected["user"];
    let actor = user["discord_id"].as_str().unwrap();
    let token =
        common::access_token(&state, "current_profile_contract", actor, "admin", true).await;
    sqlx::query(
        "UPDATE users SET username=$2, discord_handle=$3, avatar_url=$4, arma_id=$5,
        arma_character=$6, total_deployments=$7, attendance_rate=$8, created_at=$9,
        updated_at=$10, last_login_at=$11 WHERE discord_id=$1",
    )
    .bind(actor)
    .bind(user["username"].as_str().unwrap())
    .bind(user["discord_handle"].as_str().unwrap())
    .bind(user["avatar_url"].as_str().unwrap())
    .bind(user["arma_id"].as_str().unwrap())
    .bind(user["arma_character"].as_str().unwrap())
    .bind(user["total_deployments"].as_i64().unwrap())
    .bind(user["attendance_rate"].as_f64().unwrap())
    .bind(
        user["created_at"]
            .as_str()
            .unwrap()
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap(),
    )
    .bind(
        user["updated_at"]
            .as_str()
            .unwrap()
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap(),
    )
    .bind(
        user["last_login_at"]
            .as_str()
            .unwrap()
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap(),
    )
    .execute(&state.pool)
    .await
    .unwrap();
    // The displayed rate comes from decided facts: 37 attended of 40 past observations = 92.5%.
    sqlx::query("WITH fixture_mission AS (
        INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status)
        VALUES ('Profile contract', $1, 'everon', 'pve_coop', 1, 'live') RETURNING id
    ), fixture_events AS (
        INSERT INTO events(start_time, created_by) SELECT now() - interval '2 days', $1 FROM generate_series(1,40) RETURNING id
    ), attachments AS (
        INSERT INTO event_missions(event_id, mission_id, start_time)
        SELECT e.id, m.id, now() - interval '2 days' FROM fixture_events e CROSS JOIN fixture_mission m RETURNING id, event_id
    ), allocations AS (
        INSERT INTO event_participant_allocations(event_id, discord_id, quota_kind)
        SELECT id, $1, 'legacy_unclassified' FROM fixture_events RETURNING id, event_id
    ), observations AS (SELECT attachments.id, allocations.id AS allocation, row_number() OVER (ORDER BY attachments.id) AS position
        FROM attachments JOIN allocations USING (event_id))
    INSERT INTO event_registrations(event_mission_id, discord_id, reservation_state, attendance_state, legacy_attendance_state, allocation_id)
    SELECT id, $1, 'legacy_unknown',
        CASE WHEN position <= 37 THEN 'attended'::registration_state ELSE 'no_show'::registration_state END,
        CASE WHEN position <= 37 THEN 'attended'::registration_state ELSE 'no_show'::registration_state END, allocation FROM observations")
        .bind(actor).execute(&state.pool).await.unwrap();
    let response = http_router::router(state)
        .oneshot(
            Request::builder()
                .uri("/api/v1/me")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let actual: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 64 * 1024).await.unwrap()).unwrap();
    assert_eq!(
        actual, expected,
        "a stale golden must fail against the current HTTP handler"
    );
    assert_profile_contract(&actual);
}

fn assert_profile_contract(value: &Value) {
    let schema: Value = serde_json::from_str(SCHEMA).unwrap();
    let validator = jsonschema::options()
        .should_validate_formats(true)
        .build(&schema)
        .unwrap();
    assert!(
        validator.is_valid(value),
        "profile does not satisfy the published schema"
    );
    let canonical: CurrentProfileResponse = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(canonical).unwrap(), *value);
    let generated: generated::CurrentProfileResponse =
        serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(generated).unwrap(), *value);
}

#[test]
fn current_profile_boundaries_and_invalid_shapes_are_observable() {
    let schema: Value = serde_json::from_str(SCHEMA).unwrap();
    let validator = jsonschema::options()
        .should_validate_formats(true)
        .build(&schema)
        .unwrap();
    let baseline: Value = serde_json::from_str(GOLDEN).unwrap();
    for role in ["guest", "enlisted", "leader", "mission_maker", "admin"] {
        for flag in [true, false] {
            let mut value = baseline.clone();
            value["user"]["role"] = role.into();
            value["membership_stale"] = flag.into();
            value["membership_override_active"] = flag.into();
            value["can_manage_membership_override"] = flag.into();
            value["user"]["arma_id"] = Value::Null;
            value["user"]["attendance_rate"] = 0.0.into();
            value["user"]["total_deployments"] = 0.into();
            value["user"]
                .as_object_mut()
                .unwrap()
                .remove("last_login_at");
            assert_profile_contract(&value);
        }
    }
    for field in [
        "membership_stale",
        "membership_override_active",
        "can_manage_membership_override",
    ] {
        for invalid in [Value::Null, 0.into(), "false".into()] {
            let mut value = baseline.clone();
            value[field] = invalid;
            assert!(!validator.is_valid(&value));
            assert!(serde_json::from_value::<generated::CurrentProfileResponse>(value).is_err());
        }
        let mut value = baseline.clone();
        value.as_object_mut().unwrap().remove(field);
        assert!(!validator.is_valid(&value));
    }
    let mut unknown = baseline.clone();
    unknown["unexpected"] = true.into();
    assert!(!validator.is_valid(&unknown));
    assert!(serde_json::from_value::<generated::CurrentProfileResponse>(unknown).is_err());
    for (field, invalid) in [("role", "owner"), ("created_at", "yesterday")] {
        let mut value = baseline.clone();
        value["user"][field] = invalid.into();
        assert!(!validator.is_valid(&value));
    }
}
