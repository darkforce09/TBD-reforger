//! Appends rows to the audit log, and resolves the display name every row is attributed to.

use sqlx::PgPool;

use crate::administration::models::audit_log::AuditSeverity;

/// Append a row to the audit log. Best-effort: an audit failure must not break the primary
/// action, so this returns `()` and logs on error. `id` is a bigint sequence; `created_at` is
/// set app-side.
#[allow(clippy::too_many_arguments)]
pub async fn write_audit(
    pool: &PgPool,
    severity: AuditSeverity,
    actor_id: Option<&str>,
    actor_name: &str,
    action: &str,
    message: &str,
    target_type: &str,
    target_id: &str,
) {
    let res = sqlx::query(
        "INSERT INTO audit_logs \
         (severity, actor_id, actor_name, action, message, target_type, target_id, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, now())",
    )
    .bind(severity)
    .bind(actor_id)
    .bind(actor_name)
    .bind(action)
    .bind(message)
    .bind(target_type)
    .bind(target_id)
    .execute(pool)
    .await;

    if let Err(e) = res {
        tracing::error!(action, error = %e, "audit write failed");
    }
}

/// Resolve a display name for audit messages, falling back to the `discord_id`. The COALESCE
/// tolerates a NULL username defensively only, since `migrations/0001_initial_schema.sql`
/// declares `username text NOT NULL` (writing NULL fails with SQLSTATE 23502).
///
/// **The guard below is `trim().is_empty()`, not `is_empty()`.** Every audit line in the crate
/// takes its `actor_name` from here, so an untrimmed guard lets a whitespace username *bypass
/// the `discord_id` fallback that exists to prevent exactly this*: with `username = '   '`,
/// `audit_logs.actor_name` is `'   '` and the message reads `"    set     role to admin"` — an
/// audit line naming neither actor nor target. That is worse than a missing entry, because it
/// still looks like a record. `username = ''` falls through correctly, which is what makes the
/// whitespace case a gap rather than a design choice.
///
/// **Why trimming is safe *here* when it usually is not.** The rule is that a trim on read must
/// agree with the trim on write:
/// - **No writer trims, so there is no counterpart to disagree with.** `users.username` has
///   exactly two writers — `identity_and_access/handlers/discord_oauth.rs` binds
///   `du.display_name()` (Discord's `global_name`, else `username`) with no trim and no guard at
///   any hop, and `identity_and_access/handlers/developer_login.rs` binds the literal
///   `'Dev Operator'`. No CHECK constraint, no trigger, no `btrim` in SQL, and no request body
///   anywhere in the crate carries a `username` field. `display_name()`
///   (`identity_and_access/services/discord_user_profile.rs`) selects on whether `global_name`
///   is blank, so a Discord `global_name` of `"   "` reaches `users.username` verbatim when that
///   selection is not trim-aware — this is the live path by which a blank-ish username arrives.
/// - **This value is never a key.** Every consumer passes it to [`write_audit`]'s `actor_name`
///   display column or interpolates it into `message`. `write_audit`'s `actor_id` is bound
///   separately from the caller's real `discord_id`, so audit-row identity never comes from this
///   string. It is used in no `WHERE`, comparison, join or `ORDER BY`. Changing the guard
///   therefore cannot change which row anything matches — the failure mode that makes a one-sided
///   trim catastrophic for a value that *is* a key (as `faction` is in `handlers/events`) does
///   not exist here.
///
/// **Fall-through, not display-trimming — deliberately only the former.** The guard treats
/// blank-ish as absent (`'   '` → `discord_id`), but a name that survives the guard is returned
/// **exactly as stored**, so `'  Sam  '` renders as `'  Sam  '` and *never* degrades into a
/// discord_id. Padding is cosmetic; namelessness is not, and trimming the returned value is a
/// presentation change rather than a correctness one.
pub async fn actor_display_name(pool: &PgPool, discord_id: &str) -> String {
    let name: Option<String> =
        sqlx::query_scalar("SELECT COALESCE(username, '') FROM users WHERE discord_id = $1")
            .bind(discord_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    match name {
        Some(n) if !n.trim().is_empty() => n,
        _ => discord_id.to_string(),
    }
}
