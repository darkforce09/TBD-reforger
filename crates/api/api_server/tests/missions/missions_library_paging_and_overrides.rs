//! Library and bookmark lookups across paginated list responses: a lookup finds its mission past
//! page one, and the bookmarked scope lists only missions the caller can view.

use crate::missions_support;

use axum::http::StatusCode;

use missions_support::*;

/// Live ratchet: library + bookmark lookups must survive >20 rows sorting ahead of the
/// subject on `ORDER BY updated_at DESC`, without relying on shared-DB NULL `updated_at`
/// residue — impossible now that migration 0015 made those columns `NOT NULL`.
///
/// Plant 21 same-author drafts with a fresher `updated_at` than the subject. Default
/// page 1 (`limit=20`) must miss; `find_id_in_missions_list` must still find. Soft-delete
/// the fillers at the end so this run does not feed the never-pruned gate DB.
#[tokio::test]
async fn library_bookmark_lookup_survives_page1_overflow() {
    let Some((app, pool, maker, _)) = app_pool_and_tokens().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    const FILLERS: i32 = 21; // default list limit is 20
    const AUTHOR: &str = "000000000000000004"; // mission_maker dev-login discord_id

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let title = format!("Library-Subject-{stamp}");
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
    let subject = json(&b)["id"].as_str().unwrap().to_string();

    // Pin the subject behind the default page: fillers below get `now()`.
    sqlx::query("UPDATE missions SET updated_at = now() - interval '1 hour' WHERE id = $1::uuid")
        .bind(&subject)
        .execute(&pool)
        .await
        .expect("pin subject updated_at into the past");

    let filler_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
        "WITH planted AS ( \
             INSERT INTO missions (title, author_id, terrain, custom_terrain_name, game_mode, \
                 weather, time_of_day, max_players, status, thumbnail_url, briefing, \
                 rejection_reason, created_at, updated_at) \
             SELECT 'Library-Filler-' || g::text || '-' || $2::text, $1, 'everon', '', 'pve_coop', \
                 'clear', '14:00'::time, 10, 'draft'::mission_status, '', '', '', \
                 now(), now() \
             FROM generate_series(1, $3) AS g \
             RETURNING id \
         ) SELECT id FROM planted",
    )
    .bind(AUTHOR)
    .bind(stamp.to_string())
    .bind(FILLERS)
    .fetch_all(&pool)
    .await
    .expect("plant >20 newer-updated_at fillers");
    assert_eq!(
        filler_ids.len(),
        FILLERS as usize,
        "expected exactly {FILLERS} fillers"
    );

    assert!(
        !id_on_default_missions_page1(&app, &maker, "/api/v1/missions", &subject).await,
        "subject must be off default page 1 after {FILLERS} newer fillers — \
         otherwise the paginating helper is unproven"
    );
    assert!(
        find_id_in_missions_list(&app, &maker, "/api/v1/missions", &subject).await,
        "find_id_in_missions_list must still find the subject past page 1"
    );

    // Bookmark scope: same DESC/LIMIT ratchet, scoped to the caller's bookmarks only.
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{subject}/bookmark"),
        Some(&maker),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    sqlx::query(
        "INSERT INTO mission_bookmarks (discord_id, mission_id, created_at) \
         SELECT $1, id, now() FROM unnest($2::uuid[]) AS id \
         ON CONFLICT DO NOTHING",
    )
    .bind(AUTHOR)
    .bind(&filler_ids)
    .execute(&pool)
    .await
    .expect("bookmark fillers");
    // Re-pin subject behind the bookmarked fillers (bookmark POST does not bump updated_at,
    // but be explicit so a future writer cannot accidentally refresh it).
    sqlx::query("UPDATE missions SET updated_at = now() - interval '1 hour' WHERE id = $1::uuid")
        .bind(&subject)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE missions SET updated_at = now() WHERE id = ANY($1::uuid[])")
        .bind(&filler_ids)
        .execute(&pool)
        .await
        .unwrap();

    assert!(
        !id_on_default_missions_page1(&app, &maker, "/api/v1/missions?scope=bookmarked", &subject)
            .await,
        "bookmarked subject must be off default page 1 after {FILLERS} newer \
         bookmarked fillers"
    );
    assert!(
        find_id_in_missions_list(&app, &maker, "/api/v1/missions?scope=bookmarked", &subject).await,
        "find_id_in_missions_list must find the bookmarked subject past page 1"
    );

    // Retire this run's fillers + their bookmarks. Leave the subject (same as other ITs).
    sqlx::query("DELETE FROM mission_bookmarks WHERE mission_id = ANY($1::uuid[])")
        .bind(&filler_ids)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("UPDATE missions SET deleted_at = now() WHERE id = ANY($1::uuid[])")
        .bind(&filler_ids)
        .execute(&pool)
        .await
        .expect("soft-delete the planted fillers");
}

/// The `bookmarked` scope lists only the bookmarked missions the caller can see now. A peer's
/// bookmark row on someone else's draft (a row written before bookmark writes checked
/// visibility, or a mission that left the live state) stays out of the peer's list until the
/// mission is live, while the author's own bookmark on the same draft lists; a peer cannot write
/// such a bookmark through the API at all.
#[tokio::test]
async fn library_bookmarked_scope_lists_only_missions_the_caller_can_view() {
    let (app, pool, maker, _) = app_pool_and_tokens()
        .await
        .expect("the library suite requires its PostgreSQL database");
    let (_, peer) = app_and_token("enlisted")
        .await
        .expect("the library suite requires its PostgreSQL database");
    const PEER: &str = "000000000000000002"; // enlisted dev-login discord_id
    const BOOKMARKED: &str = "/api/v1/missions?scope=bookmarked";

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let (st, b) = call(
        &app,
        "POST",
        "/api/v1/missions",
        Some(&maker),
        None,
        Some(&format!(
            r#"{{"title":"Bookmark-Visibility-{stamp}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));
    let draft = json(&b)["id"].as_str().unwrap().to_string();
    let bookmark = format!("/api/v1/missions/{draft}/bookmark");

    let (st, b) = call(&app, "POST", &bookmark, Some(&maker), None, None).await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    let (st, b) = call(&app, "POST", &bookmark, Some(&peer), None, None).await;
    assert_eq!(
        st,
        StatusCode::NOT_FOUND,
        "a peer cannot bookmark a hidden draft: {}",
        String::from_utf8_lossy(&b)
    );

    sqlx::query("INSERT INTO mission_bookmarks (discord_id, mission_id, created_at) VALUES ($1, $2::uuid, now())")
        .bind(PEER)
        .bind(&draft)
        .execute(&pool)
        .await
        .expect("plant the peer's bookmark row on the draft");
    assert!(
        !find_id_in_missions_list(&app, &peer, BOOKMARKED, &draft).await,
        "a draft the peer cannot see must not list in the peer's bookmarked scope"
    );
    assert!(
        find_id_in_missions_list(&app, &maker, BOOKMARKED, &draft).await,
        "the author's own bookmarked draft must list in the author's bookmarked scope"
    );

    sqlx::query("UPDATE missions SET status = 'live' WHERE id = $1::uuid")
        .bind(&draft)
        .execute(&pool)
        .await
        .expect("make the draft live");
    assert!(
        find_id_in_missions_list(&app, &peer, BOOKMARKED, &draft).await,
        "once live, the mission lists in the peer's bookmarked scope"
    );

    sqlx::query("DELETE FROM mission_bookmarks WHERE mission_id = $1::uuid")
        .bind(&draft)
        .execute(&pool)
        .await
        .expect("remove this run's bookmarks");
    sqlx::query("UPDATE missions SET deleted_at = now() WHERE id = $1::uuid")
        .bind(&draft)
        .execute(&pool)
        .await
        .expect("soft-delete this run's mission");
}
