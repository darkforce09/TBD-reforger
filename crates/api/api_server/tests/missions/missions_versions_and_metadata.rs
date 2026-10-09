//! Saving a version and the mission-row metadata that save moves: the `updated_at` clock and audit
//! row, and the weather contract on patch.

use crate::missions_support;

use axum::http::StatusCode;
use serde_json::Value;

use missions_support::*;

/// `POST /missions/:id/versions` must bump `missions.updated_at` and write an audit
/// row. Library orders by `updated_at`; approvals projects it as `submitted_at`. Before this
/// fix the handler only wrote `current_version_id`, so both clocks stayed frozen and the
/// save left no trail in `GET /admin/audit-logs`.
///
/// Perturbation RED: drop the `updated_at = now()` clause (or the `write_audit` call) and
/// either the timestamp assert or the audit-row assert fails.
#[tokio::test]
async fn create_version_bumps_updated_at_and_writes_audit() {
    let Some((app, pool, maker, admin)) = app_pool_and_tokens().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let title = format!("Version-Version-Save-{stamp}");
    let (st, b) = call(
        &app,
        "POST",
        "/api/v1/missions",
        Some(&maker),
        None,
        Some(&format!(
            r#"{{"title":"{title}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));
    let mid = json(&b)["id"].as_str().unwrap().to_string();
    let before: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT updated_at FROM missions WHERE id = $1::uuid")
            .bind(&mid)
            .fetch_one(&pool)
            .await
            .expect("mission updated_at before save");

    // Pin the clock in the past so a same-second `now()` cannot falsely pass equality.
    sqlx::query("UPDATE missions SET updated_at = now() - interval '1 hour' WHERE id = $1::uuid")
        .bind(&mid)
        .execute(&pool)
        .await
        .unwrap();
    let pinned: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT updated_at FROM missions WHERE id = $1::uuid")
            .bind(&mid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        pinned < before,
        "pin must land strictly before the create-time stamp: pinned={pinned} before={before}"
    );

    let notes = format!("version editor notes {stamp}");
    let ver = format!(
        r#"{{"semver":"0.2.0","editor_notes":"{notes}","payload":{{"editor":{{"slots":[]}}}}}}"#
    );
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{mid}/versions"),
        Some(&maker),
        None,
        Some(&ver),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "version save: {}",
        String::from_utf8_lossy(&b)
    );
    let body = json(&b);
    assert_eq!(body["semver"], "0.2.0");
    assert_eq!(
        body["editor_notes"],
        notes.as_str(),
        "editor_notes must round-trip on the returned MissionVersion: {body}"
    );

    let after: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT updated_at FROM missions WHERE id = $1::uuid")
            .bind(&mid)
            .fetch_one(&pool)
            .await
            .expect("mission updated_at after save");
    assert!(
        after > pinned,
        "create_version must bump missions.updated_at (library + approvals clocks): \
         after={after} pinned={pinned}"
    );

    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/admin/audit-logs?limit=100&q={title}"),
        Some(&admin),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    let rows = json(&b);
    let saves: Vec<&Value> = rows["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["action"] == "mission.version" && r["target_id"] == mid.as_str())
        .collect();
    assert_eq!(
        saves.len(),
        1,
        "exactly one mission.version audit row for this save: {rows}"
    );
    assert_eq!(saves[0]["target_type"], "mission");
    assert_eq!(saves[0]["severity"], "info");
    assert!(
        saves[0]["message"]
            .as_str()
            .is_some_and(|m| m.contains("0.2.0") && m.contains(&title)),
        "audit message must name the semver and title: {}",
        saves[0]
    );
}

/// PATCH contract: after `dense_fog`, `{"weather":""}` → 400 and the row stays.
///
/// A `valid_weather` that maps `""` → Clear makes this PATCH answer 200 and rewrite the row.
/// The unit tests cover the helper; this covers the wire + persistence.
///
/// Assert-flip check: expect `StatusCode::OK` on the blank PATCH — fails while production 400s.
#[tokio::test]
async fn patch_blank_weather_rejects_and_preserves_dense_fog() {
    let Some((app, pool, maker, _)) = app_pool_and_tokens().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let title = format!("Weather-Patch-Blank-{stamp}");

    let (st, b) = call(
        &app,
        "POST",
        "/api/v1/missions",
        Some(&maker),
        None,
        Some(&format!(
            r#"{{"title":"{title}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));
    let mid = json(&b)["id"].as_str().unwrap().to_string();

    let (st, b) = call(
        &app,
        "PATCH",
        &format!("/api/v1/missions/{mid}"),
        Some(&maker),
        None,
        Some(r#"{"weather":"dense_fog"}"#),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::OK,
        "set dense_fog: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(json(&b)["weather"], "dense_fog");

    let (st, b) = call(
        &app,
        "PATCH",
        &format!("/api/v1/missions/{mid}"),
        Some(&maker),
        None,
        Some(r#"{"weather":""}"#),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "blank weather PATCH must 400: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(
        json(&b)["error"],
        "invalid weather",
        "blank weather error body: {}",
        String::from_utf8_lossy(&b)
    );

    let stored: String =
        sqlx::query_scalar("SELECT weather::text FROM missions WHERE id = $1::uuid")
            .bind(&mid)
            .fetch_one(&pool)
            .await
            .expect("weather after rejected blank PATCH");
    assert_eq!(
        stored, "dense_fog",
        "rejected blank PATCH must leave dense_fog untouched"
    );

    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{mid}"),
        Some(&maker),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(
        json(&b)["weather"],
        "dense_fog",
        "GET must still serve dense_fog after blank PATCH reject: {}",
        String::from_utf8_lossy(&b)
    );
}
