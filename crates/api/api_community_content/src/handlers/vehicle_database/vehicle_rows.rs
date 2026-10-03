//! The statements of the vehicle database: the live reads, the row lock, the writes and the
//! audit line each write appends.
//!
//! **Role:** every SQL statement the vehicle handlers run against `vehicle_databases`, the
//! transactional audit append of an accepted write, and the parse of the `{id}` path segment.
//! **Position:** called by the handlers of [`super`]; the reads take the pool and the writes the
//! caller's transaction. The audit line goes through
//! [`api_audit_log::required_audit::append_actor_audit`].
//! **Signals & state:** none; each function borrows the caller's pool or connection.
//! **Invariants:**
//! - A row with `deleted_at` set is invisible to the reads and to the lock, so every route that
//!   names it answers 404 and the list leaves it out.
//! - The list orders by `name ASC, id ASC`, a total order.
//! - Every select and `RETURNING` list `COALESCE`s the three optional columns to `''`, which
//!   [`VehicleDatabase`] leaves off the wire; the lifecycle columns never reach the wire.
//! - A write to an existing row runs after [`lock_live_vehicle`] took the row `FOR UPDATE`, and the
//!   audit line is appended after the write in the same transaction: the lock order is the
//!   vehicle row, then the audit row.
//! - The creator, editor and deleter stamps are the authenticated administrator's Discord id,
//!   never a value from the body.

use api_identifiers::DiscordUserId;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::validation::VehicleFields;
use crate::models::VehicleDatabase;
use api_audit_log::required_audit::append_actor_audit;
use api_foundation::error_handling::api_error::ApiError;

/// The `target_type` of every vehicle audit line.
const AUDIT_TARGET_TYPE: &str = "vehicle";

/// The accepted vehicle writes, each with its own audit action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VehicleWrite {
    Created,
    Replaced,
    Updated,
    Deleted,
}

impl VehicleWrite {
    /// The audit `action` of this write.
    fn audit_action(self) -> &'static str {
        match self {
            Self::Created => "vehicle.created",
            Self::Replaced => "vehicle.replaced",
            Self::Updated => "vehicle.updated",
            Self::Deleted => "vehicle.deleted",
        }
    }

    /// The verb that opens this write's audit message.
    fn audit_verb(self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::Replaced => "Replaced",
            Self::Updated => "Updated",
            Self::Deleted => "Deleted",
        }
    }
}

/// The `{id}` path segment as a row id; anything but a UUID answers 400.
pub(super) fn parse_vehicle_id(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::bad_request("invalid vehicle id"))
}

/// The answer for an id that names no live row.
fn vehicle_not_found() -> ApiError {
    ApiError::not_found("vehicle not found")
}

/// Every live row, ordered by name, then id.
pub(super) async fn list_live_vehicles(pool: &PgPool) -> Result<Vec<VehicleDatabase>, ApiError> {
    let rows = sqlx::query_as(
        "SELECT id, name, faction, armor_type, COALESCE(amphibious, '') AS amphibious, \
         COALESCE(primary_threat, '') AS primary_threat, \
         COALESCE(profile_image_url, '') AS profile_image_url \
         FROM vehicle_databases WHERE deleted_at IS NULL ORDER BY name ASC, id ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// The live row `id` names; a deleted or unknown id answers 404.
pub(super) async fn find_live_vehicle(
    pool: &PgPool,
    id: Uuid,
) -> Result<VehicleDatabase, ApiError> {
    let row: Option<VehicleDatabase> = sqlx::query_as(
        "SELECT id, name, faction, armor_type, COALESCE(amphibious, '') AS amphibious, \
         COALESCE(primary_threat, '') AS primary_threat, \
         COALESCE(profile_image_url, '') AS profile_image_url \
         FROM vehicle_databases WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.ok_or_else(vehicle_not_found)
}

/// Locks the live row `id` names `FOR UPDATE` in the caller's transaction and answers it as
/// stored; a deleted or unknown id answers 404.
pub(super) async fn lock_live_vehicle(
    connection: &mut PgConnection,
    id: Uuid,
) -> Result<VehicleDatabase, ApiError> {
    let row: Option<VehicleDatabase> = sqlx::query_as(
        "SELECT id, name, faction, armor_type, COALESCE(amphibious, '') AS amphibious, \
         COALESCE(primary_threat, '') AS primary_threat, \
         COALESCE(profile_image_url, '') AS profile_image_url \
         FROM vehicle_databases WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *connection)
    .await?;
    row.ok_or_else(vehicle_not_found)
}

/// Inserts a new row stamped as created and last changed by `actor`; `created_at` and
/// `updated_at` take their `now()` defaults.
pub(super) async fn insert_vehicle(
    connection: &mut PgConnection,
    fields: &VehicleFields,
    actor: &DiscordUserId,
) -> Result<VehicleDatabase, ApiError> {
    let row = sqlx::query_as(
        "INSERT INTO vehicle_databases \
         (name, faction, armor_type, amphibious, primary_threat, profile_image_url, created_by, updated_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7) \
         RETURNING id, name, faction, armor_type, COALESCE(amphibious, '') AS amphibious, \
         COALESCE(primary_threat, '') AS primary_threat, \
         COALESCE(profile_image_url, '') AS profile_image_url",
    )
    .bind(&fields.name)
    .bind(&fields.faction)
    .bind(&fields.armor_type)
    .bind(&fields.amphibious)
    .bind(&fields.primary_threat)
    .bind(&fields.profile_image_url)
    .bind(actor)
    .fetch_one(&mut *connection)
    .await?;
    Ok(row)
}

/// Stores every field of the row `id` names, stamped as last changed by `actor` now. The caller
/// holds the row's lock from [`lock_live_vehicle`].
pub(super) async fn store_vehicle_fields(
    connection: &mut PgConnection,
    id: Uuid,
    fields: &VehicleFields,
    actor: &DiscordUserId,
) -> Result<VehicleDatabase, ApiError> {
    let row = sqlx::query_as(
        "UPDATE vehicle_databases SET name = $2, faction = $3, armor_type = $4, amphibious = $5, \
         primary_threat = $6, profile_image_url = $7, updated_at = now(), updated_by = $8 \
         WHERE id = $1 AND deleted_at IS NULL \
         RETURNING id, name, faction, armor_type, COALESCE(amphibious, '') AS amphibious, \
         COALESCE(primary_threat, '') AS primary_threat, \
         COALESCE(profile_image_url, '') AS profile_image_url",
    )
    .bind(id)
    .bind(&fields.name)
    .bind(&fields.faction)
    .bind(&fields.armor_type)
    .bind(&fields.amphibious)
    .bind(&fields.primary_threat)
    .bind(&fields.profile_image_url)
    .bind(actor)
    .fetch_one(&mut *connection)
    .await?;
    Ok(row)
}

/// Marks the row `id` names deleted by `actor` now and answers it as stored. The caller holds the
/// row's lock from [`lock_live_vehicle`].
pub(super) async fn soft_delete_vehicle(
    connection: &mut PgConnection,
    id: Uuid,
    actor: &DiscordUserId,
) -> Result<VehicleDatabase, ApiError> {
    let row = sqlx::query_as(
        "UPDATE vehicle_databases SET deleted_at = now(), deleted_by = $2 \
         WHERE id = $1 AND deleted_at IS NULL \
         RETURNING id, name, faction, armor_type, COALESCE(amphibious, '') AS amphibious, \
         COALESCE(primary_threat, '') AS primary_threat, \
         COALESCE(profile_image_url, '') AS profile_image_url",
    )
    .bind(id)
    .bind(actor)
    .fetch_one(&mut *connection)
    .await?;
    Ok(row)
}

/// Appends the audit line of `write` on `vehicle` by `actor` in the caller's transaction.
/// `changed_keys`, when given, names the fields a PATCH changed.
pub(super) async fn append_vehicle_audit(
    connection: &mut PgConnection,
    actor: &DiscordUserId,
    write: VehicleWrite,
    vehicle: &VehicleDatabase,
    changed_keys: Option<&[&str]>,
) -> Result<(), ApiError> {
    let mut message = format!(
        "{} vehicle database entry '{}'",
        write.audit_verb(),
        vehicle.name
    );
    if let Some(keys) = changed_keys {
        let fields = if keys.is_empty() {
            "no fields".to_owned()
        } else {
            keys.join(", ")
        };
        message.push_str(&format!(" ({fields})"));
    }
    append_actor_audit(
        connection,
        actor,
        write.audit_action(),
        AUDIT_TARGET_TYPE,
        &vehicle.id.to_string(),
        &message,
    )
    .await?;
    Ok(())
}
