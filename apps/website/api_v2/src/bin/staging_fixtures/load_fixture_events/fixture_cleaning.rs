//! `clean-load-fixture-events`: removes the load fixture's events, and with them everything a load
//! run registered on them.
//!
//! **Role:** finds the fixture events, prints what their removal takes with them, and with
//! `--apply` deletes their registrations' history, their registrations and the events, leaving a
//! `event.load_fixture_removed` audit row for each.
//!
//! **Position:** undoes `fixture_seeding.rs`; the load procedure runs it before
//! `clean-load-population`, and it deletes no account.
//!
//! **Signals & state:** one database transaction.
//!
//! **Invariants:** it deletes only events whose title starts with `[Load fixture]` and whose
//! author lies in the reserved range, soft-deleted ones included, and what hangs off them; a dry
//! run writes nothing. The removal is all or nothing: the history rows go first, then the
//! registrations (so no slot deletion leaves a kicked-seat audit row behind), then the events,
//! whose attachments, slots, reservations, pools, allocations and groups cascade with them. A row
//! that restricts the delete, such as a deployment or a live slot occupancy of a fixture event,
//! fails the run and rolls it back. Every audit row stays.

use uuid::Uuid;
use website_api::administration::services::required_audit::append_system_audit;

use super::fixture_plan::FIXTURE_TITLE_PREFIX;
use crate::guarded_context::GuardedContext;
use crate::reserved_accounts::RESERVED_DISCORD_ID_PATTERN;
use crate::tool_failure::ToolFailure;

/// The history rows of the registrations on the fixture events' attachments, as the `FROM` clause
/// both the census and the delete complete.
const REGISTRATION_HISTORY_SCOPE: &str = "FROM event_registration_history history
     WHERE history.registration_id IN (
         SELECT registration.id FROM event_registrations registration
         JOIN event_missions attachment ON attachment.id = registration.event_mission_id
         WHERE attachment.event_id = ANY($1))";
/// The registrations on the fixture events' attachments, as the same kind of `FROM` clause.
const REGISTRATION_SCOPE: &str = "FROM event_registrations registration
     WHERE registration.event_mission_id IN (
         SELECT attachment.id FROM event_missions attachment WHERE attachment.event_id = ANY($1))";

/// Run the cleaning: the census, the plan, and the deletes when the run applies.
pub(super) async fn clean(context: GuardedContext) -> Result<(), ToolFailure> {
    let mut transaction = context.pool.begin().await?;
    let fixtures: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id, COALESCE(name_override, '') AS name_override, created_by FROM events
         WHERE starts_with(name_override, $1) AND created_by ~ $2
         ORDER BY name_override, id FOR UPDATE",
    )
    .bind(FIXTURE_TITLE_PREFIX)
    .bind(RESERVED_DISCORD_ID_PATTERN)
    .fetch_all(&mut *transaction)
    .await?;
    let events: Vec<Uuid> = fixtures.iter().map(|(id, _, _)| *id).collect();
    let history = count(&mut transaction, REGISTRATION_HISTORY_SCOPE, &events).await?;
    let registrations = count(&mut transaction, REGISTRATION_SCOPE, &events).await?;
    for (id, title, author) in &fixtures {
        println!("plan remove event={id} title=\"{title}\" author={author}");
    }
    println!(
        "plan remove {} fixture events, {registrations} registrations and {history} registration \
         history rows",
        events.len()
    );
    if !context.mode.writes() {
        return Ok(());
    }

    delete(&mut transaction, REGISTRATION_HISTORY_SCOPE, &events).await?;
    delete(&mut transaction, REGISTRATION_SCOPE, &events).await?;
    sqlx::query("DELETE FROM events WHERE id = ANY($1)")
        .bind(&events)
        .execute(&mut *transaction)
        .await?;
    for (id, title, _) in &fixtures {
        append_system_audit(
            &mut transaction,
            "event.load_fixture_removed",
            "event",
            &id.to_string(),
            &format!(
                "Load fixture event {title} removed with its attachments, slots and registrations"
            ),
        )
        .await?;
    }
    transaction.commit().await?;
    println!(
        "removed {} fixture events, {registrations} registrations and {history} registration \
         history rows",
        events.len()
    );
    Ok(())
}

/// The rows of `scope` for `events`.
async fn count(
    connection: &mut sqlx::PgConnection,
    scope: &str,
    events: &[Uuid],
) -> Result<i64, ToolFailure> {
    Ok(
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) {scope}")))
            .bind(events)
            .fetch_one(connection)
            .await?,
    )
}

/// Delete the rows of `scope` for `events`.
async fn delete(
    connection: &mut sqlx::PgConnection,
    scope: &str,
    events: &[Uuid],
) -> Result<(), ToolFailure> {
    sqlx::query(sqlx::AssertSqlSafe(format!("DELETE {scope}")))
        .bind(events)
        .execute(connection)
        .await?;
    Ok(())
}
