//! The read paths that serve a roster to the game server and a member directory to the SPA:
//! the machine-credential roster export (which must degrade by omitting a mission whose tip no
//! longer compiles, not by 500ing), and `GET /members` offset pagination. The suite-private
//! fixture guards live here too, since they pin the ids and the `arma_id` mint every events
//! suite seeds through. Skips without `TEST_DATABASE_URL`.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use events_support::{DB_LOCK, DEV_USER, OTHER, THIRD, arma, boot, call, token};
use serde_json::Value;
use tower::ServiceExt;

mod common;
mod events_support;

/// Two seeds must not share a fixed `arma_id` string (a flake on a shared database).
///
/// Perturbation: change [`arma`] back to `format!("events-arma-{discord_id}")` → both
/// equal the literal below → assert fails.
#[test]
fn arma_mint_never_collides_on_a_fixed_string() {
    let a = arma(OTHER);
    let b = arma(OTHER);
    assert_ne!(
        a, b,
        "arma() must mint distinct ids — fixed events-arma-{{discord}} collides under parallel IT"
    );
    let fixed = format!("events-arma-{OTHER}");
    assert_ne!(a, fixed, "mint must not be the fixed string: {a}");
    assert_ne!(b, fixed, "mint must not be the fixed string: {b}");
    assert!(
        a.starts_with(&format!("events-arma-{OTHER}-")),
        "traceable prefix required: {a}"
    );
}

/// Suite snowflakes must stay off telemetry / identity / dev-login ranges.
///
/// Perturbation: set OTHER/THIRD equal to a known foreign actor → assert fails.
#[test]
fn actor_snowflakes_are_suite_private() {
    // telemetry_server_status_ingest.rs PLAYER_DISCORD (collision class)
    const TELEMETRY_PLAYER: &str = "000000000000400003";
    // identity_link.rs ACTOR / PAD
    const IDENTITY_ACTOR: &str = "000000000000400001";
    const IDENTITY_PAD: &str = "000000000000400013";
    assert_ne!(OTHER, THIRD);
    assert_ne!(OTHER, DEV_USER);
    assert_ne!(THIRD, DEV_USER);
    assert_ne!(OTHER, TELEMETRY_PLAYER);
    assert_ne!(THIRD, TELEMETRY_PLAYER);
    assert_ne!(OTHER, IDENTITY_ACTOR);
    assert_ne!(THIRD, IDENTITY_ACTOR);
    assert_ne!(OTHER, IDENTITY_PAD);
    assert_ne!(THIRD, IDENTITY_PAD);
}

/// `GET /api/v1/members` must honour `offset` (not only return an array).
///
/// The handler does LIMIT/OFFSET behind `{data,total,limit,offset}` and a pure-oracle unit
/// test covers the arithmetic; an IT that only did `assert!(mem["data"].is_array())` stays
/// green if OFFSET is deleted. So: seed 25 suite-private members whose usernames sort under a
/// unique prefix, request `offset=20` (default limit 20), and assert the window starts at
/// member index 20 plus the envelope fields.
///
/// Perturbation: drop `OFFSET` / hard-code `LIMIT 20` with no bind → page still length 20 but
/// first username is `roster_page_user_00` (or total/offset mismatch) → assert fails.
#[tokio::test]
async fn members_list_honours_offset_pagination() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    const N: usize = 25;
    const PREFIX: &str = "roster_page_user_";
    // Suite-private discord ids — must not collide with OTHER/THIRD / other suites.
    for i in 0..N {
        let discord_id = format!("000000000000495{i:03}");
        let username = format!("{PREFIX}{i:02}");
        events_support::seed_member(
            &pool,
            &discord_id,
            &username,
            &arma(&discord_id),
            "enlisted",
        )
        .await;
    }

    let leader = token(&app, "leader").await;

    // Page 0 — member at index 20 must be invisible.
    let (st, page0) = call(
        &app,
        "GET",
        &format!("/api/v1/members?q={PREFIX}&limit=20&offset=0"),
        &leader,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "members page0: {page0}");
    let data0 = page0["data"]
        .as_array()
        .unwrap_or_else(|| panic!("page0 missing data array: {page0}"));
    assert_eq!(data0.len(), 20, "default first page size: {page0}");
    assert_eq!(
        page0["total"], N as i64,
        "filtered total must be seeded N: {page0}"
    );
    assert_eq!(page0["limit"], 20, "envelope limit: {page0}");
    assert_eq!(page0["offset"], 0, "envelope offset: {page0}");
    let names0: Vec<&str> = data0
        .iter()
        .map(|m| m["username"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(names0.first().copied(), Some("roster_page_user_00"));
    assert!(
        !names0.contains(&"roster_page_user_20"),
        "offset=0 must not include member index 20: {names0:?}"
    );

    // Page at offset=20 — member 21 (0-based index 20) is the first row.
    let (st, page_off) = call(
        &app,
        "GET",
        &format!("/api/v1/members?q={PREFIX}&limit=20&offset=20"),
        &leader,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "members offset=20: {page_off}");
    let data_off = page_off["data"]
        .as_array()
        .unwrap_or_else(|| panic!("offset page missing data array: {page_off}"));
    assert_eq!(
        data_off.len(),
        5,
        "25 seeded − offset 20 → 5 rows: {page_off}"
    );
    assert_eq!(
        page_off["total"], N as i64,
        "total must stay N across pages: {page_off}"
    );
    assert_eq!(page_off["limit"], 20, "envelope limit: {page_off}");
    assert_eq!(page_off["offset"], 20, "envelope offset: {page_off}");
    assert_eq!(
        data_off[0]["username"].as_str(),
        Some("roster_page_user_20"),
        "offset=20 must surface member at index 20 first: {page_off}"
    );
    assert_eq!(
        data_off[4]["username"].as_str(),
        Some("roster_page_user_24"),
        "last row of the window: {page_off}"
    );
}

/// Machine-credential game-runtime call (roster). Member routes stay on [`call`].
async fn call_machine(
    app: &Router,
    secret: &str,
    method: &str,
    uri: &str,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {secret}"));
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let req = b
        .body(body.map_or(Body::empty(), |s| Body::from(s.to_string())))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Editor graph with one BLUFOR SL seat — compiles under an empty or loaded cargo catalog.
const T551_GOOD_EDITOR: &str = r#"{
  "schemaVersion": 1,
  "editor": {
    "factions": [{"id":"f1","key":"BLUFOR","name":"US","squadIds":["sq1"]}],
    "squads": [{"id":"sq1","factionId":"f1","callsign":"A","name":"Alpha","slotIds":["s1"]}],
    "slots": [{
      "id":"s1","squadId":"sq1","index":0,"role":"SL",
      "position":{"x":100.0,"y":200.0,"z":0,"rotation":0}
    }],
    "editorLayers": []
  }
}"#;

/// The roster seats players from the artifact the server runs, compiled against the loaded cargo
/// catalog, and a later over-capacity tip changes nothing: the tip is not what the server runs,
/// and it can never become an artifact. Same numbers as the capacity refusal at Save: 4×60 cm³
/// into a 200 cm³ vest.
///
/// RED: compile the roster from the current version again, or compile artifacts against an
/// empty catalog — the over-capacity tip then reaches the roster or becomes an artifact.
#[tokio::test]
async fn roster_seats_the_deployed_artifact_and_an_over_capacity_tip_never_compiles() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let vest_rn = format!("roster_kit_vest_{stamp}");
    let mag_rn = format!("roster_kit_mag_{stamp}");
    let actor_arma = arma(OTHER);

    // Current modpack + private phys rows — load_cargo_phys_catalog includes all is_current.
    let pack_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO modpacks (name, version, total_size_bytes, workshop_url, is_current, created_at) \
         VALUES ($1, '0.0.1', 1, 'https://example.invalid/roster-kit', true, now()) RETURNING id",
    )
    .bind(format!("Roster Kit Pack {stamp}"))
    .fetch_one(&pool)
    .await
    .expect("seed modpack");
    sqlx::query(
        "INSERT INTO registry_items \
         (modpack_id, resource_name, display_name, category, kind, sort_order, \
          weight_kg, volume_cm3, created_at, updated_at) \
         VALUES ($1, $2, 'Mag', 'RosterKit', 'gear_vest', 0, 0.5, 60.0, now(), now())",
    )
    .bind(pack_id)
    .bind(&mag_rn)
    .execute(&pool)
    .await
    .expect("seed mag phys");
    sqlx::query(
        "INSERT INTO registry_items \
         (modpack_id, resource_name, display_name, category, kind, sort_order, \
          max_weight_kg, max_volume_cm3, created_at, updated_at) \
         VALUES ($1, $2, 'Plate Carrier', 'RosterKit', 'gear_vest', 1, 5.0, 200.0, now(), now())",
    )
    .bind(pack_id)
    .bind(&vest_rn)
    .execute(&pool)
    .await
    .expect("seed vest phys");

    let admin = token(&app, "admin").await;
    events_support::seed_member(&pool, OTHER, "roster-kit-other", &actor_arma, "enlisted").await;

    let create = format!(
        r#"{{"title":"Roster Kit Cargo {stamp}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
    );
    let (st, m) = call(&app, "POST", "/api/v1/missions", &admin, Some(&create)).await;
    assert_eq!(st, StatusCode::CREATED, "mission: {m}");
    let mid = m["id"].as_str().unwrap().to_string();

    // Publish a compiling tip so attach can derive ORBAT and the control roster seats.
    let ver = format!(r#"{{"semver":"0.2.0","payload":{T551_GOOD_EDITOR}}}"#);
    let (st, v) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{mid}/versions"),
        &admin,
        Some(&ver),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "good version: {v}");

    let (st, e) = call(
        &app,
        "POST",
        "/api/v1/events",
        &admin,
        Some(r#"{"start_time":"2027-12-01T00:00:00Z"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "event: {e}");
    let eid = e["id"].as_str().unwrap().to_string();

    let attach = format!(r#"{{"mission_id":"{mid}","start_time":"2027-12-01T00:00:00Z"}}"#);
    let (st, em) = call(
        &app,
        "POST",
        &format!("/api/v1/events/{eid}/missions"),
        &admin,
        Some(&attach),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "attach: {em}");
    let emid = em["id"].as_str().unwrap();

    let (st, orbat) = call(
        &app,
        "GET",
        &format!("/api/v1/event-missions/{emid}/orbat"),
        &admin,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "orbat: {orbat}");
    let slot_id = orbat["data"][0]["slots"][0]["id"]
        .as_str()
        .expect("slot id")
        .to_string();

    sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() WHERE id = $2::uuid")
        .bind(OTHER)
        .bind(&slot_id)
        .execute(&pool)
        .await
        .expect("assign seat");

    // The event runs on a registered server requiring the current modpack, and the approved
    // artifact is deployed there for this event mission.
    let runtime = common::event_runtime_credential(&pool, eid.parse().unwrap(), DEV_USER).await;
    let server: String =
        sqlx::query_scalar("SELECT server_id::text FROM events WHERE id = $1::uuid")
            .bind(&eid)
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("UPDATE servers SET required_modpack_id = $1 WHERE id = $2::uuid")
        .bind(pack_id)
        .bind(&server)
        .execute(&pool)
        .await
        .unwrap();
    let (st, submitted) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{mid}/submit"),
        &admin,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "submit: {submitted}");
    let (_, history) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{mid}/reviews"),
        &admin,
        None,
    )
    .await;
    let artifact = history["reviews"][0]["artifact_id"]
        .as_str()
        .unwrap()
        .to_string();
    let (st, approved) = call(
        &app,
        "POST",
        &format!("/api/v1/approvals/{mid}/approve"),
        &admin,
        Some(&format!(r#"{{"artifact_id":"{artifact}"}}"#)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "approve: {approved}");
    let (st, _) = call(
        &app,
        "PUT",
        "/api/v1/fleet/scenarios/everon",
        &admin,
        Some(r#"{"scenario_id":"{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf","display_name":"Everon"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, deployment) = call(
        &app,
        "POST",
        &format!("/api/v1/servers/{server}/deployments"),
        &admin,
        Some(&format!(
            r#"{{"mission_id":"{mid}","artifact_id":"{artifact}","event_mission_id":"{emid}"}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::ACCEPTED, "deployment: {deployment}");

    // Control — the deployed artifact seats the assigned arma_id (guards "always empty").
    let (st, body) = call_machine(
        &app,
        &runtime,
        "GET",
        &format!("/api/v1/game-runtime/events/{eid}/roster"),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "control roster: {body}");
    let assignments: Vec<&str> = body["assignments"]
        .as_array()
        .unwrap_or_else(|| panic!("assignments array: {body}"))
        .iter()
        .map(|assignment| assignment["armaId"].as_str().unwrap())
        .collect();
    assert!(
        assignments.contains(&actor_arma.as_str()),
        "the deployed artifact must seat the assigned arma_id; got {body}"
    );

    // Bypass Save (would 400) — plant the over-capacity residual tip.
    let bad_payload = format!(
        r#"{{"schemaVersion":1,"editor":{{
        "factions":[{{"id":"f1","key":"BLUFOR","name":"US","squadIds":["sq1"]}}],
        "squads":[{{"id":"sq1","factionId":"f1","callsign":"A","name":"Alpha","slotIds":["s1"]}}],
        "slots":[{{"id":"s1","squadId":"sq1","index":0,"role":"SL",
            "position":{{"x":100.0,"y":200.0,"z":0,"rotation":0}},
            "loadout":{{"version":2,"wear":{{"vest":"{vest_rn}"}},"weapons":[],
              "cargo":[{{"container":"vest","item":"{mag_rn}","qty":4}}]}}}}],
        "editorLayers":[]}}}}"#
    );
    let vid = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO mission_versions (id, mission_id, semver, json_payload, editor_notes, created_by, created_at) \
         VALUES ($1, $2::uuid, '9.9.9', $3::jsonb, '', '000000000000000001', now())",
    )
    .bind(vid)
    .bind(&mid)
    .bind(&bad_payload)
    .execute(&pool)
    .await
    .expect("insert over-capacity version");
    sqlx::query("UPDATE missions SET current_version_id = $1 WHERE id = $2::uuid")
        .bind(vid)
        .bind(&mid)
        .execute(&pool)
        .await
        .expect("point tip at over-capacity version");

    // The roster still reads the deployed artifact, never the tip.
    let (st, again) = call_machine(
        &app,
        &runtime,
        "GET",
        &format!("/api/v1/game-runtime/events/{eid}/roster"),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{again}");
    assert_eq!(
        again["assignments"], body["assignments"],
        "a tip change never reaches the roster"
    );
    // And an over-capacity version can never become an artifact: a draft whose tip is the
    // same payload is refused at submission.
    let (st, draft) = call(
        &app,
        "POST",
        "/api/v1/missions",
        &admin,
        Some(&format!(
            r#"{{"title":"Roster Kit Overloaded {stamp}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{draft}");
    let draft_id = draft["id"].as_str().unwrap().to_string();
    let overloaded = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO mission_versions (id, mission_id, semver, json_payload, editor_notes, created_by, created_at) \
         VALUES ($1, $2::uuid, '9.9.9', $3::jsonb, '', '000000000000000001', now())",
    )
    .bind(overloaded)
    .bind(&draft_id)
    .bind(&bad_payload)
    .execute(&pool)
    .await
    .expect("insert over-capacity draft version");
    sqlx::query("UPDATE missions SET current_version_id = $1 WHERE id = $2::uuid")
        .bind(overloaded)
        .bind(&draft_id)
        .execute(&pool)
        .await
        .expect("point the draft's tip at the over-capacity version");
    let (st, refused) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{draft_id}/submit"),
        &admin,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(
        refused["details"]["code"], "UNCOMPILABLE_VERSION",
        "{refused}"
    );

    // Cleanup so the shared gate DB does not accumulate forever-current packs / seats.
    let _ = sqlx::query(
        "UPDATE orbat_slots SET assigned_to = NULL, assigned_at = NULL WHERE id = $1::uuid",
    )
    .bind(&slot_id)
    .execute(&pool)
    .await;
    // The artifact names the modpack it compiled against, so the pack stays and stops being
    // current.
    sqlx::query("UPDATE modpacks SET is_current = false WHERE id = $1")
        .bind(pack_id)
        .execute(&pool)
        .await
        .unwrap();
}
