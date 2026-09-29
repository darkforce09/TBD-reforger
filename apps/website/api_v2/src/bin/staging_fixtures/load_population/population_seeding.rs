//! `seed-load-population`: creates the synthetic member accounts of a load run and writes their
//! refresh tokens to the account file.
//!
//! **Role:** turns `--accounts`, `--id-base`, `--role` and `--account-file` into that many
//! reserved-range accounts that are verified members of the main guild holding the named Discord
//! role, each with a refresh-only session, exactly as a real member who signed in would be.
//!
//! **Position:** a row of the subcommand table in `main.rs`. Each account goes through the
//! services the API's own sign-in path uses:
//! [`register_account`], [`claim_membership_refresh`] and [`accept_membership_observation`] with
//! the Discord role as the observed member roles, then [`issue_refresh`]; `account_file` writes the
//! tokens, and `population_cleanup` undoes a failed run.
//!
//! **Signals & state:** the refresh tokens of the accounts created so far, in memory until the
//! account file holds them.
//!
//! **Invariants:** every run refuses, writing nothing, while `DISCORD_BOT_TOKEN` is set in the API
//! env file (the reconciler would ask Discord about every synthetic account and demote it to
//! guest), while `DISCORD_GUILD_ID` is unset, while any account of the reserved range exists, when
//! the role does not name exactly one `discord_roles` row granting member authority only (mapped to
//! `enlisted` or to nothing), and when the account file exists or its directory admits another
//! user. The services commit account by account, so a failure deletes every account the run
//! created and the file it wrote: a failed run leaves the database as it found it. The
//! `staging.load_population_seeded` audit row is written last, once the file holds every token.

use sqlx::PgPool;
use website_api::administration::services::required_audit::append_system_audit;
use website_api::identity_and_access::services::account_registration::{
    AccountProfile, register_account,
};
use website_api::identity_and_access::services::discord_client::GuildMember;
use website_api::identity_and_access::services::discord_membership_cache::{
    accept_membership_observation, claim_membership_refresh,
};
use website_api::identity_and_access::services::session_issuance::issue_refresh;

use super::account_file::{AccountFileTarget, SeededAccount};
use super::population_cleanup::{
    POPULATION_AUDIT_TARGET_TYPE, delete_synthetic_accounts, reserved_range_audit_target,
};
use super::{SyntheticIdRange, synthetic_handle, synthetic_username};
use crate::argument_list::ArgumentList;
use crate::guarded_context::{GuardedContext, ParsedSubcommand};
use crate::reserved_accounts::{FIRST_RESERVED_DISCORD_ID, count_reserved_accounts};
use crate::tool_failure::ToolFailure;

/// The audit action of an applied seeding.
const SEEDED_AUDIT_ACTION: &str = "staging.load_population_seeded";
/// The only site role a synthetic account may hold through its Discord role.
const MEMBER_ROLE: &str = "enlisted";

/// What a seeding creates.
struct SeedPlan {
    range: SyntheticIdRange,
    role_name: String,
    account_file: AccountFileTarget,
}

/// The Discord role every synthetic account holds.
struct PopulationRole {
    discord_role_id: String,
    mapped_role: Option<String>,
}

/// Parse `--accounts <n> --role <discord role name> --account-file <path> [--id-base <id>]`.
pub(crate) fn parse(arguments: &mut ArgumentList) -> Result<ParsedSubcommand, ToolFailure> {
    let count = arguments.required_parsed::<u32>("--accounts", "a whole number")?;
    let first = arguments
        .optional_parsed::<u64>("--id-base", "a Discord id")?
        .unwrap_or(FIRST_RESERVED_DISCORD_ID);
    let range = SyntheticIdRange::new(first, count)?;
    let role_name = arguments.required("--role")?;
    if role_name.trim().is_empty() {
        return Err(ToolFailure::refused("--role names a Discord role"));
    }
    let account_file = AccountFileTarget::new(arguments.required("--account-file")?)?;
    let plan = SeedPlan {
        range,
        role_name,
        account_file,
    };
    Ok(Box::new(move |context| Box::pin(seed(context, plan))))
}

async fn seed(context: GuardedContext, plan: SeedPlan) -> Result<(), ToolFailure> {
    let environment = &context.api_environment;
    if environment.non_empty("DISCORD_BOT_TOKEN").is_some() {
        return Err(ToolFailure::refused(format!(
            "DISCORD_BOT_TOKEN is set in {}; a load population is seeded only while the bot token \
             is unset, because the reconciler would ask Discord about every synthetic account and \
             demote it to guest",
            environment.path().display()
        )));
    }
    let guild = environment
        .non_empty("DISCORD_GUILD_ID")
        .ok_or_else(|| {
            ToolFailure::refused(format!(
                "DISCORD_GUILD_ID is not set in {}; the population's membership is verified in \
                 the main guild",
                environment.path().display()
            ))
        })?
        .to_owned();
    let present = count_reserved_accounts(&context.pool).await?;
    if present > 0 {
        return Err(ToolFailure::refused(format!(
            "{present} accounts of the reserved range exist; run clean-load-population first"
        )));
    }
    let role = resolve_population_role(&context.pool, &plan.role_name).await?;
    plan.account_file.check()?;
    println!(
        "seed plan: accounts={} first={} last={} guild={guild} role=\"{}\" discord_role_id={} \
         mapped_role={} account_file={}",
        plan.range.count(),
        plan.range.first(),
        plan.range.last(),
        plan.role_name,
        role.discord_role_id,
        role.mapped_role.as_deref().unwrap_or("none"),
        plan.account_file.path().display()
    );
    if !context.mode.writes() {
        return Ok(());
    }
    let mut seeded: Vec<SeededAccount> = Vec::with_capacity(plan.range.count() as usize);
    let mut file_written = false;
    let outcome = async {
        seed_accounts(&context.pool, &plan, &guild, &role, &mut seeded).await?;
        plan.account_file.write(&seeded)?;
        file_written = true;
        record_seeding(&context.pool, &plan).await
    }
    .await;
    if let Err(failure) = outcome {
        let progress = SeedingProgress {
            accounts_finished: seeded.len(),
            file_written,
        };
        return Err(undo_seeding(&context.pool, &plan, progress, failure).await);
    }
    println!(
        "seeded: accounts={} first={} last={} account_file={} (mode 600)",
        seeded.len(),
        plan.range.first(),
        plan.range.last(),
        plan.account_file.path().display()
    );
    Ok(())
}

/// The `discord_roles` row named `name`, refused unless exactly one exists and it grants member
/// authority only.
async fn resolve_population_role(pool: &PgPool, name: &str) -> Result<PopulationRole, ToolFailure> {
    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT discord_role_id, mapped_role::text FROM discord_roles WHERE name = $1
         ORDER BY discord_role_id COLLATE \"C\"",
    )
    .bind(name)
    .fetch_all(pool)
    .await?;
    let [(discord_role_id, mapped_role)] = rows.as_slice() else {
        return Err(ToolFailure::refused(format!(
            "{} Discord roles are named \"{name}\" in discord_roles; --role names exactly one",
            rows.len()
        )));
    };
    if let Some(mapped) = mapped_role.as_deref()
        && mapped != MEMBER_ROLE
    {
        return Err(ToolFailure::refused(format!(
            "the Discord role \"{name}\" maps to the site role {mapped}; synthetic accounts hold \
             member authority only (a role mapped to {MEMBER_ROLE} or to nothing)"
        )));
    }
    Ok(PopulationRole {
        discord_role_id: discord_role_id.clone(),
        mapped_role: mapped_role.clone(),
    })
}

/// Register, verify and issue a refresh session for every account of the range, in order,
/// appending each finished account to `seeded`.
async fn seed_accounts(
    pool: &PgPool,
    plan: &SeedPlan,
    guild: &str,
    role: &PopulationRole,
    seeded: &mut Vec<SeededAccount>,
) -> Result<(), ToolFailure> {
    for index in 0..plan.range.count() {
        let discord_id = plan.range.discord_id(index);
        let (username, handle) = (synthetic_username(index), synthetic_handle(index));
        let profile = AccountProfile {
            discord_id: &discord_id,
            username: &username,
            discord_handle: &handle,
            avatar_url: "",
        };
        register_account(pool, &profile).await?;
        let lease = claim_membership_refresh(pool, &discord_id, guild, true)
            .await?
            .ok_or_else(|| {
                ToolFailure::failed(format!(
                    "the membership lease of {discord_id} was not granted"
                ))
            })?;
        let member = GuildMember {
            nick: String::new(),
            roles: vec![role.discord_role_id.clone()],
        };
        if !accept_membership_observation(pool, &lease, Some(&member), guild).await? {
            return Err(ToolFailure::failed(format!(
                "the membership observation of {discord_id} lost its lease"
            )));
        }
        let refresh_token = issue_refresh(pool, &discord_id).await.map_err(|error| {
            ToolFailure::failed(format!(
                "issuing the refresh session of {discord_id}: {}",
                error.message
            ))
        })?;
        seeded.push(SeededAccount {
            discord_id,
            refresh_token,
        });
    }
    Ok(())
}

/// Append the seeding's audit row.
async fn record_seeding(pool: &PgPool, plan: &SeedPlan) -> Result<(), ToolFailure> {
    let mut connection = pool.acquire().await?;
    append_system_audit(
        &mut connection,
        SEEDED_AUDIT_ACTION,
        POPULATION_AUDIT_TARGET_TYPE,
        &reserved_range_audit_target(),
        &format!(
            "seeded the synthetic load population: {} verified member accounts {} to {} holding \
             the Discord role \"{}\", each with a refresh-only session",
            plan.range.count(),
            plan.range.first(),
            plan.range.last(),
            plan.role_name
        ),
    )
    .await?;
    Ok(())
}

/// How far a failed seeding got.
struct SeedingProgress {
    /// Accounts that hold their refresh session.
    accounts_finished: usize,
    /// Whether this run wrote the account file; a file the run did not write is never removed.
    file_written: bool,
}

/// Delete the accounts a failed run created and the account file it wrote, and return the
/// failure with what remains named.
async fn undo_seeding(
    pool: &PgPool,
    plan: &SeedPlan,
    progress: SeedingProgress,
    failure: ToolFailure,
) -> ToolFailure {
    let reason = match &failure {
        ToolFailure::Refused(reason) | ToolFailure::Failed(reason) => reason.clone(),
    };
    let file_note = if !progress.file_written {
        ""
    } else if std::fs::remove_file(plan.account_file.path()).is_ok() {
        "; the account file it wrote was removed"
    } else {
        "; the account file it wrote could not be removed"
    };
    // The run began with no reserved account, so every reserved id of the range is one it
    // created, including an account whose seeding stopped halfway.
    let created: Vec<String> = (0..plan.range.count())
        .map(|index| plan.range.discord_id(index))
        .collect();
    let undone = async {
        let mut transaction = pool.begin().await?;
        let deleted = delete_synthetic_accounts(&mut transaction, Some(&created)).await?;
        transaction.commit().await?;
        Ok::<_, sqlx::Error>(deleted.accounts)
    }
    .await;
    let accounts_note = match undone {
        Ok(accounts) => format!("; the {accounts} accounts it created were deleted"),
        Err(error) => format!(
            "; deleting the accounts it created failed ({error}); run clean-load-population \
             --apply"
        ),
    };
    ToolFailure::failed(format!(
        "{reason} (after {} of {} accounts){accounts_note}{file_note}",
        progress.accounts_finished,
        plan.range.count()
    ))
}
