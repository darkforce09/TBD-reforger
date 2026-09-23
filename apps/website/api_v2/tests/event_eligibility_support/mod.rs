//! Owned event fixtures for eligibility, quota, promotion and visibility suites: members,
//! guests, unverified and partner-guild accounts, and helpers that drive the real HTTP routes.
//!
//! Compiled into each suite that writes `mod event_eligibility_support;`; it adds no test binary.

#![allow(dead_code)]

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::core::{
    application_state::AppState, configuration::Config, database, http_router,
};
use website_api::identity_and_access::services::session_issuance::issue_session;

use crate::common;

pub const PARTNER_ROLE: &str = "partner-role-rifleman";

/// Serializes the tests of one suite that request or drain event reservation re-evaluations. The
/// queue is global to the database, so one test's drain would otherwise lease and re-evaluate
/// another test's event while that test observes it.
pub static REEVALUATION_QUEUE: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

pub struct Actor {
    pub id: String,
    pub token: String,
}

pub struct Fixture {
    pub state: AppState,
    pub app: Router,
    pub admin: Actor,
    pub leader: Actor,
    pub event: Uuid,
    pub missions: Vec<Uuid>,
    /// Seats per mission in allocation order.
    pub slots: Vec<Vec<Uuid>>,
    pub main_guild: String,
    pub partner_guild: String,
    suite: String,
}

pub struct EventShape<'a> {
    pub max_slots: i64,
    /// One entry per mission; each entry lists the squad of every seat, in order.
    pub missions: &'a [&'a [&'a str]],
}

impl Fixture {
    pub async fn new(suite: &str, shape: EventShape<'_>) -> Self {
        let url =
            common::require_test_database_url().expect("event eligibility requires PostgreSQL");
        let pool = database::connect(&url).await.unwrap();
        database::migrate(&pool).await.unwrap();
        let config = Config::for_tests(url, "event-eligibility");
        let main_guild = config.discord_guild_id.clone();
        let state = AppState::new(pool, config);
        let app = http_router::router(state.clone());
        let mut fixture = Self {
            admin: Actor {
                id: String::new(),
                token: String::new(),
            },
            leader: Actor {
                id: String::new(),
                token: String::new(),
            },
            state,
            app,
            event: Uuid::nil(),
            missions: Vec::new(),
            slots: Vec::new(),
            main_guild,
            partner_guild: format!("partner-{}", Uuid::new_v4()),
            suite: suite.to_owned(),
        };
        fixture.admin = fixture.account("admin", "admin").await;
        fixture.leader = fixture.account("leader", "leader").await;
        fixture.event = sqlx::query_scalar(
            "INSERT INTO events(name_override, start_time, status, max_slots, created_by, created_at)
             VALUES ('Eligibility operation', clock_timestamp() + interval '3 days', 'open', $1, $2,
                 clock_timestamp() - interval '1 hour') RETURNING id",
        )
        .bind(shape.max_slots)
        .bind(&fixture.admin.id)
        .fetch_one(&fixture.state.pool)
        .await
        .unwrap();
        for squads in shape.missions {
            let mission: Uuid = sqlx::query_scalar(
                "INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status)
                 VALUES ('Eligibility mission', $1, 'everon', 'pve_coop', 64, 'live') RETURNING id",
            )
            .bind(&fixture.admin.id)
            .fetch_one(&fixture.state.pool)
            .await
            .unwrap();
            let attachment: Uuid = sqlx::query_scalar(
                "INSERT INTO event_missions(event_id, mission_id, start_time)
                 VALUES ($1, $2, clock_timestamp() + interval '3 days') RETURNING id",
            )
            .bind(fixture.event)
            .bind(mission)
            .fetch_one(&fixture.state.pool)
            .await
            .unwrap();
            let mut seats = Vec::new();
            for (index, squad) in squads.iter().enumerate() {
                seats.push(
                    sqlx::query_scalar(
                        "INSERT INTO orbat_slots(event_mission_id, faction, squad, role, slot_index)
                         VALUES ($1, 'BLUFOR', $2, 'Rifleman', $3) RETURNING id",
                    )
                    .bind(attachment)
                    .bind(*squad)
                    .bind(index as i64)
                    .fetch_one(&fixture.state.pool)
                    .await
                    .unwrap(),
                );
            }
            fixture.missions.push(attachment);
            fixture.slots.push(seats);
        }
        fixture
    }

    fn fresh_id(&self, label: &str) -> String {
        format!("{}-{label}-{}", self.suite, Uuid::new_v4())
    }

    /// An account with a verified main-guild snapshot for `role` (guest is a confirmed nonmember).
    pub async fn account(&self, label: &str, role: &str) -> Actor {
        let id = self.fresh_id(label);
        let token = common::access_token(&self.state, &self.suite, &id, role, true).await;
        Actor { id, token }
    }

    pub async fn member(&self, label: &str) -> Actor {
        self.account(label, "enlisted").await
    }

    /// A verified member whose Arma identity is not linked yet.
    pub async fn unlinked_member(&self, label: &str) -> Actor {
        let id = self.fresh_id(label);
        let token = common::access_token(&self.state, &self.suite, &id, "enlisted", false).await;
        Actor { id, token }
    }

    pub async fn guest(&self, label: &str) -> Actor {
        self.account(label, "guest").await
    }

    /// An authenticated account no Discord observation has verified yet.
    pub async fn unverified(&self, label: &str) -> Actor {
        let id = self.fresh_id(label);
        sqlx::query(
            "INSERT INTO users (discord_id, username, role, arma_id, created_at, updated_at)
             VALUES ($1, $1, 'guest', $2, now(), now())",
        )
        .bind(&id)
        .bind(common::unique_arma("eligibility-unverified"))
        .execute(&self.state.pool)
        .await
        .unwrap();
        let (token, _, _) = issue_session(&self.state, &id).await.unwrap();
        Actor { id, token }
    }

    /// A confirmed TBD nonmember whose partner guild membership is verified with `roles`.
    pub async fn partner(&self, label: &str, roles: &[&str]) -> Actor {
        let actor = self.guest(label).await;
        self.observe(&actor, &self.partner_guild.clone(), "member", roles, 0)
            .await;
        actor
    }

    /// Record a bot-authenticated observation `hours_ago`, replacing roles in that guild.
    pub async fn observe(
        &self,
        actor: &Actor,
        guild: &str,
        status: &str,
        roles: &[&str],
        hours_ago: i64,
    ) {
        let mut tx = self.state.pool.begin().await.unwrap();
        sqlx::query(
            "INSERT INTO discord_membership_snapshots (discord_id, guild_id, membership_status, verified_at, revision)
             VALUES ($1, $2, $3, CASE WHEN $3 = 'unknown' THEN NULL ELSE clock_timestamp() - make_interval(hours => $4::int) END, 1)
             ON CONFLICT (discord_id, guild_id) DO UPDATE SET membership_status = EXCLUDED.membership_status,
                 verified_at = EXCLUDED.verified_at, revision = discord_membership_snapshots.revision + 1",
        )
        .bind(&actor.id)
        .bind(guild)
        .bind(status)
        .bind(hours_ago as i32)
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query("DELETE FROM user_discord_roles WHERE discord_id = $1 AND guild_id = $2")
            .bind(&actor.id)
            .bind(guild)
            .execute(&mut *tx)
            .await
            .unwrap();
        for role in roles {
            sqlx::query(
                "INSERT INTO user_discord_roles (discord_id, discord_role_id, guild_id, synced_at)
                 VALUES ($1, $2, $3, now())",
            )
            .bind(&actor.id)
            .bind(*role)
            .bind(guild)
            .execute(&mut *tx)
            .await
            .unwrap();
        }
        tx.commit().await.unwrap();
    }

    pub async fn call(
        &self,
        actor: &Actor,
        method: &str,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {}", actor.token));
        if body.is_some() {
            request = request.header(header::CONTENT_TYPE, "application/json");
        }
        let request = request
            .body(body.map_or(Body::empty(), |value| Body::from(value.to_string())))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// One game-server request authenticated with the ingest service token.
    pub async fn service_call(&self, uri: &str, body: Value) -> (StatusCode, Value) {
        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("x-service-token", self.state.cfg.service_token.as_str())
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// The catalog mission played by attachment `mission`.
    pub async fn catalog_mission(&self, mission: usize) -> Uuid {
        sqlx::query_scalar("SELECT mission_id FROM event_missions WHERE id = $1")
            .bind(self.missions[mission])
            .fetch_one(&self.state.pool)
            .await
            .unwrap()
    }

    /// Move the event and every attachment start `hours` into the past.
    pub async fn move_schedule_into_past(&self, hours: i32) {
        let mut tx = self.state.pool.begin().await.unwrap();
        sqlx::query("UPDATE events SET start_time = clock_timestamp() - make_interval(hours => $2) WHERE id = $1")
            .bind(self.event)
            .bind(hours)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE event_missions SET start_time = clock_timestamp() - make_interval(hours => $2) WHERE event_id = $1",
        )
        .bind(self.event)
        .bind(hours)
        .execute(&mut *tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    pub async fn register(
        &self,
        actor: &Actor,
        mission: usize,
        slot: Option<usize>,
    ) -> (StatusCode, Value) {
        let slot = slot
            .map(|slot| self.slots[mission][slot].to_string())
            .unwrap_or_default();
        self.call(
            actor,
            "POST",
            &format!("/api/v1/event-missions/{}/register", self.missions[mission]),
            Some(json!({ "slot_id": slot })),
        )
        .await
    }

    pub async fn withdraw(&self, actor: &Actor, mission: usize) -> (StatusCode, Value) {
        self.call(
            actor,
            "DELETE",
            &format!("/api/v1/event-missions/{}/register", self.missions[mission]),
            None,
        )
        .await
    }

    pub async fn assign(
        &self,
        actor: &Actor,
        mission: usize,
        slot: usize,
        target: &Actor,
    ) -> (StatusCode, Value) {
        self.call(
            actor,
            "PUT",
            &format!(
                "/api/v1/event-missions/{}/slots/{}/assign",
                self.missions[mission], self.slots[mission][slot]
            ),
            Some(json!({ "discord_id": target.id })),
        )
        .await
    }

    pub async fn access_revision(&self) -> i64 {
        sqlx::query_scalar("SELECT access_revision FROM events WHERE id = $1")
            .bind(self.event)
            .fetch_one(&self.state.pool)
            .await
            .unwrap()
    }

    pub async fn put_event_policy(&self, policy: Value) -> (StatusCode, Value) {
        let revision = self.access_revision().await;
        self.call(
            &self.admin,
            "PUT",
            &format!("/api/v1/events/{}/access-policy", self.event),
            Some(json!({ "expected_access_revision": revision, "policy": policy })),
        )
        .await
    }

    pub async fn put_squad_policy(
        &self,
        mission: usize,
        squad: &str,
        policy: Value,
    ) -> (StatusCode, Value) {
        let revision = self.access_revision().await;
        self.call(
            &self.admin,
            "PUT",
            &format!(
                "/api/v1/event-missions/{}/squads/BLUFOR/{squad}/access-policy",
                self.missions[mission]
            ),
            Some(json!({ "expected_access_revision": revision, "policy": policy })),
        )
        .await
    }

    pub async fn put_slot_policy(
        &self,
        mission: usize,
        slot: usize,
        policy: Value,
    ) -> (StatusCode, Value) {
        let revision = self.access_revision().await;
        self.call(
            &self.admin,
            "PUT",
            &format!(
                "/api/v1/event-missions/{}/slots/{}/access-policy",
                self.missions[mission], self.slots[mission][slot]
            ),
            Some(json!({ "expected_access_revision": revision, "policy": policy })),
        )
        .await
    }

    pub async fn put_quotas(&self, quotas: Value) -> (StatusCode, Value) {
        let revision = self.access_revision().await;
        self.call(
            &self.admin,
            "PUT",
            &format!("/api/v1/events/{}/reservation-quotas", self.event),
            Some(json!({ "expected_access_revision": revision, "reservation_quotas": quotas })),
        )
        .await
    }

    /// Members and guests draw from uncapped pools that are already open; the open pool stays
    /// closed. New events otherwise grant guests no places.
    pub async fn open_member_and_guest_pools(&self) {
        let opened = self
            .put_quotas(json!({
                "member": {"seats": null, "opens_at": "2020-01-01T00:00:00Z"},
                "guest": {"seats": null, "opens_at": "2020-01-01T00:00:00Z"},
                "open": {"seats": 0, "opens_at": "2020-01-01T00:00:00Z"}
            }))
            .await;
        assert_eq!(opened.0, StatusCode::OK, "{opened:?}");
    }

    pub async fn create_group(&self, name: &str, source: Value) -> Uuid {
        let revision = self.access_revision().await;
        let (status, body) = self
            .call(
                &self.admin,
                "POST",
                &format!("/api/v1/events/{}/groups", self.event),
                Some(
                    json!({ "expected_access_revision": revision, "name": name, "source": source }),
                ),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["access"]["groups"]
            .as_array()
            .unwrap()
            .iter()
            .find(|group| group["name"] == name)
            .and_then(|group| group["id"].as_str())
            .unwrap()
            .parse()
            .unwrap()
    }

    pub async fn add_roster(&self, group: Uuid, member: &Actor) -> (StatusCode, Value) {
        let revision = self.access_revision().await;
        self.call(
            &self.admin,
            "PUT",
            &format!(
                "/api/v1/events/{}/groups/{group}/members/{}",
                self.event, member.id
            ),
            Some(json!({ "expected_access_revision": revision })),
        )
        .await
    }

    /// `(registration, reservation_state, slot, release_reason)` of the actor in `mission`.
    pub async fn registration(
        &self,
        actor: &Actor,
        mission: usize,
    ) -> Option<(Uuid, String, Option<Uuid>, Option<String>)> {
        sqlx::query_as(
            "SELECT id, reservation_state::text, slot_id, release_reason FROM event_registrations
             WHERE event_mission_id = $1 AND discord_id = $2",
        )
        .bind(self.missions[mission])
        .bind(&actor.id)
        .fetch_optional(&self.state.pool)
        .await
        .unwrap()
    }

    /// The actor's active allocation kind in this event.
    pub async fn allocation(&self, actor: &Actor) -> Option<String> {
        sqlx::query_scalar(
            "SELECT quota_kind FROM event_participant_allocations
             WHERE event_id = $1 AND discord_id = $2 AND released_at IS NULL",
        )
        .bind(self.event)
        .bind(&actor.id)
        .fetch_optional(&self.state.pool)
        .await
        .unwrap()
    }

    pub async fn occupant(&self, mission: usize, slot: usize) -> Option<String> {
        sqlx::query_scalar("SELECT assigned_to FROM orbat_slots WHERE id = $1")
            .bind(self.slots[mission][slot])
            .fetch_one(&self.state.pool)
            .await
            .unwrap()
    }

    pub async fn audit_count(&self, action: &str, target: &str) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM audit_logs WHERE action = $1 AND target_id = $2")
            .bind(action)
            .bind(target)
            .fetch_one(&self.state.pool)
            .await
            .unwrap()
    }

    pub fn pool(&self) -> &PgPool {
        &self.state.pool
    }
}

impl Fixture {
    /// Hold the event parent lock in an open transaction until the caller commits it.
    pub async fn event_barrier(&self) -> (sqlx::Transaction<'static, sqlx::Postgres>, i32) {
        let mut transaction = self.state.pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM events WHERE id = $1 FOR NO KEY UPDATE")
            .bind(self.event)
            .fetch_one(&mut *transaction)
            .await
            .unwrap();
        let pid = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *transaction)
            .await
            .unwrap();
        (transaction, pid)
    }

    /// Wait until at least `minimum` sessions queue behind the barrier owner.
    pub async fn wait_for_blocked(&self, owner: i32, minimum: i64) {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let count: i64 = sqlx::query_scalar(
                    "WITH RECURSIVE blocked(pid) AS (
                     SELECT pid FROM pg_stat_activity WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid))
                     UNION SELECT a.pid FROM pg_stat_activity a JOIN blocked b ON b.pid = ANY(pg_blocking_pids(a.pid))
                     WHERE a.datname = current_database()) SELECT count(*) FROM blocked",
                )
                .bind(owner)
                .fetch_one(&self.state.pool)
                .await
                .unwrap();
                if count >= minimum {
                    return;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("requests must reach the controlled event lock barrier");
    }

    /// Seed a waiting registration directly, as if queued at `hours_ago`.
    pub async fn seed_waiting(&self, actor: &Actor, mission: usize, hours_ago: i32) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO event_registrations (event_mission_id, discord_id, reservation_state, queue_entered_at)
             VALUES ($1, $2, 'waitlisted', clock_timestamp() - make_interval(hours => $3)) RETURNING id",
        )
        .bind(self.missions[mission])
        .bind(&actor.id)
        .bind(hours_ago)
        .fetch_one(&self.state.pool)
        .await
        .unwrap()
    }
}

/// A policy with one grant of the given conditions.
pub fn single_grant(conditions: Value) -> Value {
    json!({ "grants": [{ "conditions": conditions }] })
}

pub fn tbd_members() -> Value {
    single_grant(json!([{ "kind": "tbd_member" }]))
}

pub fn named(account: &Actor) -> Value {
    single_grant(json!([{ "kind": "named_account", "discord_id": account.id }]))
}
