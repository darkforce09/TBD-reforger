//! Who may create a mission, and the single door into the approvals queue: submit, reject,
//! resubmit, approve, and the source states and authors each transition refuses.
//!
//! Skips without `TEST_DATABASE_URL`.

use axum::http::StatusCode;
use serde_json::Value;

mod common;
mod missions_support;

use missions_support::*;

#[tokio::test]
async fn enlisted_cannot_create_mission() {
    let Some((app, tok)) = app_and_token("enlisted").await else {
        return;
    };
    let create = r#"{"title":"X","terrain":"everon","game_mode":"pve_coop","max_players":16}"#;
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/missions",
        Some(&tok),
        None,
        Some(create),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}

/// The mission approval state machine, driven end to end.
///
/// The queue can only ever contain what `POST /missions/:id/submit` put there: `apply_status_patch`
/// refuses `pending_approval` (asserted below), so this route is the sole writer and its authors are
/// the sole source of `GET /approvals`. That makes three things worth pinning, each of which was
/// once wrong or absent:
///
/// 1. **A resubmission clears the whole previous review round.** Only `rejection_reason` was wiped,
///    so a rejected-then-resubmitted mission was served as `pending_approval` while still carrying
///    the earlier reviewer's `reviewed_by`/`reviewed_at` — "already reviewed" on a mission awaiting
///    review, and a stale ordering key for anything that ever lists reviewed missions.
/// 2. **The submission is audited.** `mission.approve` and `mission.reject` both write an audit row;
///    the transition that CREATES the reviewer's work did not, and the mission row has no
///    `submitted_by`/`submitted_at` of its own — so nothing recorded who queued it.
/// 3. **Only the author or an admin may submit, and only from `draft`/`rejected`.**
#[tokio::test]
async fn mission_submit_is_the_only_door_into_the_approvals_queue() {
    let Some((app, pool, maker, admin)) = app_pool_and_tokens().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // The title carries a run-unique, URL-safe stamp because the audit assertion below reads the
    // log back through `?q=` (an ILIKE on `message`, the only server-side filter there is) and then
    // counts the hits. A fixed title would also match every previous run's rows in a test database
    // the suite never truncates, and the count would drift upward run over run.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let title = format!("Approval-Submit-Path-{stamp}");
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
    let submit = format!("/api/v1/missions/{mid}/submit");

    // PATCH is not a second door: the only status values it accepts are `archived` and `draft`
    // (`apply_status_patch`). If this ever starts returning 200, the queue has a writer that skips
    // every guard below and this test's premise is void.
    let (st, b) = call(
        &app,
        "PATCH",
        &format!("/api/v1/missions/{mid}"),
        Some(&maker),
        None,
        Some(r#"{"status":"pending_approval"}"#),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "PATCH must not be able to enqueue: {}",
        String::from_utf8_lossy(&b)
    );

    // --- draft -> pending, and it reaches the admin queue ---
    let (st, b) = call(&app, "POST", &submit, Some(&maker), None, None).await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json(&b)["status"], "pending_approval");

    // The author cannot read the queue they just fed — it is admin-tier.
    let (st, _) = call(&app, "GET", "/api/v1/approvals", Some(&maker), None, None).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "queue must be admin-only");

    // Paginate the queue — page-1 LIMIT 20 reds once residue > 20 (measured total 26 on
    // a shared gate database).
    let row = find_in_approvals(&app, &admin, &mid)
        .await
        .unwrap_or_else(|| panic!("submitted mission missing from GET /approvals (paginated)"));
    assert_eq!(row["title"], title.as_str());
    assert!(
        row["submitted_at"]
            .as_str()
            .is_some_and(|s| s.ends_with('Z')),
        "the queue row must carry a submitted_at: {row}"
    );

    // A second submit is a 409, not a duplicate enqueue.
    let (st, _) = call(&app, "POST", &submit, Some(&maker), None, None).await;
    assert_eq!(st, StatusCode::CONFLICT, "double submit");

    // --- reject, then resubmit: the previous review round must be gone ---
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/approvals/{mid}/reject"),
        Some(&admin),
        None,
        Some(r#"{"reason":"objective markers overlap the spawn"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json(&b)["status"], "rejected");
    assert_eq!(
        json(&b)["rejection_reason"],
        "objective markers overlap the spawn"
    );
    assert!(
        json(&b)["reviewed_by"].is_string(),
        "the rejection must stamp a reviewer, or the next assertion proves nothing"
    );

    let (st, b) = call(&app, "POST", &submit, Some(&maker), None, None).await;
    assert_eq!(st, StatusCode::OK, "resubmit a rejected mission");
    let m = json(&b);
    assert_eq!(m["status"], "pending_approval");
    // `skip_serializing_if = "Option::is_none"` means "cleared" is "absent", exactly as on a
    // mission that has never been reviewed — not a literal null.
    assert!(
        m.get("reviewed_by").is_none() && m.get("reviewed_at").is_none(),
        "a resubmitted mission must carry no reviewer stamp: {m}"
    );
    assert!(
        m.get("rejection_reason").is_none(),
        "the old rejection reason must be gone: {m}"
    );

    // --- the submission is on the audit trail ---
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
    let submits: Vec<&Value> = rows["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["action"] == "mission.submit" && r["target_id"] == mid.as_str())
        .collect();
    assert_eq!(
        submits.len(),
        2,
        "both the first submit and the resubmit must be audited: {rows}"
    );
    assert_eq!(submits[0]["target_type"], "mission");
    assert_eq!(submits[0]["severity"], "info");

    // --- approve, and then no further submit is possible ---
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/approvals/{mid}/approve"),
        Some(&admin),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json(&b)["status"], "live");
    let (st, _) = call(&app, "POST", &submit, Some(&maker), None, None).await;
    assert_eq!(
        st,
        StatusCode::CONFLICT,
        "a live mission is not submittable"
    );

    // --- authorisation + the remaining refused source states ---
    // A second author, inserted directly: we need a mission_maker who is neither the
    // maker (`…004`) nor the admin (`…001`) under test.
    let other = "999000000000000009";
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_character, role, \
         is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'Approval Other Author', 'approvalother', '', '', 'mission_maker', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO NOTHING",
    )
    .bind(other)
    .execute(&pool)
    .await
    .unwrap();
    let seed = |author: &'static str, status: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, uuid::Uuid>(
                "INSERT INTO missions (title, author_id, terrain, custom_terrain_name, game_mode, \
                 weather, time_of_day, max_players, status, thumbnail_url, briefing, \
                 rejection_reason, created_at, updated_at) \
                 VALUES ($1, $2, 'everon', '', 'pve_coop', 'clear', '14:00'::time, 10, \
                 $3::mission_status, '', '', '', now(), now()) RETURNING id",
            )
            .bind(format!("Approval {status} by {author}"))
            .bind(author)
            .bind(status)
            .fetch_one(&pool)
            .await
            .unwrap()
            .to_string()
        }
    };

    let foreign = seed(other, "draft").await;
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{foreign}/submit"),
        Some(&maker),
        None,
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::FORBIDDEN,
        "a mission_maker must not submit someone else's mission: {}",
        String::from_utf8_lossy(&b)
    );
    // The admin override is the same one PATCH and DELETE already grant, and it is deliberate:
    // an admin acting for an author must be able to queue the mission. Pinned so that removing
    // `can_edit`'s admin arm here becomes a visible decision, not a silent one.
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{foreign}/submit"),
        Some(&admin),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json(&b)["status"], "pending_approval");

    // The maker is `…004`, not the admin `…001` — seed under the maker so submit reaches
    // the archived-status guard (not a foreign-author 403).
    let archived = seed("000000000000000004", "archived").await;
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{archived}/submit"),
        Some(&maker),
        None,
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CONFLICT,
        "an archived mission is not submittable"
    );
}
