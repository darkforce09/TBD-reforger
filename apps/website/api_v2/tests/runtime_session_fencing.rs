//! Runtime sessions fence heartbeats: a generation per server boot and a strictly increasing
//! sequence per session, so stale, duplicated or superseded messages never overwrite newer live
//! state; silent sessions expire and take their server offline.

use axum::http::StatusCode;
use serde_json::json;
use website_api::background_workers::runtime_session_expiry::expire_runtime_sessions;

mod common;
mod event_eligibility_support;
mod fleet_support;

use event_eligibility_support::{EventShape, Fixture};
use fleet_support::{credential, heartbeat, machine, refusal_code, register_server, session};

const SUITE: &str = "runtime_session_fencing";

async fn player_count(f: &Fixture, server: uuid::Uuid) -> (bool, i64) {
    sqlx::query_as("SELECT is_online, player_count FROM server_statuses WHERE server_id = $1")
        .bind(server)
        .fetch_one(f.pool())
        .await
        .unwrap()
}

async fn end_reason(f: &Fixture, session: uuid::Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT end_reason FROM server_runtime_sessions WHERE id = $1")
        .bind(session)
        .fetch_one(f.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn heartbeat_fencing_rejects_stale_generation_and_sequence() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let server = register_server(&f, "Fenced host").await;
    let secret = credential(&f, server, "mod_runtime").await;
    let (first, generation) = session(&f, &secret).await;
    assert_eq!(generation, 1);
    let players = |count: i64| json!({"is_online": true, "player_count": count});

    assert_eq!(
        heartbeat(&f, &secret, first, 1, 1, players(10)).await.0,
        StatusCode::OK
    );
    // A duplicated or reordered message cannot move the sequence backwards.
    let (status, duplicate) = heartbeat(&f, &secret, first, 1, 1, players(99)).await;
    assert_eq!(
        (status, refusal_code(&duplicate)),
        (StatusCode::CONFLICT, "STALE_SEQUENCE"),
        "{duplicate}"
    );
    assert_eq!(duplicate["details"]["last_sequence"], 1);
    assert_eq!(
        heartbeat(&f, &secret, first, 1, 0, players(99)).await.0,
        StatusCode::BAD_REQUEST
    );
    // A message naming another generation of the session is refused.
    let (status, wrong) = heartbeat(&f, &secret, first, 2, 2, players(99)).await;
    assert_eq!(
        (status, refusal_code(&wrong)),
        (StatusCode::CONFLICT, "STALE_GENERATION"),
        "{wrong}"
    );
    // Gaps are allowed: lost heartbeats never block later ones.
    assert_eq!(
        heartbeat(&f, &secret, first, 1, 5, players(12)).await.0,
        StatusCode::OK
    );
    assert_eq!(player_count(&f, server).await, (true, 12));

    // A new boot supersedes the session; the old runtime's late message changes nothing.
    let (second, generation) = session(&f, &secret).await;
    assert_eq!(generation, 2);
    assert_eq!(end_reason(&f, first).await.as_deref(), Some("superseded"));
    let (status, late) = heartbeat(&f, &secret, first, 1, 6, players(99)).await;
    assert_eq!(
        (status, refusal_code(&late)),
        (StatusCode::CONFLICT, "RUNTIME_SESSION_ENDED"),
        "{late}"
    );
    assert_eq!(late["details"]["end_reason"], "superseded");
    assert_eq!(
        player_count(&f, server).await,
        (true, 12),
        "the stale message wrote nothing"
    );
    assert_eq!(
        heartbeat(&f, &secret, second, 2, 1, players(20)).await.0,
        StatusCode::OK
    );
    assert_eq!(player_count(&f, server).await, (true, 20));
    let history: i64 =
        sqlx::query_scalar("SELECT count(*) FROM server_status_histories WHERE server_id = $1")
            .bind(server)
            .fetch_one(f.pool())
            .await
            .unwrap();
    assert_eq!(history, 3, "only admitted heartbeats leave samples");

    // A malformed fence is refused before any reading is considered.
    let (status, _) = f
        .call(
            &machine(&secret),
            "POST",
            &format!("/api/v1/game-runtime/sessions/{second}/heartbeats"),
            Some(json!({"player_count": 1})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let unknown = uuid::Uuid::new_v4();
    assert_eq!(
        heartbeat(&f, &secret, unknown, 2, 2, players(1)).await.0,
        StatusCode::NOT_FOUND
    );
    f.pool().close().await;
}

#[tokio::test]
async fn heartbeat_fencing_concurrent_duplicates_admit_exactly_one() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let server = register_server(&f, "Duplicate host").await;
    let secret = credential(&f, server, "mod_runtime").await;
    let (live, generation) = session(&f, &secret).await;
    let reading = json!({"is_online": true, "player_count": 7});
    let (a, b, c) = tokio::join!(
        heartbeat(&f, &secret, live, generation, 1, reading.clone()),
        heartbeat(&f, &secret, live, generation, 1, reading.clone()),
        heartbeat(&f, &secret, live, generation, 1, reading.clone()),
    );
    let admitted = [a.0, b.0, c.0]
        .iter()
        .filter(|status| **status == StatusCode::OK)
        .count();
    assert_eq!(admitted, 1, "{a:?} {b:?} {c:?}");
    for (status, body) in [a, b, c] {
        if status != StatusCode::OK {
            assert_eq!(
                (status, refusal_code(&body)),
                (StatusCode::CONFLICT, "STALE_SEQUENCE")
            );
        }
    }
    let last_sequence: i64 =
        sqlx::query_scalar("SELECT last_sequence FROM server_runtime_sessions WHERE id = $1")
            .bind(live)
            .fetch_one(f.pool())
            .await
            .unwrap();
    assert_eq!(last_sequence, 1);
    f.pool().close().await;
}

#[tokio::test]
async fn heartbeat_fencing_silent_session_expires_and_marks_server_offline() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let server = register_server(&f, "Silent host").await;
    let secret = credential(&f, server, "mod_runtime").await;
    let (silent, generation) = session(&f, &secret).await;
    assert_eq!(
        heartbeat(
            &f,
            &secret,
            silent,
            generation,
            1,
            json!({"is_online": true, "player_count": 30})
        )
        .await
        .0,
        StatusCode::OK
    );
    // A recent heartbeat keeps the session open.
    expire_runtime_sessions(&f.state).await.unwrap();
    assert_eq!(end_reason(&f, silent).await, None);
    // The runtime stops reporting for longer than the expiry window.
    sqlx::query(
        "UPDATE server_runtime_sessions SET started_at = clock_timestamp() - interval '10 minutes',
             last_heartbeat_at = clock_timestamp() - interval '2 minutes' WHERE id = $1",
    )
    .bind(silent)
    .execute(f.pool())
    .await
    .unwrap();
    assert!(expire_runtime_sessions(&f.state).await.unwrap() >= 1);
    assert_eq!(end_reason(&f, silent).await.as_deref(), Some("expired"));
    assert_eq!(player_count(&f, server).await, (false, 30));
    assert_eq!(
        f.audit_count("server.runtime_session_expired", &server.to_string())
            .await,
        1
    );
    let (status, refused) = heartbeat(
        &f,
        &secret,
        silent,
        generation,
        2,
        json!({"is_online": true}),
    )
    .await;
    assert_eq!(
        (status, refusal_code(&refused)),
        (StatusCode::CONFLICT, "RUNTIME_SESSION_ENDED")
    );
    assert_eq!(refused["details"]["end_reason"], "expired");

    // The runtime ends its own next session; ending it again reports the same end.
    let (next, next_generation) = session(&f, &secret).await;
    assert_eq!(next_generation, generation + 1);
    let end = format!("/api/v1/game-runtime/sessions/{next}/end");
    let (status, ended) = f.call(&machine(&secret), "POST", &end, None).await;
    assert_eq!(
        (status, &ended["end_reason"]),
        (StatusCode::OK, &json!("ended_by_runtime"))
    );
    let (status, again) = f.call(&machine(&secret), "POST", &end, None).await;
    assert_eq!(
        (status, &again["end_reason"]),
        (StatusCode::OK, &json!("ended_by_runtime"))
    );
    let open: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM server_runtime_sessions WHERE server_id = $1 AND ended_at IS NULL",
    )
    .bind(server)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(open, 0);
    f.pool().close().await;
}
