//! `staging seed-load` and `staging clean-load`: the synthetic population and the fixture events
//! on the host, and the token file on the workstation.
//!
//! **Role:** resolves the live staging mission, seeds the population and then the fixture events
//! through `staging-fixtures`, moves the account file the host tool wrote into the operator's
//! token file (created exclusively, mode 600) and removes the host copy; and cleans the fixture
//! events, then the population, then any host copy left behind.
//!
//! **Position:** called by `staging_dispatch.rs` for the two confirmed actions; the commands come from
//! `remote_actions/host_fixture_commands.rs`, the reads from `load_queries.rs`.
//!
//! **Signals & state:** none held; the tokens pass through memory once, from the host read to
//! the token file.
//!
//! **Invariants:** seeding runs the population before the fixture events and cleaning the
//! reverse, the order the foreign keys fix; a seeding refuses an existing token file and a
//! mission title that is not exactly one live mission before it changes anything; the account
//! file's content is never printed, journaled or passed as an argument; `--dry-run` prints the
//! plan and opens no connection.

use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use crate::error::{Result, ResultExt, bail, ensure};
use serde::Deserialize;

use super::committed_load_data::CommittedLoadData;
use super::load_queries::{STAGING_MISSION, mission_ids};
use crate::remote_actions::host_fixture_commands::{LoadSeeding, load_fixtures};
use crate::remote_observers::database_reader::select;
use crate::remote_observers::remote_command::{HostCommandRunner, RemoteCommand, shell_quote};
use crate::staging_command::PlanOnly;
use crate::staging_dispatch::{ConfirmedAnswer, confirmed, confirmed_answer};
use crate::staging_settings::StagingSettings;

/// Where the host tool writes the account file, under the host user's home.
pub(crate) const HOST_ACCOUNT_FILE: &str = "tbd/load/load-accounts.json";

/// The host account file's path.
pub(crate) fn host_account_file(settings: &StagingSettings) -> String {
    format!("{}/{HOST_ACCOUNT_FILE}", settings.home)
}

/// The removal of any host copy of the account file.
pub(crate) fn remove_host_account_file(settings: &StagingSettings) -> RemoteCommand {
    RemoteCommand::change(
        "host account file",
        format!("rm -f -- {}", shell_quote(&host_account_file(settings))),
    )
}

/// `staging seed-load --token-file <path>`.
pub(crate) fn seed_load(
    settings: &StagingSettings,
    data: &CommittedLoadData,
    token_file: &Path,
    plan: &PlanOnly,
    host: &mut dyn HostCommandRunner,
    output: &mut dyn Write,
) -> Result<u8> {
    let population = &data.population;
    let title = population.fixture_events.mission_title.clone();
    let mission_read = select(
        &settings.database_container,
        &STAGING_MISSION,
        &[("title", title.clone())],
    )?;
    let account_file = host_account_file(settings);
    let id_base = data.id_base()?.to_string();
    let seeding = |mission_id: &str| -> Vec<RemoteCommand> {
        load_fixtures(
            settings,
            Some(&LoadSeeding {
                accounts: population.accounts,
                id_base: &id_base,
                discord_role: &population.discord_role,
                account_file: &account_file,
                mission_id,
            }),
        )
    };
    if plan.dry_run {
        writeln!(
            output,
            "[dry-run] {}: {} (the live mission titled {title:?})",
            mission_read.observer, mission_read.command_line
        )?;
        for command in seeding("<id of that mission>") {
            confirmed_answer(&command, plan, host, output)?;
        }
        writeln!(
            output,
            "[dry-run] then: read {account_file} into {} (created exclusively, mode 600) and remove the host copy",
            token_file.display()
        )?;
        return Ok(0);
    }
    ensure!(
        !token_file.exists(),
        "the token file {} already exists; a seeding writes a new one",
        token_file.display()
    );
    let answer = host.run(&mission_read)?;
    ensure!(
        answer.exit_code == 0,
        "reading the staging mission exited {}: {}",
        answer.exit_code,
        answer.stderr.trim()
    );
    let missions = mission_ids(&answer.stdout);
    let [mission] = missions.as_slice() else {
        bail!(
            "{} live missions are titled {title:?}; the fixture events attach exactly one",
            missions.len()
        );
    };
    writeln!(output, "staging mission {title:?}: {mission}")?;
    for command in seeding(mission) {
        if let ConfirmedAnswer::Failed = confirmed_answer(&command, plan, host, output)? {
            return Ok(1);
        }
    }
    let read = RemoteCommand::read(
        "host account file",
        format!("cat -- {}", shell_quote(&account_file)),
    );
    let answer = host.run(&read)?;
    ensure!(
        answer.exit_code == 0,
        "reading {account_file} exited {}",
        answer.exit_code
    );
    let accounts = account_count(&answer.stdout)?;
    ensure!(
        accounts == u64::from(population.accounts),
        "{account_file} holds {accounts} accounts; the population has {}",
        population.accounts
    );
    write_token_file(token_file, answer.stdout.as_bytes())?;
    writeln!(
        output,
        "token file: {} ({accounts} accounts, mode 600)",
        token_file.display()
    )?;
    match confirmed_answer(&remove_host_account_file(settings), plan, host, output)? {
        ConfirmedAnswer::Failed => Ok(1),
        _ => Ok(0),
    }
}

/// `staging clean-load`: the fixture events, then the population, then any host account file.
pub(crate) fn clean_load(
    settings: &StagingSettings,
    plan: &PlanOnly,
    host: &mut dyn HostCommandRunner,
    output: &mut dyn Write,
) -> Result<u8> {
    let mut commands = load_fixtures(settings, None);
    commands.push(remove_host_account_file(settings));
    confirmed(&commands, plan, host, output)
}

#[derive(Deserialize)]
struct AccountCount {
    accounts: Vec<serde::de::IgnoredAny>,
}

/// The number of accounts in an account file's text; the tokens are not kept.
fn account_count(text: &str) -> Result<u64> {
    let document: AccountCount =
        serde_json::from_str(text).context("the host account file is not an account file")?;
    Ok(document.accounts.len() as u64)
}

/// Creates `path` exclusively with mode 600 and writes `bytes` into it.
pub(crate) fn write_token_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("creating the token file {}", path.display()))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .with_context(|| format!("writing the token file {}", path.display()))
}
