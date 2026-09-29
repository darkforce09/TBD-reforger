//! `clean-load-population`: deletes the synthetic accounts of a load population with every row
//! they own, and undoes a failed seeding.
//!
//! **Role:** counts, then deletes, the accounts of the reserved range and the rows that reference
//! them. The registrations go first, with their history and participation rows, because those
//! restrict the delete of a registration and a registration restricts the delete of its quota
//! allocation; then the fire missions and link codes the accounts created, which no foreign key
//! ties to them; then the accounts, whose sessions, refresh tokens, membership snapshots and
//! Discord roles, reservations, quota allocations and bookmarks cascade (`ON DELETE CASCADE`) and
//! whose slot assignments clear (`ON DELETE SET NULL`).
//!
//! **Position:** a row of the subcommand table in `main.rs`; `population_seeding` calls
//! [`delete_synthetic_accounts`] with the ids it created when a seeding fails.
//!
//! **Signals & state:** one transaction.
//!
//! **Invariants:** only rows of reserved-range accounts are deleted, and the registration history
//! and participation rows of their registrations; audit rows stay, since the audit log is
//! append-only, and an applied cleanup appends its own `staging.load_population_cleaned` row in
//! the same transaction; the reservation consistency checks, deferred to the commit, see no
//! synthetic participant left; a dry run counts and deletes nothing; a run that finds nothing to
//! delete writes nothing.

use sqlx::{AssertSqlSafe, PgConnection};
use website_api::administration::services::required_audit::append_system_audit;

use crate::argument_list::ArgumentList;
use crate::guarded_context::{GuardedContext, ParsedSubcommand};
use crate::reserved_accounts::{
    FIRST_RESERVED_DISCORD_ID, LAST_RESERVED_DISCORD_ID, RESERVED_DISCORD_ID_PATTERN,
};
use crate::tool_failure::ToolFailure;

/// The audit action of an applied cleanup.
const CLEANED_AUDIT_ACTION: &str = "staging.load_population_cleaned";
/// The audit target type of the population's audit rows: the whole reserved range.
pub(super) const POPULATION_AUDIT_TARGET_TYPE: &str = "staging_load_population";

/// The rows a cleanup deletes, counted before it deletes them.
const CENSUS_QUERY: &str = "SELECT
    (SELECT count(*) FROM users WHERE discord_id ~ $1) AS accounts,
    (SELECT count(*) FROM authentication_sessions WHERE discord_id ~ $1) AS sessions,
    (SELECT count(*) FROM refresh_tokens WHERE discord_id ~ $1) AS refresh_tokens,
    (SELECT count(*) FROM discord_membership_snapshots WHERE discord_id ~ $1)
        AS membership_snapshots,
    (SELECT count(*) FROM event_registrations WHERE discord_id ~ $1) AS registrations,
    (SELECT count(*) FROM event_registration_history WHERE registration_id IN
        (SELECT id FROM event_registrations WHERE discord_id ~ $1)) AS registration_history,
    (SELECT count(*) FROM event_registration_participation WHERE registration_id IN
        (SELECT id FROM event_registrations WHERE discord_id ~ $1)) AS participation_records,
    (SELECT count(*) FROM mission_bookmarks WHERE discord_id ~ $1) AS bookmarks,
    (SELECT count(*) FROM fire_missions WHERE created_by ~ $1) AS fire_missions,
    (SELECT count(*) FROM identity_link_codes WHERE discord_id ~ $1) AS link_codes";

/// The census of a population's rows.
#[derive(Debug, sqlx::FromRow)]
struct PopulationCensus {
    accounts: i64,
    sessions: i64,
    refresh_tokens: i64,
    membership_snapshots: i64,
    registrations: i64,
    registration_history: i64,
    participation_records: i64,
    bookmarks: i64,
    fire_missions: i64,
    link_codes: i64,
}

impl PopulationCensus {
    /// Whether nothing is left to delete: every other counted row hangs off an account or its
    /// registrations, except the fire missions and link codes, which can outlive one.
    fn is_empty(&self) -> bool {
        self.accounts == 0 && self.fire_missions == 0 && self.link_codes == 0
    }

    fn summary(&self) -> String {
        format!(
            "accounts={} sessions={} refresh_tokens={} membership_snapshots={} registrations={} \
             registration_history={} participation_records={} bookmarks={} fire_missions={} \
             link_codes={}",
            self.accounts,
            self.sessions,
            self.refresh_tokens,
            self.membership_snapshots,
            self.registrations,
            self.registration_history,
            self.participation_records,
            self.bookmarks,
            self.fire_missions,
            self.link_codes
        )
    }
}

/// The audit target id of the population's audit rows: the reserved range, first to last.
pub(super) fn reserved_range_audit_target() -> String {
    format!("{FIRST_RESERVED_DISCORD_ID}-{LAST_RESERVED_DISCORD_ID}")
}

/// Parse `clean-load-population`, which takes no flags of its own.
pub(crate) fn parse(_arguments: &mut ArgumentList) -> Result<ParsedSubcommand, ToolFailure> {
    Ok(Box::new(|context| Box::pin(clean(context))))
}

async fn clean(context: GuardedContext) -> Result<(), ToolFailure> {
    let mut transaction = context.pool.begin().await?;
    let census: PopulationCensus = sqlx::query_as(CENSUS_QUERY)
        .bind(RESERVED_DISCORD_ID_PATTERN)
        .fetch_one(&mut *transaction)
        .await?;
    println!("clean plan: {}", census.summary());
    if census.is_empty() {
        println!("nothing to clean: no row of the reserved range exists");
        return Ok(());
    }
    if !context.mode.writes() {
        return Ok(());
    }
    let deleted = delete_synthetic_accounts(&mut transaction, None).await?;
    append_system_audit(
        &mut transaction,
        CLEANED_AUDIT_ACTION,
        POPULATION_AUDIT_TARGET_TYPE,
        &reserved_range_audit_target(),
        &format!(
            "deleted the synthetic load population: {} accounts with their sessions and \
             bookmarks, {} registrations with their history, {} fire missions and {} link codes",
            deleted.accounts, deleted.registrations, deleted.fire_missions, deleted.link_codes
        ),
    )
    .await?;
    transaction.commit().await?;
    println!(
        "cleaned: accounts={} registrations={} fire_missions={} link_codes={}",
        deleted.accounts, deleted.registrations, deleted.fire_missions, deleted.link_codes
    );
    Ok(())
}

/// The rows [`delete_synthetic_accounts`] deleted itself; the rest went with the accounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DeletedRows {
    /// Accounts deleted.
    pub(super) accounts: u64,
    /// Registrations the accounts held, deleted with their history and participation rows.
    pub(super) registrations: u64,
    /// Fire missions the accounts created.
    pub(super) fire_missions: u64,
    /// Link codes the accounts requested.
    pub(super) link_codes: u64,
}

/// Each deletion in its order, with the column that names the account; `{scope}` becomes that
/// column's [`account_scope`].
const DELETIONS: [(&str, &str); 6] = [
    (
        "DELETE FROM event_registration_history WHERE registration_id IN \
         (SELECT id FROM event_registrations WHERE {scope})",
        "discord_id",
    ),
    (
        "DELETE FROM event_registration_participation WHERE registration_id IN \
         (SELECT id FROM event_registrations WHERE {scope})",
        "discord_id",
    ),
    (
        "DELETE FROM event_registrations WHERE {scope}",
        "discord_id",
    ),
    ("DELETE FROM fire_missions WHERE {scope}", "created_by"),
    (
        "DELETE FROM identity_link_codes WHERE {scope}",
        "discord_id",
    ),
    ("DELETE FROM users WHERE {scope}", "discord_id"),
];

/// Delete the registrations of reserved-range accounts with their history and participation rows,
/// their fire missions and link codes, then the accounts, whose other rows go with them. `only`
/// narrows every deletion to those ids, which still count only inside the reserved range.
pub(super) async fn delete_synthetic_accounts(
    connection: &mut PgConnection,
    only: Option<&[String]>,
) -> sqlx::Result<DeletedRows> {
    let mut affected = [0_u64; DELETIONS.len()];
    for ((statement, column), count) in DELETIONS.iter().zip(affected.iter_mut()) {
        // AssertSqlSafe: the statements and columns are the constants above; the ids bind.
        let sql = statement.replace("{scope}", &account_scope(column));
        *count = sqlx::query(AssertSqlSafe(sql))
            .bind(RESERVED_DISCORD_ID_PATTERN)
            .bind(only)
            .execute(&mut *connection)
            .await?
            .rows_affected();
    }
    let [_, _, registrations, fire_missions, link_codes, accounts] = affected;
    Ok(DeletedRows {
        accounts,
        registrations,
        fire_missions,
        link_codes,
    })
}

/// The reserved-range accounts a deletion covers by `column`: all of them, or only those of `$2`
/// when it is not null.
fn account_scope(column: &str) -> String {
    format!("{column} ~ $1 AND ($2::text[] IS NULL OR {column} = ANY($2))")
}
