//! `staging-fixtures`: the host tool that stages the fixtures of a staging verification run in
//! the staging database and on the staging host.
//!
//! ```text
//! staging-fixtures <subcommand> [subcommand flags] --confirm-database <name>
//!                  [--apply] [--api-env-file <path>]
//! staging-fixtures --help
//! ```
//!
//! **Role:** the subcommand table and the guards every subcommand passes before it runs.
//!
//! **Position:** run on the staging host from the checkout root; reads the API env file
//! (`apps/api/.env` unless `--api-env-file` names another), connects to the database its
//! `DATABASE_URL` names, and hands a [`GuardedContext`] to the parsed subcommand.
//!
//! **Signals & state:** none of its own; the context owns the pool for the run.
//!
//! **Invariants:** a subcommand's flags parse, and every flag is taken, before anything connects;
//! a run proceeds only when `--confirm-database` equals the connected `current_database()`; a run
//! without `--apply` writes nothing; exit 0 is success (a dry run included), 1 a failure whose
//! writes were undone, 2 a refusal with nothing written; no secret is printed, and every failure
//! message names paths, ids and flags only.

mod argument_list;
mod credential_rotation;
mod discord_member_probe;
mod fleet_provisioning;
mod guarded_context;
mod load_fixture_events;
mod load_population;
mod membership_aging;
mod reserved_accounts;
mod secret_files;
mod tool_failure;

use std::path::PathBuf;
use std::process::ExitCode;

use api::core::database;
use tracing_subscriber::EnvFilter;

use crate::argument_list::ArgumentList;
use crate::guarded_context::{ApiEnvironment, GuardedContext, ParsedSubcommand, RunMode};
use crate::reserved_accounts::count_reserved_accounts;
use crate::tool_failure::ToolFailure;

/// The API env file of a checkout, relative to its root: the file the API unit loads.
const DEFAULT_API_ENV_FILE: &str = "apps/api/.env";

/// One row of the subcommand table.
struct SubcommandEntry {
    /// The name on the command line.
    name: &'static str,
    /// The subcommand's own flags, besides the common ones.
    synopsis: &'static str,
    /// What an applied run does.
    summary: &'static str,
    /// Takes the subcommand's flags and returns the run, before any guard runs.
    parse: fn(&mut ArgumentList) -> Result<ParsedSubcommand, ToolFailure>,
}

/// Every subcommand of the tool.
const SUBCOMMANDS: &[SubcommandEntry] = &[
    SubcommandEntry {
        name: "provision-fleet",
        synopsis: "--instances <n> --actor <discord id> --ip <address> --secrets-root <path> \
                   [--game-port-base <port>]",
        summary: "registers the new servers \"TBD Staging 1\" to \"TBD Staging <n>\" on game ports \
                  base+1 to base+n and writes each one's host agent and mod runtime credentials \
                  into <secrets root>/instance-<n>/secrets/, mode 600",
        parse: fleet_provisioning::parse,
    },
    SubcommandEntry {
        name: "rotate-credential",
        synopsis: "--instance <n> --executor host_agent|mod_runtime --secrets-root <path> \
                   (--stage --actor <discord id> | --promote)",
        summary: "--stage issues a new credential into <credential file>.staged; --promote moves \
                  the staged file over the live one once the old credential is revoked",
        parse: credential_rotation::parse,
    },
    SubcommandEntry {
        name: "seed-load-fixture-events",
        synopsis: "--mission <uuid>",
        summary: "creates the open events \"[Load fixture] 01\" to \"[Load fixture] 10\", starting \
                  14 days ahead and authored by the load population's first account, each \
                  attaching <mission> with a 2 x 8 x 8 ORBAT of 128 slots; refuses while \
                  DISCORD_BOT_TOKEN is set or fixture events exist",
        parse: load_fixture_events::parse_seed,
    },
    SubcommandEntry {
        name: "clean-load-fixture-events",
        synopsis: "",
        summary: "deletes the \"[Load fixture]\" events of reserved authors with their \
                  attachments, slots, registrations and registration history; audit rows stay",
        parse: load_fixture_events::parse_clean,
    },
    SubcommandEntry {
        name: "seed-load-population",
        synopsis: "--accounts <n> --role <discord role name> --account-file <path> \
                   [--id-base <discord id>]",
        summary: "creates <n> synthetic accounts from the id base (the first reserved id by \
                  default) as verified members of the DISCORD_GUILD_ID guild holding the named \
                  Discord role, each with a refresh-only session, and writes their refresh tokens \
                  to <path>, mode 600, in the load engine's account file format; refuses while \
                  DISCORD_BOT_TOKEN is set or any reserved account exists",
        parse: load_population::parse_seed,
    },
    SubcommandEntry {
        name: "clean-load-population",
        synopsis: "",
        summary: "deletes every account of the reserved range with its sessions, tokens, \
                  membership rows, registrations, bookmarks, fire missions and link codes; audit \
                  rows stay",
        parse: load_population::parse_clean,
    },
    SubcommandEntry {
        name: "age-membership-snapshot",
        synopsis: "--discord-id <id> --hours <h>",
        summary: "stages the account's DISCORD_GUILD_ID membership snapshot as verified <h> hours \
                  ago (1 to 168), with a staging.membership_snapshot_aged audit row",
        parse: membership_aging::parse,
    },
    SubcommandEntry {
        name: "observe-discord-member",
        synopsis: "--discord-id <id> --guild main|partner [--partner-guild-id <id>] \
                   [--discord-api-base <loopback url>]",
        summary: "reads the member once with the bot token of the API env file and prints a \
                  discord-member-read line: the outcome, the role ids and the rate-limit headers",
        parse: discord_member_probe::parse_observe,
    },
    SubcommandEntry {
        name: "spend-discord-member-bucket",
        synopsis: "--discord-id <id> --guild main|partner [--partner-guild-id <id>] \
                   --start-at-unix-ms <ms> --hold-seconds <s> --max-requests <n> \
                   [--discord-api-base <loopback url>]",
        summary: "from the start time, reads the member until the Get Guild Member bucket is spent \
                  and keeps it spent for the hold (1 to 10 s), at most <n> requests (1 to 50), \
                  printing a discord-member-read line per request and a discord-bucket-spend \
                  summary",
        parse: discord_member_probe::parse_spend,
    },
];

/// The flags every subcommand takes.
struct CommonOptions {
    mode: RunMode,
    confirm_database: String,
    api_env_file: PathBuf,
}

impl CommonOptions {
    fn take(arguments: &mut ArgumentList) -> Result<Self, ToolFailure> {
        Ok(Self {
            mode: RunMode::from_apply_switch(arguments.switch("--apply")?),
            confirm_database: arguments.required("--confirm-database")?,
            api_env_file: arguments
                .optional("--api-env-file")?
                .map_or_else(|| PathBuf::from(DEFAULT_API_ENV_FILE), PathBuf::from),
        })
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    // Services log the cause of a database failure through `tracing`; stderr carries it.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();
    let tokens: Vec<String> = std::env::args().skip(1).collect();
    match run(&tokens).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("staging-fixtures: {failure}");
            failure.exit_code()
        }
    }
}

async fn run(tokens: &[String]) -> Result<(), ToolFailure> {
    let Some((name, rest)) = tokens.split_first() else {
        eprint!("{}", usage());
        return Err(ToolFailure::refused("name a subcommand"));
    };
    if name == "--help" || name == "help" {
        print!("{}", usage());
        return Ok(());
    }
    let entry = SUBCOMMANDS
        .iter()
        .find(|entry| entry.name == name)
        .ok_or_else(|| {
            ToolFailure::refused(format!(
                "unknown subcommand `{name}`; staging-fixtures --help lists them"
            ))
        })?;
    if rest.iter().any(|token| token == "--help") {
        print!("{}", entry_usage(entry));
        return Ok(());
    }
    let mut arguments = ArgumentList::parse(rest)?;
    let common = CommonOptions::take(&mut arguments)?;
    let subcommand = (entry.parse)(&mut arguments)?;
    arguments.finish()?;
    let context = guarded_context(common).await?;
    let reserved = count_reserved_accounts(&context.pool).await?;
    let mode = context.mode;
    println!(
        "staging-fixtures {}: database {} confirmed; {}; reserved synthetic accounts present: \
         {reserved}",
        entry.name,
        context.database,
        match mode {
            RunMode::DryRun => "dry run",
            RunMode::Apply => "applying",
        }
    );
    subcommand(context).await?;
    if mode == RunMode::DryRun {
        println!("dry run: nothing written; pass --apply to write");
    }
    Ok(())
}

/// The guards: read the API env file, connect to its `DATABASE_URL`, and refuse unless the
/// connected database is the one `--confirm-database` names.
async fn guarded_context(common: CommonOptions) -> Result<GuardedContext, ToolFailure> {
    let api_environment = ApiEnvironment::read(&common.api_env_file)?;
    let url = api_environment.non_empty("DATABASE_URL").ok_or_else(|| {
        ToolFailure::refused(format!(
            "DATABASE_URL is not set in {}",
            api_environment.path().display()
        ))
    })?;
    let pool = database::connect(url).await.map_err(|error| {
        ToolFailure::failed(format!(
            "cannot connect to the database DATABASE_URL in {} names: {error}",
            api_environment.path().display()
        ))
    })?;
    let database: String = sqlx::query_scalar("SELECT current_database()::text")
        .fetch_one(&pool)
        .await?;
    if database != common.confirm_database {
        return Err(ToolFailure::refused(format!(
            "--confirm-database {} does not match the connected database {database}",
            common.confirm_database
        )));
    }
    Ok(GuardedContext {
        pool,
        database,
        mode: common.mode,
        api_environment,
    })
}

fn usage() -> String {
    let mut text = String::from(
        "usage: staging-fixtures <subcommand> [flags] --confirm-database <name> [--apply] \
         [--api-env-file <path>]\n\n\
         Every subcommand is a dry run unless --apply is given. --confirm-database must equal the \
         current_database() of the DATABASE_URL in the API env file (default \
         apps/api/.env). Exit codes: 0 done, 1 failed with its writes undone, 2 \
         refused with nothing written.\n\nsubcommands:\n",
    );
    for entry in SUBCOMMANDS {
        text.push_str(&entry_usage(entry));
    }
    text
}

fn entry_usage(entry: &SubcommandEntry) -> String {
    format!(
        "  {} {}\n      {}\n",
        entry.name, entry.synopsis, entry.summary
    )
}
