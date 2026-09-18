//! Library and bookmark lookups across paginated list responses, and the admin aggregate over
//! which mission defaults authors override.
//!
//! Skips without `TEST_DATABASE_URL`.

use axum::http::StatusCode;

mod common;
mod missions_support;

use missions_support::*;

/// Source ratchet: `find_id_in_missions_list` must keep walking `offset` pages, and the
/// lifecycle test must call it. Needles are `concat!`-split so this assert's own prose
/// cannot satisfy a positive `contains` if the helper body is deleted.
///
/// The helper lives in `tests/missions_support/mod.rs` and the lifecycle test in
/// `tests/missions_compiled_document.rs`, so this reads both sources rather than its own.
#[test]
fn find_id_in_missions_list_always_paginates() {
    let support = include_str!("missions_support/mod.rs");
    let helper = support
        .split("async fn find_id_in_missions_list")
        .nth(1)
        .expect("find_id_in_missions_list must exist")
        .split("async fn find_in_approvals")
        .next()
        .expect("helper must precede find_in_approvals");
    assert!(
        helper.contains("loop {"),
        "find_id_in_missions_list must loop across pages"
    );
    let advance = concat!("offset += ", "rows.len()");
    assert!(
        helper.contains(advance),
        "find_id_in_missions_list must advance offset across full pages"
    );
    let page_uri = concat!("limit={PAGE}", "&offset={offset}");
    assert!(
        helper.contains(page_uri),
        "find_id_in_missions_list must request limit={{PAGE}}&offset={{offset}}"
    );

    let lifecycle = include_str!("missions_compiled_document.rs")
        .split("async fn mission_lifecycle_and_compiled")
        .nth(1)
        .expect("mission_lifecycle_and_compiled must exist")
        .split("#[tokio::test]")
        .next()
        .expect("lifecycle body bounded by next tokio::test");
    assert!(
        lifecycle.contains("find_id_in_missions_list"),
        "lifecycle library/bookmark pins must go through find_id_in_missions_list"
    );
    // The page-1 shape this forbids: list["data"].as_array()….any(|c| c["id"] == …)
    let page1_shape = [r#"list["data"]"#, ".as_array()"].concat();
    assert!(
        !lifecycle.contains(&page1_shape),
        "lifecycle must not reintroduce page-1 list[\"data\"] membership"
    );
}

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
    let title = format!("T496-Subject-{stamp}");
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
             SELECT 'T496-Filler-' || g::text || '-' || $2::text, $1, 'everon', '', 'pve_coop', \
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

/// The whole point of the aggregate: which mission defaults every author changes, answered as a
/// QUERY over the corpus rather than a machine-parse of shipped PBOs.
///
/// Seeds two missions on ONE authored default-bearing key (`graceSeconds`, schema default 30): one
/// that authors the default value (in the population, NOT overriding) and one that authors a
/// non-default value (overriding). Asserts the fraction and the value histogram off those two, and
/// that a non-admin gets 403.
#[tokio::test]
async fn mission_default_overrides_reports_fraction_and_histogram() {
    let Some((app, pool, maker, admin)) = app_pool_and_tokens().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // A run-unique override value so the histogram assertion cannot collide with any other seeded
    // mission in this binary's database. 30 is the schema default for graceSeconds; this is not.
    let override_grace = 700_000
        + (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros()
            % 100_000) as i64;

    // Mission DEFAULT: authors graceSeconds = 30 (the schema default). It has a zone, so it is in
    // the denominator, but it must NOT be counted as overriding.
    let m_default = seed_zone_rules_mission(&pool, r#"{"graceSeconds":30}"#).await;
    // Mission OVERRIDE: authors graceSeconds = <unique>, which differs from the default.
    let m_override =
        seed_zone_rules_mission(&pool, &format!(r#"{{"graceSeconds":{override_grace}}}"#)).await;

    // ── Admin tier: 200 with the aggregate. ──────────────────────────────────────────────────────
    let (st, b) = call(
        &app,
        "GET",
        "/api/v1/admin/mission-default-overrides",
        Some(&admin),
        None,
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::OK,
        "admin read: {}",
        String::from_utf8_lossy(&b)
    );
    let body = json(&b);
    assert!(
        body["generated_at"].is_string(),
        "generated_at present: {body}"
    );

    let grace = find_override(&body, "zones[].rules.graceSeconds")
        .unwrap_or_else(|| panic!("graceSeconds row missing from {}", body["data"]));

    // The default is READ FROM THE SCHEMA, not hardcoded in the handler — assert the wire carries it.
    assert_eq!(
        grace["default_value"], 30,
        "graceSeconds schema default is 30"
    );

    // Both seeded missions author a zone, so the population is at least 2 (other tests in this
    // binary may add more — assert a floor, not equality).
    let total = grace["missions_total"].as_i64().unwrap();
    let overriding = grace["missions_overriding"].as_i64().unwrap();
    assert!(total >= 2, "missions_total >= 2 (the two seeded): {total}");
    assert!(
        overriding >= 1,
        "missions_overriding >= 1 (the non-default seed): {overriding}"
    );
    assert!(
        overriding <= total,
        "overriding {overriding} cannot exceed total {total}"
    );

    // override_fraction is exactly overriding/total (float compare with a tolerance).
    let frac = grace["override_fraction"].as_f64().unwrap();
    let expected = overriding as f64 / total as f64;
    assert!(
        (frac - expected).abs() < 1e-9,
        "override_fraction {frac} == overriding/total {expected}"
    );

    // Histogram: the unique override value appears exactly once (one mission authored it), and the
    // default value 30 appears at least once (the default-authoring seed).
    let hist = grace["histogram"].as_array().unwrap();
    let over_bucket = hist
        .iter()
        .find(|h| h["value"].as_i64() == Some(override_grace))
        .unwrap_or_else(|| {
            panic!("override value {override_grace} missing from histogram {hist:?}")
        });
    assert_eq!(
        over_bucket["count"].as_i64(),
        Some(1),
        "exactly one mission authored the unique override value"
    );
    let def_bucket = hist
        .iter()
        .find(|h| h["value"].as_i64() == Some(30))
        .unwrap_or_else(|| panic!("default value 30 missing from histogram {hist:?}"));
    assert!(
        def_bucket["count"].as_i64().unwrap() >= 1,
        "the default-authoring mission is a histogram bar too"
    );

    // ── Non-admin tier: 403. ─────────────────────────────────────────────────────────────────────
    let (st, _b) = call(
        &app,
        "GET",
        "/api/v1/admin/mission-default-overrides",
        Some(&maker),
        None,
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::FORBIDDEN,
        "mission_maker is below the admin tier"
    );

    // Cleanup so the shared gate DB does not accumulate these seed missions forever.
    for id in [&m_default, &m_override] {
        let _ = sqlx::query("UPDATE missions SET current_version_id = NULL WHERE id = $1::uuid")
            .bind(id)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM mission_versions WHERE mission_id = $1::uuid")
            .bind(id)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM missions WHERE id = $1::uuid")
            .bind(id)
            .execute(&pool)
            .await;
    }
}
