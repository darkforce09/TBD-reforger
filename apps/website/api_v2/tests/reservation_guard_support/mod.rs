//! Owned actors, reservation fixtures and PostgreSQL lock barriers for reservation guard tests.

use axum::{Router, http::StatusCode};
use serde_json::{Value, json};
use std::time::Duration;
use uuid::Uuid;
use website_api::core::{
    application_state::AppState, configuration::Config, database, http_router,
};

use crate::{common, telemetry_support};

pub const DEADLINE: Duration = Duration::from_secs(10);

pub struct Actor {
    pub id: String,
    pub token: String,
}

pub struct Fixture {
    pub state: AppState,
    pub app: Router,
    pub admin: Actor,
    pub leader: Actor,
    pub players: Vec<Actor>,
    pub event: Uuid,
    pub missions: Vec<Uuid>,
    pub slots: Vec<Vec<Uuid>>,
}

impl Fixture {
    pub async fn new(maximum: i64, seats: i32) -> Self {
        let url =
            common::require_test_database_url().expect("reservation guards require PostgreSQL");
        let pool = database::connect(&url).await.unwrap();
        database::migrate(&pool).await.unwrap();
        let state = AppState::new(pool, Config::for_tests(url, "reservation-guards"));
        let admin = actor(&state, "admin").await;
        let leader = actor(&state, "leader").await;
        let mut players = Vec::new();
        for _ in 0..4 {
            players.push(actor(&state, "enlisted").await);
        }
        let event: Uuid = sqlx::query_scalar(
            "INSERT INTO events(name_override, start_time, status, max_slots, created_by)
             VALUES ('Reservation guard fixture', clock_timestamp() + interval '3 days', 'open', $1, $2) RETURNING id")
            .bind(maximum).bind(&admin.id).fetch_one(&state.pool).await.unwrap();
        let mut missions = Vec::new();
        let mut slots = Vec::new();
        for _ in 0..2 {
            let mission: Uuid = sqlx::query_scalar(
                "INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status)
                 VALUES ('Reservation guard mission', $1, 'everon', 'pve_coop', 32, 'live') RETURNING id")
                .bind(&admin.id).fetch_one(&state.pool).await.unwrap();
            let attachment: Uuid = sqlx::query_scalar(
                "INSERT INTO event_missions(event_id, mission_id, start_time)
                 VALUES ($1, $2, clock_timestamp() + interval '3 days') RETURNING id",
            )
            .bind(event)
            .bind(mission)
            .fetch_one(&state.pool)
            .await
            .unwrap();
            let mut attachment_slots = Vec::new();
            for index in 0..seats {
                attachment_slots.push(sqlx::query_scalar(
                    "INSERT INTO orbat_slots(event_mission_id, faction, squad, role, slot_index)
                     VALUES ($1, 'USA', 'Alpha', 'Rifleman', $2) RETURNING id")
                    .bind(attachment).bind(index).fetch_one(&state.pool).await.unwrap());
            }
            missions.push(attachment);
            slots.push(attachment_slots);
        }
        let app = http_router::router(state.clone());
        Self {
            state,
            app,
            admin,
            leader,
            players,
            event,
            missions,
            slots,
        }
    }

    pub async fn call(
        &self,
        actor: &Actor,
        method: &str,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        telemetry_support::call(
            &self.app,
            method,
            uri,
            Some(&actor.token),
            None,
            body.as_ref().map(Value::to_string).as_deref(),
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
            Some(json!({"discord_id":target.id})),
        )
        .await
    }

    pub async fn clear(&self, actor: &Actor, mission: usize, slot: usize) -> (StatusCode, Value) {
        self.call(
            actor,
            "DELETE",
            &format!(
                "/api/v1/event-missions/{}/slots/{}/assign",
                self.missions[mission], self.slots[mission][slot]
            ),
            None,
        )
        .await
    }

    pub async fn register(
        &self,
        actor: &Actor,
        mission: usize,
        slot: Option<usize>,
    ) -> (StatusCode, Value) {
        self.call(actor, "POST", &format!("/api/v1/event-missions/{}/register", self.missions[mission]),
            Some(json!({"slot_id":slot.map(|slot| self.slots[mission][slot].to_string()).unwrap_or_default()}))).await
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

    pub async fn squad(&self, actor: &Actor, release: bool) -> (StatusCode, Value) {
        let action = if release { "release" } else { "reserve" };
        self.call(
            actor,
            "POST",
            &format!(
                "/api/v1/event-missions/{}/squads/{action}",
                self.missions[0]
            ),
            Some(json!({"squad":"Alpha"})),
        )
        .await
    }

    pub async fn operation(&self, name: &str, actor: &Actor) -> (StatusCode, Value) {
        match name {
            "assign" => self.assign(actor, 0, 1, &self.players[0]).await,
            "clear" => self.clear(actor, 0, 0).await,
            "hold" => self.squad(actor, false).await,
            "release" => self.squad(actor, true).await,
            "register" => self.register(actor, 0, Some(1)).await,
            "withdraw" => self.withdraw(actor, 0).await,
            _ => panic!("unknown reservation fixture operation {name}"),
        }
    }

    pub async fn seed_registration(
        &self,
        actor: &Actor,
        mission: usize,
        state: &str,
        slot: Option<usize>,
    ) {
        if let Some(slot) = slot {
            self.seed_occupant(actor, mission, slot).await;
        }
        let mut fixture = self.state.pool.begin().await.unwrap();
        let allocation = if matches!(state, "registered" | "legacy_unknown") {
            Some(
                common::participant_allocation(&mut fixture, self.missions[mission], &actor.id)
                    .await,
            )
        } else {
            None
        };
        sqlx::query(
            "INSERT INTO event_registrations(event_mission_id, discord_id, reservation_state, slot_id, allocation_id)
             VALUES ($1, $2, $3::registration_state, $4, $5)")
            .bind(self.missions[mission]).bind(&actor.id).bind(state)
            .bind(slot.map(|slot| self.slots[mission][slot])).bind(allocation)
            .execute(&mut *fixture).await.unwrap();
        fixture.commit().await.unwrap();
    }

    pub async fn seed_occupant(&self, actor: &Actor, mission: usize, slot: usize) {
        sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = clock_timestamp() WHERE id = $2")
            .bind(&actor.id).bind(self.slots[mission][slot]).execute(&self.state.pool).await.unwrap();
    }

    pub async fn seed_hold(&self) {
        sqlx::query("INSERT INTO orbat_reservations(event_mission_id, squad, reserved_by) VALUES ($1, 'Alpha', $2)")
            .bind(self.missions[0]).bind(&self.leader.id).execute(&self.state.pool).await.unwrap();
    }

    pub async fn snapshot(&self) -> Value {
        sqlx::query_scalar(
            "SELECT jsonb_build_object(
             'slots', (SELECT jsonb_agg(to_jsonb(s) ORDER BY s.id) FROM orbat_slots s WHERE event_mission_id = ANY($1)),
             'registrations', (SELECT jsonb_agg(to_jsonb(r) ORDER BY r.id) FROM event_registrations r WHERE event_mission_id = ANY($1)),
             'history', (SELECT jsonb_agg(to_jsonb(h) ORDER BY h.id) FROM event_registration_history h
                 JOIN event_registrations r ON r.id = h.registration_id WHERE r.event_mission_id = ANY($1)),
             'holds', (SELECT jsonb_agg(to_jsonb(h) ORDER BY h.id) FROM orbat_reservations h WHERE event_mission_id = ANY($1)),
             'assignment_audits', (SELECT jsonb_agg(to_jsonb(a) ORDER BY a.id) FROM audit_logs a
                 WHERE action = 'event.slot_assigned' AND actor_id = ANY($2)))")
            .bind(&self.missions).bind(vec![self.admin.id.clone(), self.leader.id.clone()])
            .fetch_one(&self.state.pool).await.unwrap()
    }

    pub async fn registration(
        &self,
        actor: &Actor,
        mission: usize,
    ) -> Option<(Uuid, String, Option<Uuid>)> {
        sqlx::query_as("SELECT id, reservation_state::text, slot_id FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2")
            .bind(self.missions[mission]).bind(&actor.id).fetch_optional(&self.state.pool).await.unwrap()
    }

    pub async fn occupants(&self, actor: &Actor, mission: usize) -> Vec<Uuid> {
        sqlx::query_scalar("SELECT id FROM orbat_slots WHERE event_mission_id = $1 AND assigned_to = $2 ORDER BY id")
            .bind(self.missions[mission]).bind(&actor.id).fetch_all(&self.state.pool).await.unwrap()
    }

    pub async fn assignment_evidence(&self) -> (i64, i64) {
        sqlx::query_as(
            "SELECT count(*), count(p.audit_id) FROM audit_logs a
            LEFT JOIN audit_publication_pending p ON p.audit_id = a.id
            WHERE a.action = 'event.slot_assigned' AND a.actor_id = $1",
        )
        .bind(&self.admin.id)
        .fetch_one(&self.state.pool)
        .await
        .unwrap()
    }

    pub async fn audit_evidence(&self, action: &str, target: Uuid) -> (i64, i64) {
        sqlx::query_as(
            "SELECT count(*), count(p.audit_id) FROM audit_logs a
            LEFT JOIN audit_publication_pending p ON p.audit_id = a.id
            WHERE a.action = $1 AND a.target_id = $2",
        )
        .bind(action)
        .bind(target.to_string())
        .fetch_one(&self.state.pool)
        .await
        .unwrap()
    }

    pub async fn registration_facts(&self, actor: &Actor, mission: usize) -> Value {
        sqlx::query_scalar(
            "SELECT jsonb_build_object('registration', to_jsonb(r),
            'history', (SELECT jsonb_agg(to_jsonb(h) ORDER BY h.id)
            FROM event_registration_history h WHERE h.registration_id = r.id))
            FROM event_registrations r WHERE event_mission_id = $1 AND discord_id = $2",
        )
        .bind(self.missions[mission])
        .bind(&actor.id)
        .fetch_one(&self.state.pool)
        .await
        .unwrap()
    }

    pub async fn seed_cancelled_registration(&self, actor: &Actor) {
        self.seed_registration(actor, 0, "withdrawn", None).await;
        sqlx::query(
            "UPDATE event_registrations SET release_reason = 'event_cancelled',
            withdrawn_at = clock_timestamp() - interval '1 hour',
            attendance_state = 'attended', legacy_attendance_state = 'attended'
            WHERE event_mission_id = $1 AND discord_id = $2",
        )
        .bind(self.missions[0])
        .bind(&actor.id)
        .execute(&self.state.pool)
        .await
        .unwrap();
    }

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
}

pub async fn actor(state: &AppState, role: &str) -> Actor {
    let id = format!("reservation-{role}-{}", Uuid::new_v4());
    let token =
        common::access_token(state, "reservation_guard_transactions", &id, role, true).await;
    Actor { id, token }
}

pub async fn wait_for_blocked(pool: &sqlx::PgPool, owner: i32, minimum: i64) {
    tokio::time::timeout(DEADLINE, async {
        loop {
            // Include waiters queued behind another waiter, not just the original lock owner.
            let count: i64 = sqlx::query_scalar(
                "WITH RECURSIVE blocked(pid) AS (
                 SELECT pid FROM pg_stat_activity WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid))
                 UNION SELECT a.pid FROM pg_stat_activity a JOIN blocked b ON b.pid = ANY(pg_blocking_pids(a.pid))
                 WHERE a.datname = current_database()) SELECT count(*) FROM blocked")
                .bind(owner).fetch_one(pool).await.unwrap();
            if count >= minimum { return; }
            tokio::task::yield_now().await;
        }
    }).await.expect("requests must reach the controlled reservation lock barrier");
}
