//! HTTP lifecycle operations distinguish retained participation from active event attachments.

use crate::{common, telemetry_support};

use api_configuration::configuration::Config;
use api_server::router::router;
use api_state::AppState;
use axum::{Router, http::StatusCode};
use serde_json::{Value, json};
use tokio::sync::Mutex;
use uuid::Uuid;

use telemetry_support::match_reports::ReportingServer;

static CATALOG_RACES: Mutex<()> = Mutex::const_new(());

struct Fixture {
    state: AppState,
    app: Router,
    token: String,
    actor: String,
    title: String,
    event: Uuid,
    mission: Uuid,
    starts_at: String,
    /// The game server that reports the fixture's played matches.
    reporter: ReportingServer,
}

impl Fixture {
    async fn new() -> Self {
        let url =
            common::require_test_database_url().expect("mission lifecycle requires PostgreSQL");
        let pool = api_database::connect(&url)
            .await
            .expect("the test database accepts a connection");
        api_database::migrate(&pool)
            .await
            .expect("the migrations apply to the test database");
        let state = api_server::composition::application_state(
            pool,
            Config::for_tests(url, "mission-detachment"),
        );
        let actor = format!("mission-detachment-{}", Uuid::new_v4());
        let token = common::access_token(
            &state,
            "mission_detachment_lifecycle",
            &actor,
            "admin",
            true,
        )
        .await;
        let app = router(state.clone());
        let title = format!("Retained mission {}", Uuid::new_v4());
        let created = telemetry_support::call(
            &app,
            "POST",
            "/api/v1/missions",
            Some(&token),
            None,
            Some(
                &json!({"title":title,"terrain":"everon","game_mode":"pve_coop","max_players":2})
                    .to_string(),
            ),
        )
        .await;
        assert_eq!(created.0, StatusCode::CREATED, "{created:?}");
        let mission = created.1["id"]
            .as_str()
            .expect("the `id` field is a string")
            .parse()
            .expect("the `id` field parses as an id");
        let starts_at = (chrono::Utc::now() + chrono::Duration::days(3)).to_rfc3339();
        let created = telemetry_support::call(&app, "POST", "/api/v1/events", Some(&token), None,
            Some(&json!({"name_override":"Detachment fixture","start_time":starts_at,"status":"open"}).to_string())).await;
        assert_eq!(created.0, StatusCode::CREATED, "{created:?}");
        let event = created.1["id"]
            .as_str()
            .expect("the `id` field is a string")
            .parse()
            .expect("the `id` field parses as an id");
        let reporter = ReportingServer::open(&app, &state.pool, "Detachment server").await;
        Self {
            state,
            app,
            token,
            actor,
            title,
            event,
            mission,
            starts_at,
            reporter,
        }
    }

    async fn call(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        telemetry_support::call(
            &self.app,
            method,
            uri,
            Some(&self.token),
            None,
            body.as_ref().map(Value::to_string).as_deref(),
        )
        .await
    }

    async fn attach(&self) -> (StatusCode, Value) {
        self.call("POST", &format!("/api/v1/events/{}/missions", self.event), Some(json!({
            "mission_id":self.mission,"start_time":self.starts_at,
            "orbat":[{"faction":"USA","squad":"Alpha","callsign":"A","slots":[{"role":"Rifleman"}]}]
        }))).await
    }

    async fn attach_with_attendance(&self) -> (Uuid, Uuid) {
        let attached = self.attach().await;
        assert_eq!(attached.0, StatusCode::CREATED, "{attached:?}");
        let attachment: Uuid = attached.1["id"]
            .as_str()
            .expect("the `id` field is a string")
            .parse()
            .expect("the `id` field parses as an id");
        let slot: Uuid =
            sqlx::query_scalar("SELECT id FROM orbat_slots WHERE event_mission_id = $1")
                .bind(attachment)
                .fetch_one(&self.state.pool)
                .await
                .expect("the read of orbat_slots returns a row");
        let registered = self
            .call(
                "POST",
                &format!("/api/v1/event-missions/{attachment}/register"),
                Some(json!({"slot_id":slot})),
            )
            .await;
        assert_eq!(registered.0, StatusCode::OK, "{registered:?}");
        let registration: Uuid = sqlx::query_scalar(
            "SELECT id FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
        )
        .bind(attachment)
        .bind(&self.actor)
        .fetch_one(&self.state.pool)
        .await
        .expect("the read of event_registrations returns a row");
        let arma: String = sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
            .bind(&self.actor)
            .fetch_one(&self.state.pool)
            .await
            .expect("the read of users returns a row");
        let ingested = self
            .reporter
            .report_results(
                &self.app,
                &json!({"match":{"source_match_id":Uuid::new_v4().to_string(),"event_id":self.event,"mission_id":self.mission,"outcome":"success"},
                    "players":[{"arma_id":arma,"source_event_id":"participation","role_played":"Rifleman",
                        "counters":{"kills":4,"deaths":1,"team_kills":0,"longest_kill_m":0,"vehicles_destroyed":0,"is_command":false}}]}),
            )
            .await;
        assert_eq!(ingested.0, StatusCode::OK, "{ingested:?}");
        (attachment, registration)
    }

    async fn detach(&self, attachment: Uuid) {
        let removed = self
            .call(
                "DELETE",
                &format!("/api/v1/events/{}/missions/{attachment}", self.event),
                None,
            )
            .await;
        assert_eq!(removed.0, StatusCode::NO_CONTENT, "{removed:?}");
    }

    async fn lifecycle(&self, delete: bool) -> (StatusCode, Value) {
        self.call(
            if delete { "DELETE" } else { "PATCH" },
            &format!("/api/v1/missions/{}", self.mission),
            (!delete).then(|| json!({"status":"archived"})),
        )
        .await
    }

    async fn mission_row(&self) -> Value {
        sqlx::query_scalar("SELECT to_jsonb(m) FROM missions m WHERE id = $1")
            .bind(self.mission)
            .fetch_one(&self.state.pool)
            .await
            .expect("the read of missions returns a row")
    }

    async fn evidence_counts(&self, action: Option<&str>) -> (i64, i64) {
        sqlx::query_as(
            "SELECT count(*), count(p.audit_id) FROM audit_logs a
             LEFT JOIN audit_publication_pending p ON p.audit_id = a.id
             WHERE a.target_type = 'mission' AND a.target_id = $1
               AND ($2::text IS NULL OR (a.action = $2 AND a.actor_id = $3))",
        )
        .bind(self.mission.to_string())
        .bind(action)
        .bind(&self.actor)
        .fetch_one(&self.state.pool)
        .await
        .expect("the read of audit_logs returns a row")
    }

    async fn history(&self, attachment: Uuid) -> Value {
        sqlx::query_scalar(
            "SELECT jsonb_build_object(
             'attachment', (SELECT to_jsonb(em) FROM event_missions em WHERE em.id = $1),
             'registrations', (SELECT jsonb_agg(to_jsonb(r) ORDER BY r.id) FROM event_registrations r WHERE r.event_mission_id = $1),
             'reservation_history', (SELECT jsonb_agg(to_jsonb(h) ORDER BY h.id) FROM event_registration_history h
                 JOIN event_registrations r ON r.id = h.registration_id WHERE r.event_mission_id = $1),
             'participation', (SELECT jsonb_agg(to_jsonb(p) ORDER BY p.registration_id, p.match_id, p.arma_id)
                 FROM event_registration_participation p JOIN event_registrations r ON r.id = p.registration_id WHERE r.event_mission_id = $1),
             'slots', (SELECT jsonb_agg(to_jsonb(s) ORDER BY s.id) FROM orbat_slots s WHERE s.event_mission_id = $1),
             'matches', (SELECT jsonb_agg(to_jsonb(m) ORDER BY m.id) FROM matches m WHERE m.event_id = $2 AND m.mission_id = $3),
             'results', (SELECT jsonb_agg(to_jsonb(s) ORDER BY s.id) FROM match_player_stats s JOIN matches m ON m.id = s.match_id WHERE m.event_id = $2 AND m.mission_id = $3),
             'versions', (SELECT jsonb_agg(to_jsonb(v) ORDER BY v.id) FROM mission_versions v WHERE v.mission_id = $3),
             'authored_audit', (SELECT jsonb_agg(to_jsonb(a) ORDER BY a.id) FROM audit_logs a
                 WHERE (a.target_id = $1::text AND a.action = 'event.mission_removed')
                    OR (a.target_id = $2::text AND a.action = 'event.mission_attached')))"
        ).bind(attachment).bind(self.event).bind(self.mission).fetch_one(&self.state.pool).await.expect("the read of event_missions returns a row")
    }

    async fn service_history(&self) -> Value {
        let response = self.call("GET", "/api/v1/me/deployments", None).await;
        assert_eq!(response.0, StatusCode::OK, "{response:?}");
        assert_eq!(response.1["upcoming"], json!([]));
        let history = response.1["service_history"].clone();
        assert_eq!(
            history
                .as_array()
                .expect("the service history is an array")
                .len(),
            1
        );
        assert_eq!(history[0]["operation"], self.title);
        history
    }
}

async fn detach_then_lifecycle_preserves_history(delete: bool) {
    let f = Fixture::new().await;
    let (attachment, registration) = f.attach_with_attendance().await;
    let registered_at: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT registered_at FROM event_registrations WHERE id = $1")
            .bind(registration)
            .fetch_one(&f.state.pool)
            .await
            .expect("the read of event_registrations returns a row");
    f.detach(attachment).await;
    let before = f.history(attachment).await;
    let service_before = f.service_history().await;
    let authored_before = f.mission_row().await;
    let response = f.lifecycle(delete).await;
    assert_eq!(
        response.0,
        if delete {
            StatusCode::NO_CONTENT
        } else {
            StatusCode::OK
        },
        "{response:?}"
    );
    let after = f.mission_row().await;
    assert_eq!(
        f.evidence_counts(Some(lifecycle_action(delete))).await,
        (1, 1)
    );
    if delete {
        assert!(!after["deleted_at"].is_null());
        assert_eq!(
            f.call("GET", &format!("/api/v1/missions/{}", f.mission), None)
                .await
                .0,
            StatusCode::NOT_FOUND
        );
    } else {
        assert_eq!(response.1["status"], "archived");
        assert_eq!(after["status"], "archived");
        assert!(after["deleted_at"].is_null());
    }
    for field in [
        "id",
        "author_id",
        "title",
        "current_version_id",
        "created_at",
    ] {
        assert_eq!(
            after[field], authored_before[field],
            "mission authorship field {field} remains factual"
        );
    }
    assert_eq!(
        f.history(attachment).await,
        before,
        "lifecycle writes preserve every retained reference and participation fact"
    );
    assert_eq!(
        f.service_history().await,
        service_before,
        "historical HTTP records keep their mission name"
    );
    let retained: (
        String,
        String,
        Option<Uuid>,
        chrono::DateTime<chrono::Utc>,
        i64,
    ) = sqlx::query_as(
        "SELECT reservation_state::text, attendance_state::text, slot_id, registered_at,
         (SELECT count(*) FROM event_registration_participation WHERE registration_id = $1)
         FROM event_registrations WHERE id = $1",
    )
    .bind(registration)
    .fetch_one(&f.state.pool)
    .await
    .expect("the read of event_registration_participation returns a row");
    assert_eq!(
        retained,
        (
            "withdrawn".into(),
            "attended".into(),
            None,
            registered_at,
            1
        )
    );
    let hidden = f
        .call(
            "GET",
            &format!("/api/v1/event-missions/{attachment}/orbat"),
            None,
        )
        .await;
    assert_eq!(hidden.0, StatusCode::NOT_FOUND);
    let event = f
        .call("GET", &format!("/api/v1/events/{}", f.event), None)
        .await;
    assert_eq!(event.0, StatusCode::OK);
    assert_eq!(event.1["missions"], json!([]));
    f.state.pool.close().await;
}

#[tokio::test]
async fn detached_mission_archives_without_rewriting_attendance_or_authorship() {
    let _guard = CATALOG_RACES.lock().await;
    detach_then_lifecycle_preserves_history(false).await;
}

#[tokio::test]
async fn active_upcoming_attachment_still_blocks_archive_and_delete_atomically() {
    let _guard = CATALOG_RACES.lock().await;
    let f = Fixture::new().await;
    let (attachment, _) = f.attach_with_attendance().await;
    let before = f.mission_row().await;
    let history = f.history(attachment).await;
    for delete in [false, true] {
        let response = f.lifecycle(delete).await;
        assert_eq!(response.0, StatusCode::CONFLICT, "{response:?}");
        assert_eq!(f.mission_row().await, before);
        assert_eq!(f.history(attachment).await, history);
    }
    f.state.pool.close().await;
}

fn lifecycle_action(delete: bool) -> &'static str {
    if delete {
        "mission.delete_authorized"
    } else {
        "mission.archive"
    }
}
