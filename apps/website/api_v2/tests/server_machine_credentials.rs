//! Machine credentials are scoped to one server and one executor, shown once, revocable one at
//! a time with attribution and reason, and can never act on another server's resources.

use axum::http::StatusCode;
use serde_json::json;

mod common;
mod event_eligibility_support;
mod fleet_support;

use event_eligibility_support::{EventShape, Fixture};
use fleet_support::{
    bind_event, credential, heartbeat, issue, machine, refusal_code, register_server, revoke,
    session, start_session,
};

const SUITE: &str = "server_machine_credentials";

async fn audit_count(f: &Fixture, action: &str, target: &str) -> i64 {
    f.audit_count(action, target).await
}

#[tokio::test]
async fn server_credentials_are_scoped_and_independently_revocable() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let server = register_server(&f, "Credential host").await;
    let (status, primary) = issue(&f, server, "mod_runtime", "Primary runtime").await;
    assert_eq!(status, StatusCode::CREATED, "{primary}");
    let primary_secret = primary["secret"].as_str().unwrap().to_owned();
    let primary_id = primary["credential"]["id"].as_str().unwrap().to_owned();
    assert!(primary_secret.starts_with("tbdm_"));
    assert_eq!(primary["credential"]["executor_kind"], "mod_runtime");
    assert_eq!(primary["credential"]["created_by"], f.admin.id.as_str());
    let (_, standby) = issue(&f, server, "mod_runtime", "Standby runtime").await;
    let standby_secret = standby["secret"].as_str().unwrap().to_owned();
    let standby_id = standby["credential"]["id"].as_str().unwrap().to_owned();
    let agent_secret = credential(&f, server, "host_agent").await;

    // Secrets are shown once: neither the listing nor the database holds them.
    let (status, listed) = f
        .call(
            &f.admin,
            "GET",
            &format!("/api/v1/servers/{server}/credentials"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_eq!(listed["items"].as_array().unwrap().len(), 3);
    for secret in [&primary_secret, &standby_secret, &agent_secret] {
        assert!(!listed.to_string().contains(secret.as_str()));
        let stored: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM server_machine_credentials WHERE secret_sha256 = $1 OR label = $1",
        )
        .bind(secret)
        .fetch_one(f.pool())
        .await
        .unwrap();
        assert_eq!(stored, 0);
    }
    assert_eq!(
        audit_count(&f, "server.credential_issued", &primary_id).await,
        1
    );

    // Both runtime credentials authenticate; the second boot supersedes the first session.
    let (first_session, first_generation) = session(&f, &primary_secret).await;
    let (second_session, second_generation) = session(&f, &standby_secret).await;
    assert_eq!((first_generation, second_generation), (1, 2));
    let reading = json!({"is_online": true, "player_count": 4});
    assert_eq!(
        heartbeat(&f, &standby_secret, second_session, 2, 1, reading.clone())
            .await
            .0,
        StatusCode::OK
    );

    // Revoking one credential leaves the other working; a repeated revocation changes nothing.
    let (status, revoked) = revoke(&f, server, &primary_id, "rotated+after+handover").await;
    assert_eq!(status, StatusCode::OK, "{revoked}");
    assert_eq!(revoked["revoke_reason"], "rotated after handover");
    assert_eq!(revoked["revoked_by"], f.admin.id.as_str());
    let (status, again) = revoke(&f, server, &primary_id, "second+attempt").await;
    assert_eq!(
        (status, &again["revoke_reason"]),
        (StatusCode::OK, &json!("rotated after handover"))
    );
    assert_eq!(
        audit_count(&f, "server.credential_revoked", &primary_id).await,
        1
    );
    let (status, refused) = start_session(&f, &primary_secret).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{refused}");
    assert_eq!(refused["error"], "machine credential revoked");
    assert_eq!(
        heartbeat(&f, &standby_secret, second_session, 2, 2, reading.clone())
            .await
            .0,
        StatusCode::OK
    );
    let first_end: Option<String> =
        sqlx::query_scalar("SELECT end_reason FROM server_runtime_sessions WHERE id = $1")
            .bind(first_session)
            .fetch_one(f.pool())
            .await
            .unwrap();
    assert_eq!(first_end.as_deref(), Some("superseded"));

    // Revoking the credential of the open session ends that session.
    assert_eq!(
        revoke(&f, server, &standby_id, "decommissioned").await.0,
        StatusCode::OK
    );
    let second_end: Option<String> =
        sqlx::query_scalar("SELECT end_reason FROM server_runtime_sessions WHERE id = $1")
            .bind(second_session)
            .fetch_one(f.pool())
            .await
            .unwrap();
    assert_eq!(second_end.as_deref(), Some("credential_revoked"));
    assert_eq!(
        heartbeat(&f, &standby_secret, second_session, 2, 3, reading)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );

    // Use is recorded; an unused credential shows none.
    let used: Vec<(String, bool)> = sqlx::query_as(
        "SELECT label, last_used_at IS NOT NULL FROM server_machine_credentials WHERE server_id = $1 ORDER BY created_at, id",
    )
    .bind(server)
    .fetch_all(f.pool())
    .await
    .unwrap();
    assert_eq!(
        used,
        vec![
            ("Primary runtime".to_owned(), true),
            ("Standby runtime".to_owned(), true),
            ("host_agent credential".to_owned(), false)
        ]
    );

    // Only administrators issue or list, and a deactivated server receives nothing and
    // authenticates nothing.
    let member = f.member("member").await;
    let forbidden = f
        .call(
            &member,
            "POST",
            &format!("/api/v1/servers/{server}/credentials"),
            Some(json!({"executor_kind": "mod_runtime", "label": "x"})),
        )
        .await;
    assert_eq!(forbidden.0, StatusCode::FORBIDDEN);
    assert_eq!(
        f.call(
            &member,
            "GET",
            &format!("/api/v1/servers/{server}/credentials"),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE servers SET is_active = false WHERE id = $1")
        .bind(server)
        .execute(f.pool())
        .await
        .unwrap();
    assert_eq!(
        issue(&f, server, "mod_runtime", "Late").await.0,
        StatusCode::CONFLICT
    );
    let (status, _) = f
        .call(
            &machine(&agent_secret),
            "POST",
            "/api/v1/game-runtime/sessions",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    f.pool().close().await;
}

#[tokio::test]
async fn server_credentials_cannot_operate_on_another_server() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let (home, foreign) = (
        register_server(&f, "Home host").await,
        register_server(&f, "Foreign host").await,
    );
    let home_secret = credential(&f, home, "mod_runtime").await;
    let foreign_secret = credential(&f, foreign, "mod_runtime").await;
    let agent_secret = credential(&f, home, "host_agent").await;
    let (foreign_session, generation) = session(&f, &foreign_secret).await;
    let reading = json!({"is_online": true});

    // Another server's session can be neither fed nor ended.
    let (status, body) = heartbeat(
        &f,
        &home_secret,
        foreign_session,
        generation,
        1,
        reading.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let (status, _) = f
        .call(
            &machine(&home_secret),
            "POST",
            &format!("/api/v1/game-runtime/sessions/{foreign_session}/end"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        heartbeat(
            &f,
            &foreign_secret,
            foreign_session,
            generation,
            1,
            reading.clone()
        )
        .await
        .0,
        StatusCode::OK
    );

    // The server identity comes from the credential, never from the body.
    let mut claimed = json!({"server_id": foreign});
    claimed["is_online"] = json!(true);
    let (status, body) =
        heartbeat(&f, &foreign_secret, foreign_session, generation, 2, claimed).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    // A roster is served only to the runtime of the server the event is bound to.
    let roster = format!("/api/v1/game-runtime/events/{}/roster", f.event);
    assert_eq!(
        f.call(&machine(&home_secret), "GET", &roster, None).await.0,
        StatusCode::FORBIDDEN,
        "unbound event"
    );
    bind_event(&f, foreign).await;
    assert_eq!(
        f.call(&machine(&home_secret), "GET", &roster, None).await.0,
        StatusCode::FORBIDDEN
    );
    let (status, served) = f
        .call(&machine(&foreign_secret), "GET", &roster, None)
        .await;
    assert_eq!(status, StatusCode::OK, "{served}");
    assert_eq!(served["eventId"], f.event.to_string());

    // Executor kinds are not interchangeable.
    let (status, body) = f
        .call(
            &machine(&agent_secret),
            "POST",
            "/api/v1/game-runtime/sessions",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(
        f.call(&machine(&agent_secret), "GET", &roster, None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );

    // Malformed, unknown and shared-service secrets authenticate nothing.
    let unknown = format!("tbdm_{}_{}", uuid::Uuid::new_v4().simple(), "ab".repeat(32));
    let tampered = format!(
        "{}{}",
        &home_secret[..home_secret.len() - 1],
        if home_secret.ends_with('0') { '1' } else { '0' }
    );
    for secret in [
        "",
        "garbage",
        unknown.as_str(),
        tampered.as_str(),
        "test-service-token",
    ] {
        let (status, body) = f.call(&machine(secret), "GET", &roster, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{secret:?}: {body}");
        assert_eq!(refusal_code(&body), "");
    }

    // A credential is revocable only through its own server.
    let home_credential: String = sqlx::query_scalar(
        "SELECT id::text FROM server_machine_credentials WHERE server_id = $1 AND executor_kind = 'mod_runtime'",
    )
    .bind(home)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(
        revoke(&f, foreign, &home_credential, "wrong+server")
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        f.call(
            &machine(&home_secret),
            "POST",
            "/api/v1/game-runtime/sessions",
            None
        )
        .await
        .0,
        StatusCode::CREATED
    );
    f.pool().close().await;
}
