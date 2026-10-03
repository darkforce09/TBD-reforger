//! The identities a staging receipt records as `environment:` lines: the host, what it runs, and
//! (for the load check) the load generator.
//!
//! **Role:** declares the three identity modules and [`collect`], which reads them all once per
//! recorded run, journals each raw answer, and returns the environment entries.
//!
//! **Position:** called by `procedure_runner/recording.rs` before the first step.
//!
//! **Signals & state:** none held; reads through the run's [`HostCommandRunner`] and journal.
//!
//! **Invariants:** only reads; an identity that cannot be read is recorded as
//! `unavailable (<why>)`, never omitted or guessed; no entry names a secret (the recorder refuses
//! one).

pub(crate) mod build_identity;
pub(crate) mod load_generator_identity;
pub(crate) mod staging_host_identity;

use crate::error::Result;

use super::observation_journal::journal::{JournalEntry, ObservationJournal};
use super::remote_actions::game_server_update;
use super::remote_observers::remote_command::{HostCommandRunner, RemoteCommand};
use super::remote_observers::{console_log_reader, database_reader, metrics_reader};
use super::staging_settings::StagingSettings;
use api_readiness_checks::operational_recording::{EnvironmentEntry, StagingCheck};

/// Reads every identity of `check`'s environment; errors only when the journal cannot be
/// written or an entry is malformed.
pub(crate) fn collect(
    settings: &StagingSettings,
    host: &mut dyn HostCommandRunner,
    journal: &mut ObservationJournal,
    observed_unix_ms: u64,
    check: StagingCheck,
) -> Result<Vec<EnvironmentEntry>> {
    let mut values: Vec<(String, String)> = Vec::new();
    let mut read = |name: &str, command: Result<RemoteCommand>| -> Result<Option<String>> {
        let answer = command.and_then(|command| {
            let output = host.run(&command)?;
            let sha256 = journal.archive(&JournalEntry {
                step: &format!("identity.{name}"),
                observer: command.observer,
                observed_unix_ms,
                summary: &format!("exit {}", output.exit_code),
                verdict: "identity",
                artifact: output.stdout.as_bytes(),
            })?;
            crate::error::ensure!(
                output.exit_code == 0,
                "{} exited {} (artifact {sha256})",
                command.observer,
                output.exit_code
            );
            Ok(output.stdout)
        });
        Ok(match answer {
            Ok(text) => Some(text),
            Err(error) => {
                values.push((name.to_string(), format!("unavailable ({error:#})")));
                None
            }
        })
    };
    let mut found: Vec<(String, String)> = Vec::new();
    if let Some(text) = read("staging_host", Ok(staging_host_identity::command()))? {
        found.extend(staging_host_identity::entries(&text));
    }
    if let Some(text) = read("host_files", Ok(build_identity::host_files(settings)))? {
        found.extend(build_identity::key_values(&text));
    }
    let schema = database_reader::select(
        &settings.database_container,
        &database_reader::SCHEMA_IDENTITY,
        &[],
    );
    if let Some(text) = read("schema", schema)? {
        let row = database_reader::rows(&text)
            .into_iter()
            .next()
            .unwrap_or_default();
        found.push((
            "migration_head".into(),
            row.first().cloned().unwrap_or_default(),
        ));
        found.push((
            "postgres_version".into(),
            row.get(1).cloned().unwrap_or_default(),
        ));
    }
    let metrics = metrics_reader::exposition(&settings.api_env_file(), &settings.api_origin);
    if let Some(text) = read("api_build", Ok(metrics))? {
        found.push((
            "api_build_version".into(),
            metrics_reader::build_version(&text).unwrap_or_default(),
        ));
    }
    if let Some(text) = read(
        "game_server_build",
        Ok(game_server_update::installed_build(settings)),
    )? {
        found.push((
            "game_server_build".into(),
            game_server_update::build_id(&text)
                .unwrap_or_default()
                .to_string(),
        ));
    }
    let console = console_log_reader::newest(&settings.fleet_root(), 1);
    if let Some(text) = read("workshop_version", Ok(console))? {
        found.push((
            "workshop_version".into(),
            console_log_reader::parse(&text)
                .and_then(|log| build_identity::workshop_version(&log.text))
                .unwrap_or_default(),
        ));
    }
    if let Some(partner) = &settings.partner_guild_id {
        found.push(("partner_guild_id".into(), partner.clone()));
    }
    if check == StagingCheck::Load {
        found.push((
            "load_generator_hardware".into(),
            load_generator_identity::hardware(),
        ));
        found.push((
            "load_generator_network".into(),
            load_generator_identity::network(settings),
        ));
    }
    for (key, value) in found {
        let value = if value.trim().is_empty() {
            "unavailable (the answer held no value)".to_string()
        } else {
            value
        };
        values.push((key, value));
    }
    values
        .iter()
        .map(|(key, value)| Ok(EnvironmentEntry::new(key, value)?))
        .collect()
}
