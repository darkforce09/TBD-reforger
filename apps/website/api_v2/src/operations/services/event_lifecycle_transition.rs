//! Materialize observed lifecycle state before mutable scheduling inputs can change its derivation.
use super::event_status_rules::is_pre_start;
use crate::administration::services::required_audit::append_system_audit;
use crate::operations::models::EventStatus;
use sqlx::PgConnection;
use uuid::Uuid;

/// Caller holds the event NO KEY UPDATE lock and observes both states after acquiring that lock.
/// Returns the automatic edges taken, with each edge and its required publication committed together.
pub(crate) async fn advance_locked_event(
    connection: &mut PgConnection,
    event: Uuid,
    stored: EventStatus,
    observed: EventStatus,
) -> sqlx::Result<(bool, bool)> {
    use EventStatus::{Completed, Live};
    let started = is_pre_start(stored) && matches!(observed, Live | Completed);
    let completed = observed == Completed && (stored == Live || started);
    if stored != observed && !started && !completed {
        return Err(sqlx::Error::Protocol(
            "invalid automatic lifecycle transition".into(),
        ));
    }
    if started {
        apply_edge(
            connection,
            event,
            Live,
            "event.auto_live",
            &format!(
                "start time reached: {} → live; registration closed",
                stored.as_str()
            ),
        )
        .await?;
    }
    if completed {
        apply_edge(
            connection,
            event,
            Completed,
            "event.auto_completed",
            "end horizon passed: live → completed",
        )
        .await?;
    }
    Ok((started, completed))
}

async fn apply_edge(
    connection: &mut PgConnection,
    event: Uuid,
    status: EventStatus,
    action: &str,
    message: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE events SET status = $2, updated_at = statement_timestamp() WHERE id = $1")
        .bind(event)
        .bind(status)
        .execute(&mut *connection)
        .await?;
    append_system_audit(connection, action, "event", &event.to_string(), message).await
}
