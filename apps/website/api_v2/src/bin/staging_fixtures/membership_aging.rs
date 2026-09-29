//! `age-membership-snapshot`: stages an account's main-guild membership snapshot as verified a
//! given number of hours ago, the precondition of the staging Discord procedure's grace step.
//!
//! **Role:** moves `verified_at` of one `discord_membership_snapshots` row back in time, with a
//! `staging.membership_snapshot_aged` audit row.
//!
//! **Position:** a row of the subcommand table in `main.rs`. The `staging discord` harness runs it
//! on the host while the outage drop-in holds the API away from Discord, so no successful
//! observation replaces the staged age before the browser reads `/me`; the receipt records the age
//! as a staged precondition, never as elapsed time.
//!
//! **Signals & state:** one transaction under the account lock, the lock every membership writer
//! takes first.
//!
//! **Invariants:** only a verified member snapshot of the `DISCORD_GUILD_ID` guild ages; a
//! snapshot whose refresh lease is live is refused, because the refresh in flight would replace
//! the staged age; the membership status, roles, `last_error`, `revision` and refresh schedule
//! stay as they are; the new `verified_at` and its audit row commit together; a dry run writes
//! nothing.

use chrono::{DateTime, Duration, Utc};
use website_api::administration::services::required_audit::append_system_audit;
use website_api::identity_and_access::services::account_authority::lock_account;

use crate::argument_list::ArgumentList;
use crate::guarded_context::{GuardedContext, ParsedSubcommand};
use crate::tool_failure::ToolFailure;

/// The audit action of an applied aging.
const AGED_AUDIT_ACTION: &str = "staging.membership_snapshot_aged";
/// The oldest age a snapshot is staged at: a week, well past every grace window.
pub(crate) const MAXIMUM_AGE_HOURS: u32 = 168;

/// Which snapshot ages, and by how much.
struct AgingPlan {
    discord_id: String,
    hours: u32,
}

/// The snapshot as the transaction reads it.
#[derive(sqlx::FromRow)]
struct SnapshotState {
    membership_status: String,
    verified_at: Option<DateTime<Utc>>,
    lease_live: bool,
    database_now: DateTime<Utc>,
}

/// Parse `--discord-id <id> --hours <h>`.
pub(crate) fn parse(arguments: &mut ArgumentList) -> Result<ParsedSubcommand, ToolFailure> {
    let discord_id = arguments.required("--discord-id")?;
    if discord_id.is_empty() || !discord_id.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ToolFailure::refused(format!(
            "--discord-id takes a decimal Discord id, not `{discord_id}`"
        )));
    }
    let hours = arguments.required_parsed::<u32>("--hours", "a whole number of hours")?;
    if !(1..=MAXIMUM_AGE_HOURS).contains(&hours) {
        return Err(ToolFailure::refused(format!(
            "--hours takes 1 to {MAXIMUM_AGE_HOURS}, not {hours}"
        )));
    }
    let plan = AgingPlan { discord_id, hours };
    Ok(Box::new(move |context| Box::pin(age(context, plan))))
}

async fn age(context: GuardedContext, plan: AgingPlan) -> Result<(), ToolFailure> {
    let guild = context
        .api_environment
        .non_empty("DISCORD_GUILD_ID")
        .ok_or_else(|| {
            ToolFailure::refused(format!(
                "DISCORD_GUILD_ID is not set in {}; the aged snapshot is the main guild's",
                context.api_environment.path().display()
            ))
        })?
        .to_owned();
    let id = &plan.discord_id;
    let mut transaction = context.pool.begin().await?;
    if !lock_account(&mut transaction, id).await? {
        return Err(ToolFailure::refused(format!(
            "no account has the Discord id {id}"
        )));
    }
    let state: SnapshotState = sqlx::query_as(
        "SELECT membership_status, verified_at,
                COALESCE(lease_expires_at > clock_timestamp(), false) AS lease_live,
                clock_timestamp() AS database_now
         FROM discord_membership_snapshots WHERE discord_id = $1 AND guild_id = $2",
    )
    .bind(id)
    .bind(&guild)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| {
        ToolFailure::refused(format!(
            "account {id} has no membership snapshot in the main guild {guild}"
        ))
    })?;
    let Some(verified_at) = state
        .verified_at
        .filter(|_| state.membership_status == "member")
    else {
        return Err(ToolFailure::refused(format!(
            "the main-guild snapshot of {id} is {}, not a verified membership",
            state.membership_status
        )));
    };
    if state.lease_live {
        return Err(ToolFailure::refused(format!(
            "a membership refresh of {id} holds the snapshot's lease; retry once it ends"
        )));
    }
    let staged = state.database_now - Duration::hours(i64::from(plan.hours));
    println!(
        "age plan: discord_id={id} guild_id={guild} verified_at={} staged_verified_at={} hours={}",
        verified_at.to_rfc3339(),
        staged.to_rfc3339(),
        plan.hours
    );
    if !context.mode.writes() {
        return Ok(());
    }
    let aged: DateTime<Utc> = sqlx::query_scalar(
        "UPDATE discord_membership_snapshots
         SET verified_at = clock_timestamp() - make_interval(hours => $3)
         WHERE discord_id = $1 AND guild_id = $2 RETURNING verified_at",
    )
    .bind(id)
    .bind(&guild)
    .bind(i32::try_from(plan.hours).unwrap_or(i32::MAX))
    .fetch_one(&mut *transaction)
    .await?;
    append_system_audit(
        &mut transaction,
        AGED_AUDIT_ACTION,
        "user",
        id,
        &format!(
            "staged the main-guild membership snapshot of {id} as verified {} hours ago \
             (staging precondition)",
            plan.hours
        ),
    )
    .await?;
    transaction.commit().await?;
    println!(
        "membership snapshot aged: discord_id={id} guild_id={guild} verified_at={} hours={}",
        aged.to_rfc3339(),
        plan.hours
    );
    Ok(())
}
