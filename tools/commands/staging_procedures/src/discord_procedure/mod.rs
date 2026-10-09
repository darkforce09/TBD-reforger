//! The Discord procedure (`staging_discord`): partner membership, role loss, the outage and its
//! grace, the administrator override, recovery and the rate limit, observed against the real
//! guilds.
//!
//! **Role:** the [`StagingProcedure`] of the Discord check: its action and recovery lists, the
//! preconditions `staging preflight --discord` adds, its steps and declared cases, its fixture
//! identities, the staged precondition its receipt's environment names, and its observations.
//!
//! **Position:** selected by `staging_dispatch.rs` for `staging discord --record`,
//! `staging action-list discord` and `staging preflight --discord`; runs on
//! `procedure_runner/`; the steps live in `partner_steps.rs`, `outage_steps.rs` and
//! `rate_limit_steps.rs`.
//!
//! **Signals & state:** none; the procedure is a stateless value.
//!
//! **Invariants:** the plan declares thirteen cases, two of them not run for want of a test
//! subject account, so the receipt fails honestly until one exists; a plan whose operator,
//! partner guild or partner role is unset is refused before the recording begins; preflight
//! only reads, and no check prints the bot token.

mod discord_cases;
mod membership_queries;
mod operator_actions;
mod outage_steps;
mod partner_steps;
mod rate_limit_steps;
mod reconciliation_readers;

use crate::error::Result;
use serde_json::{Value, json};

use self::membership_queries::{DiscordTargets, MAIN_SNAPSHOT, snapshot};
use crate::operator_coordination::action_list::PlannedAction;
use crate::procedure_receipts::{EnvironmentEntry, FixtureManifest, Observations, StagingCheck};
use crate::procedure_runner::procedure::{ProcedurePlan, ProcedureRun, StagingProcedure};
use crate::remote_observers::discord_member_reader::{self, GuildScope, member_read};
use crate::remote_observers::remote_command::{HostCommandRunner, RemoteCommand, shell_quote};
use crate::staging_settings::StagingSettings;
use crate::support_commands::preflight::PreflightCheck;

/// The run stops itself here, inside the check's 7,200 s timeout.
const HARD_STOP_SECONDS: u64 = 6_900;
/// Seconds between two polls; each bot read is one Discord request.
const POLL_INTERVAL_SECONDS: u64 = 5;
/// A fresh snapshot was verified at most this long ago (the API's freshness period).
const FRESH_SNAPSHOT_MS: u64 = 60_000;

/// The Discord procedure.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DiscordProcedure;

impl StagingProcedure for DiscordProcedure {
    fn check(&self) -> StagingCheck {
        StagingCheck::Discord
    }

    fn action_list(&self, settings: &StagingSettings) -> Vec<PlannedAction> {
        operator_actions::run_actions(settings)
    }

    fn recovery_action_list(&self, settings: &StagingSettings) -> Vec<PlannedAction> {
        operator_actions::recovery_actions(settings)
    }

    fn preflight_checks(&self, settings: &StagingSettings) -> Vec<PreflightCheck> {
        preflight_checks(settings)
    }

    fn plan(&self, settings: &StagingSettings) -> Result<ProcedurePlan> {
        let targets = DiscordTargets::from_settings(settings)?;
        let mut steps = vec![
            partner_steps::partner_registration(&targets)?,
            partner_steps::partner_role_removal(&targets)?,
        ];
        steps.extend(outage_steps::outage(&targets)?);
        steps.extend(outage_steps::override_steps(&targets)?);
        steps.extend(outage_steps::recovery_steps(&targets)?);
        steps.extend(rate_limit_steps::rate_limit_steps(&targets)?);
        steps.push(rate_limit_steps::partner_event_deletion(&targets)?);
        Ok(ProcedurePlan {
            declared_cases: discord_cases::declared_cases()?,
            steps,
            hard_stop_seconds: HARD_STOP_SECONDS,
            poll_interval_seconds: POLL_INTERVAL_SECONDS,
        })
    }

    fn fixture_identities(
        &self,
        settings: &StagingSettings,
        _host: &mut dyn HostCommandRunner,
    ) -> Result<Value> {
        Ok(json!({
            "operator_discord_id": settings.operator_discord_id,
            "partner_guild_id": settings.partner_guild_id,
            "partner_role_id": settings.partner_role_id,
            "partner_event_title": partner_steps::PARTNER_EVENT_TITLE,
            "staged_preconditions": [format!(
                "the operator's main-guild snapshot is staged {} h old by staging-fixtures \
                 age-membership-snapshot (audit staging.membership_snapshot_aged); not elapsed time",
                outage_steps::STAGED_SNAPSHOT_AGE_HOURS
            )],
            "bucket_spend": {
                "lead_ms": rate_limit_steps::SPEND_LEAD_MS,
                "hold_seconds": rate_limit_steps::BUCKET_HOLD_SECONDS,
                "max_requests": rate_limit_steps::BUCKET_MAX_REQUESTS,
            },
        }))
    }

    fn staged_preconditions(&self) -> Result<Vec<EnvironmentEntry>> {
        Ok(vec![EnvironmentEntry::new(
            "staged_precondition",
            &format!(
                "membership_snapshot_aged_{}h",
                outage_steps::STAGED_SNAPSHOT_AGE_HOURS
            ),
        )?])
    }

    fn observations(&self, run: &ProcedureRun, manifest: &FixtureManifest) -> Observations {
        Observations::Discord {
            scenarios: discord_cases::scenarios(&run.cases),
            fixture_sha256: manifest.sha256().to_string(),
        }
    }
}

/// The Discord preconditions: the bot token key set, the bot reading both guilds with the
/// partner role held, the operator's snapshot fresh, and the pre-Discord backup taken.
fn preflight_checks(settings: &StagingSettings) -> Vec<PreflightCheck> {
    let targets = match DiscordTargets::from_settings(settings) {
        Ok(targets) => targets,
        Err(error) => {
            let why = format!("{error:#}");
            return vec![PreflightCheck::local("discord settings", move || {
                Err(why.clone())
            })];
        }
    };
    let token = RemoteCommand::read(
        "api env",
        format!(
            "if grep -q '^DISCORD_BOT_TOKEN=..*' {}; then echo set; else echo unset; fi",
            shell_quote(&settings.api_env_file())
        ),
    );
    let role = targets.partner_role.clone();
    let mut checks = vec![
        PreflightCheck::host("discord bot token key set", token, |answer| {
            match answer.stdout.trim() {
                "set" => Ok("DISCORD_BOT_TOKEN is set (value not read)".into()),
                _ => Err("DISCORD_BOT_TOKEN is empty in the API env file".into()),
            }
        }),
        PreflightCheck::host(
            "bot reads the main guild",
            discord_member_reader::member(settings, GuildScope::Main),
            |answer| match member_read(&answer.stdout) {
                Some(read) if read.outcome == "member" => {
                    Ok(format!("member of guild {}", read.guild_id))
                }
                Some(read) => Err(format!("bot read answered {}", read.outcome)),
                None => Err(format!("no member read (exit {})", answer.exit_code)),
            },
        ),
        PreflightCheck::host(
            "bot reads the partner guild, partner role held",
            discord_member_reader::member(settings, GuildScope::Partner),
            move |answer| match member_read(&answer.stdout) {
                Some(read) if read.holds_role(&role) == Some(true) => {
                    Ok(format!("role {role} held in {}", read.guild_id))
                }
                Some(read) => Err(format!(
                    "bot read answered {} without role {role}",
                    read.outcome
                )),
                None => Err(format!("no member read (exit {})", answer.exit_code)),
            },
        ),
        PreflightCheck::host(
            "operator snapshot fresh",
            match targets.select(
                &MAIN_SNAPSHOT,
                &[
                    ("discord_id", targets.operator.clone()),
                    ("partner_guild_id", targets.partner_guild.clone()),
                ],
            ) {
                Ok(command) => command,
                Err(error) => RemoteCommand::read(
                    "database",
                    format!("echo {}; exit 1", shell_quote(&format!("{error:#}"))),
                ),
            },
            |answer| match snapshot(&answer.stdout) {
                Some(row)
                    if row.status == "member"
                        && row.last_error.is_empty()
                        && row.now_ms.saturating_sub(row.verified_ms) <= FRESH_SNAPSHOT_MS =>
                {
                    Ok(format!(
                        "member, verified {} ms ago",
                        row.now_ms - row.verified_ms
                    ))
                }
                Some(row) => Err(format!(
                    "{} verified {} last_error {:?}",
                    row.status, row.verified_ms, row.last_error
                )),
                None => Err("no main-guild snapshot of the operator".into()),
            },
        ),
    ];
    checks.push(PreflightCheck::host(
        "pre-Discord backup taken",
        RemoteCommand::read(
            "backups",
            format!(
                "find {} -maxdepth 2 -name 'tbd_reforger-pre-discord-*.dump' -size +0 -mmin -1440",
                shell_quote(&format!("{}/tbd/backups", settings.home))
            ),
        ),
        |answer| match answer.stdout.lines().find(|line| !line.trim().is_empty()) {
            Some(file) => Ok(file.trim().to_string()),
            None => Err(
                "no pre-discord backup in the last 24 h (staging backup --label pre-discord)"
                    .into(),
            ),
        },
    ));
    checks
}
