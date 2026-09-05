//! T-940.6 — the 0025 audit triggers and the LISTEN/NOTIFY admin stream.
//!
//! Anchor 2026-09-04: `handlers/admin/audit.rs:161` re-SELECTed `audit_logs` every 2 s per
//! connected admin, and three actions wrote no audit row at all — event create, mission
//! soft-delete and slot kick. The rows now come from row triggers in `0025_audit_notify.sql`
//! (the handler files belong to other slices this wave), and every `audit_logs` insert raises
//! `pg_notify('audit_log', id)` so `services::audit_notify` can push instead of poll.
//!
//! Skips (`skip:` line) unless `TEST_DATABASE_URL` is set; the wave gate and `cargo xtask db
//! test-it` always set it, so a printed skip is a red there. Each fixture gets fresh ids, so the
//! assertions are scoped to the rows this test planted and survive a shared, dirty database.

mod common;

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;
use website_api::db;

/// One planted `audit_logs` row, as the trigger wrote it.
#[derive(Debug, sqlx::FromRow)]
struct AuditRow {
    severity: String,
    actor_id: Option<String>,
    actor_name: Option<String>,
    message: String,
    target_type: Option<String>,
    target_id: Option<String>,
    metadata: Option<Value>,
}

async fn boot() -> Option<PgPool> {
    let url = common::require_test_database_url()?;
    Some(db::connect(&url).await.expect("connect"))
}

/// A fresh, snowflake-shaped discord id so parallel tests never share a user row.
fn fresh_discord_id() -> String {
    let n = Uuid::new_v4().as_u128() % 1_000_000_000_000;
    format!("940600{n:012}")
}

async fn seed_user(pool: &PgPool, username: &str) -> String {
    let id = fresh_discord_id();
    common::seed_user(pool, &id, username, &common::unique_arma("t9406"), "admin").await;
    id
}

async fn seed_event(pool: &PgPool, creator: &str, name: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, created_by, created_at, updated_at) \
         VALUES ($1, now() + interval '1 day', $2, now(), now()) RETURNING id",
    )
    .bind(name)
    .bind(creator)
    .fetch_one(pool)
    .await
    .expect("seed event")
}

async fn seed_mission(pool: &PgPool, author: &str, title: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO missions (title, author_id, terrain, game_mode, max_players, created_at, updated_at) \
         VALUES ($1, $2, 'everon', 'pve_coop', 40, now(), now()) RETURNING id",
    )
    .bind(title)
    .bind(author)
    .fetch_one(pool)
    .await
    .expect("seed mission")
}

async fn seed_event_mission(pool: &PgPool, event: Uuid, mission: Uuid) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO event_missions (event_id, mission_id, start_time, created_at, updated_at) \
         VALUES ($1, $2, now() + interval '1 day', now(), now()) RETURNING id",
    )
    .bind(event)
    .bind(mission)
    .fetch_one(pool)
    .await
    .expect("seed event mission")
}

async fn seed_slot(pool: &PgPool, em: Uuid, callsign: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO orbat_slots (event_mission_id, faction, squad, callsign, role, slot_index) \
         VALUES ($1, 'BLUFOR', 'Alpha', $2, 'Rifleman', 1) RETURNING id",
    )
    .bind(em)
    .bind(callsign)
    .fetch_one(pool)
    .await
    .expect("seed slot")
}

async fn seed_registration(pool: &PgPool, em: Uuid, who: &str, slot: Option<Uuid>) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, state) \
         VALUES ($1, $2, $3, 'registered') RETURNING id",
    )
    .bind(em)
    .bind(who)
    .bind(slot)
    .fetch_one(pool)
    .await
    .expect("seed registration")
}

async fn audit_rows(pool: &PgPool, action: &str, target_id: &str) -> Vec<AuditRow> {
    sqlx::query_as(
        "SELECT severity::text AS severity, actor_id, actor_name, message, target_type, target_id, metadata \
         FROM audit_logs WHERE action = $1 AND target_id = $2 ORDER BY id",
    )
    .bind(action)
    .bind(target_id)
    .fetch_all(pool)
    .await
    .expect("select audit rows")
}

/// Creating an event (`INSERT INTO events`, the only write `create_event` does to that table)
/// must leave exactly one `event.create` row naming the creator.
///
/// RED on main (no 0025): 0 rows. RED if the trigger drops `created_by`: actor asserts fire.
#[tokio::test]
async fn event_insert_writes_an_event_create_audit_row() {
    let Some(pool) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let creator = seed_user(&pool, "t9406-creator").await;
    let event = seed_event(&pool, &creator, "T-940.6 Operation Lantern").await;

    let rows = audit_rows(&pool, "event.create", &event.to_string()).await;
    assert_eq!(
        rows.len(),
        1,
        "exactly one event.create audit row for event {event}, got {rows:?}"
    );
    let r = &rows[0];
    assert_eq!(r.severity, "info");
    assert_eq!(r.actor_id.as_deref(), Some(creator.as_str()));
    assert_eq!(r.actor_name.as_deref(), Some("t9406-creator"));
    assert_eq!(r.target_type.as_deref(), Some("event"));
    assert_eq!(r.target_id.as_deref(), Some(event.to_string().as_str()));
    assert!(
        r.message.contains("T-940.6 Operation Lantern"),
        "message names the event: {:?}",
        r.message
    );
}

/// Soft-deleting a mission (`UPDATE missions SET deleted_at = now()`, missions.rs `delete_mission`)
/// writes one `mission.delete` row — and only the NULL → NOT NULL edge does: a rename, a repeated
/// soft-delete and a restore write nothing.
///
/// RED on main: 0 rows. RED if the trigger loses its WHEN clause: the rename or the repeat
/// produces a second row and the `== 1` asserts fire.
#[tokio::test]
async fn mission_soft_delete_writes_one_mission_delete_audit_row() {
    let Some(pool) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let author = seed_user(&pool, "t9406-author").await;
    let mission = seed_mission(&pool, &author, "T-940.6 Night Ferry").await;
    let target = mission.to_string();

    sqlx::query("UPDATE missions SET title = 'T-940.6 Night Ferry (renamed)' WHERE id = $1")
        .bind(mission)
        .execute(&pool)
        .await
        .expect("rename");
    assert!(
        audit_rows(&pool, "mission.delete", &target).await.is_empty(),
        "a rename is not a delete"
    );

    sqlx::query("UPDATE missions SET deleted_at = now() WHERE id = $1")
        .bind(mission)
        .execute(&pool)
        .await
        .expect("soft delete");
    let rows = audit_rows(&pool, "mission.delete", &target).await;
    assert_eq!(rows.len(), 1, "one mission.delete row after soft-delete, got {rows:?}");
    let r = &rows[0];
    assert_eq!(r.severity, "warn");
    assert_eq!(r.actor_id, None, "delete_mission stamps no actor; none may be invented");
    assert_eq!(r.target_type.as_deref(), Some("mission"));
    assert_eq!(r.target_id.as_deref(), Some(target.as_str()));
    assert!(
        r.message.contains("Night Ferry"),
        "message names the mission: {:?}",
        r.message
    );

    sqlx::query("UPDATE missions SET deleted_at = now() WHERE id = $1")
        .bind(mission)
        .execute(&pool)
        .await
        .expect("repeat soft delete");
    sqlx::query("UPDATE missions SET deleted_at = NULL WHERE id = $1")
        .bind(mission)
        .execute(&pool)
        .await
        .expect("restore");
    assert_eq!(
        audit_rows(&pool, "mission.delete", &target).await.len(),
        1,
        "a repeated soft-delete and a restore write no further row"
    );
}

/// Kicking a player off a slot is `clear_slot`'s statement, verbatim: `UPDATE event_registrations
/// SET slot_id = NULL WHERE event_mission_id = $1 AND slot_id = $2`. That, and only that edge
/// (NOT NULL → NULL), writes an `event.slot_kick` row: taking a seat writes nothing, and a withdraw
/// is a DELETE that never fires it. Deleting the ORBAT slot itself reaches the same edge through
/// the 0018 `ON DELETE SET NULL` and is audited too — the occupant lost the seat either way.
///
/// RED on main: 0 rows. RED if the trigger fires on every slot_id change: the assign step
/// produces a row and the first `is_empty()` fires.
#[tokio::test]
async fn clearing_a_slot_writes_an_event_slot_kick_audit_row() {
    let Some(pool) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let creator = seed_user(&pool, "t9406-leader").await;
    let player = seed_user(&pool, "t9406-kicked-player").await;
    let event = seed_event(&pool, &creator, "T-940.6 Slot Kick").await;
    let mission = seed_mission(&pool, &creator, "T-940.6 Slot Kick mission").await;
    let em = seed_event_mission(&pool, event, mission).await;
    let slot = seed_slot(&pool, em, "A1").await;
    let target = em.to_string();

    let reg = seed_registration(&pool, em, &player, None).await;
    sqlx::query("UPDATE event_registrations SET slot_id = $1 WHERE id = $2")
        .bind(slot)
        .bind(reg)
        .execute(&pool)
        .await
        .expect("assign");
    assert!(
        audit_rows(&pool, "event.slot_kick", &target).await.is_empty(),
        "taking a seat is not a kick"
    );

    sqlx::query(
        "UPDATE event_registrations SET slot_id = NULL WHERE event_mission_id = $1 AND slot_id = $2",
    )
    .bind(em)
    .bind(slot)
    .execute(&pool)
    .await
    .expect("clear slot");
    let rows = audit_rows(&pool, "event.slot_kick", &target).await;
    assert_eq!(rows.len(), 1, "one event.slot_kick row after clear_slot, got {rows:?}");
    let r = &rows[0];
    assert_eq!(r.severity, "warn");
    assert_eq!(r.actor_id, None, "clear_slot stamps no actor; none may be invented");
    assert_eq!(r.target_type.as_deref(), Some("event_mission"));
    assert_eq!(r.target_id.as_deref(), Some(target.as_str()));
    assert!(
        r.message.contains("t9406-kicked-player") && r.message.contains("A1"),
        "message names the player and the slot: {:?}",
        r.message
    );
    let meta = r.metadata.as_ref().expect("metadata carries the kicked registration");
    assert_eq!(meta["discord_id"], Value::String(player.clone()));
    assert_eq!(meta["slot_id"], Value::String(slot.to_string()));
    assert_eq!(meta["registration_id"], Value::String(reg.to_string()));

    sqlx::query("DELETE FROM event_registrations WHERE id = $1")
        .bind(reg)
        .execute(&pool)
        .await
        .expect("withdraw");
    assert_eq!(
        audit_rows(&pool, "event.slot_kick", &target).await.len(),
        1,
        "a withdraw (DELETE) is not a kick"
    );

    // Seat again, then delete the ORBAT slot: the FK's SET NULL is the same edge.
    seed_registration(&pool, em, &player, Some(slot)).await;
    sqlx::query("DELETE FROM orbat_slots WHERE id = $1")
        .bind(slot)
        .execute(&pool)
        .await
        .expect("delete slot");
    assert_eq!(
        audit_rows(&pool, "event.slot_kick", &target).await.len(),
        2,
        "the ON DELETE SET NULL path audits the lost seat too"
    );
}
