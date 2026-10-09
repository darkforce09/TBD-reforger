//! A recorded Discord run on the fake clock: a host that answers the procedure's reads with the
//! staging host's recorded answers, timed from the moments the procedure's own actions and first
//! reads reached it, and that saves the orchestrator's browser inbox entries during their steps.
//! One planted [`Defect`] per run breaks exactly one declared case.
use std::path::PathBuf;

use crate::error::{Result, bail};
use serde_json::json;

use super::membership_queries::{
    AUDIT_ROWS_SINCE, EVENT_DELETION, GRACE_OVERRIDE, MAIN_SNAPSHOT, OPERATOR_REGISTRATION,
    PARTNER_EVENT, PARTNER_ROLE_ROWS, PARTNER_SNAPSHOT,
};
use super::outage_steps::STAGED_SNAPSHOT_AGE_HOURS;
use super::rate_limit_steps::bucket_spend;
use crate::procedure_runner::fake_clock::FakeClock;
use crate::procedure_runner::runner_support::TEST_DEPLOY_ENV;
use crate::remote_actions::host_fixture_commands::age_membership_snapshot;
use crate::remote_actions::outage_dropin;
use crate::remote_observers::database_reader::CommittedQuery;
use crate::remote_observers::discord_member_reader::{self, GuildScope};
use crate::remote_observers::remote_command::{CommandOutput, HostCommandRunner, RemoteCommand};
use crate::staging_settings::StagingSettings;
use deploy_settings::DeployEnvironment;
use time_source::Clock as _;

/// The fake clock's start.
pub(crate) const T0: u64 = 1_800_000_000_000;
/// The operator of the test settings.
pub(crate) const OPERATOR: &str = "123456789012345678";
/// The partner guild of the test settings.
pub(crate) const PARTNER_GUILD: &str = "223456789012345678";
/// The partner role the Discord settings add.
pub(crate) const PARTNER_ROLE: &str = "323456789012345678";
const MAIN_GUILD: &str = "423456789012345678";
const EVENT_ID: &str = "00000000-0000-4000-8000-00000000d15c";
const REGISTRATION_ID: &str = "00000000-0000-4000-8000-0000000000a1";
const OUTAGE_ERROR: &str = "discord unavailable: proxy CONNECT refused";
const HOUR_MS: u64 = 3_600_000;
const COUNTER: &str = "tbd_discord_reconcile_outcomes_total";

/// The test deploy settings with the partner role the Discord procedure needs.
pub(crate) fn discord_settings() -> StagingSettings {
    let text = format!("{TEST_DEPLOY_ENV}TBD_STAGING_PARTNER_ROLE_ID={PARTNER_ROLE}\n");
    let environment =
        DeployEnvironment::from_text(std::path::Path::new("/deploy.env"), Some(&text), Vec::new())
            .unwrap();
    StagingSettings::from_environment(&environment).unwrap()
}

/// One planted defect; each breaks the case named in its comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Defect {
    /// `partner_membership`: the saved registration answer names no verification refusal.
    RegistrationNotRefused,
    /// `eligibility_release`: the release is never audited.
    ReleaseUnaudited,
    /// `partner_role_propagation_within_60_seconds`: the release lands 61 s after the bot read.
    LateRelease,
    /// `network_outage`: no `unavailable` outcome is logged or counted during the drop-in.
    NoUnavailableOutcome,
    /// `staleness_warning`: the outage page read shows no banner.
    BannerAbsent,
    /// `cached_grace`: the outage page read shows the operator as a guest.
    GuestDuringOutage,
    /// `non_blocking_during_outage`: `/api/v1/events` answered 503 during the outage.
    EventsUnavailable,
    /// `admin_override`: no override row is ever written.
    OverrideMissing,
    /// `outage_recovery`: the partner role never syncs back.
    PartnerRoleNotSynced,
    /// `rate_limit`: the rate-limited count stays at its baseline.
    CounterNotMoving,
    /// `rate_limit_recovery`: no verification follows the rate limit.
    NoVerificationAfterRateLimit,
}

/// When the procedure's actions and first reads reached the host.
#[derive(Debug, Default)]
struct Marks {
    event_asked: Option<u64>,
    removal_asked: Option<u64>,
    install: Option<u64>,
    aging: Option<u64>,
    override_asked: Option<u64>,
    removal: Option<u64>,
    restore_asked: Option<u64>,
    spend: Option<u64>,
    deletion_asked: Option<u64>,
}

/// `mark + offset` once the clock has reached it.
fn after(mark: Option<u64>, offset: u64, now: u64) -> Option<u64> {
    mark.map(|at| at + offset).filter(|at| now >= *at)
}

/// The staging host of a recorded Discord run.
pub(crate) struct RecordedDiscord {
    clock: FakeClock,
    defect: Option<Defect>,
    inbox: Box<dyn Fn() -> PathBuf>,
    actions: [RemoteCommand; 4],
    partner_read: RemoteCommand,
    marks: Marks,
    /// Every command it was asked to run, in order.
    pub calls: Vec<RemoteCommand>,
}

impl RecordedDiscord {
    /// The host of a run on `clock` whose browser inbox folder `inbox` names, with `defect`.
    pub(crate) fn new(
        settings: &StagingSettings,
        clock: &FakeClock,
        inbox: impl Fn() -> PathBuf + 'static,
        defect: Option<Defect>,
    ) -> Self {
        Self {
            clock: clock.clone(),
            defect,
            inbox: Box::new(inbox),
            actions: [
                outage_dropin::install(settings),
                age_membership_snapshot(settings, OPERATOR, STAGED_SNAPSHOT_AGE_HOURS),
                outage_dropin::remove(settings),
                bucket_spend(settings).unwrap(),
            ],
            partner_read: discord_member_reader::member(settings, GuildScope::Partner),
            marks: Marks::default(),
            calls: Vec::new(),
        }
    }

    fn planted(&self, defect: Defect) -> bool {
        self.defect == Some(defect)
    }

    /// Saves the orchestrator's page read of `step`, captured at `captured_at_unix_ms`.
    fn save_page(&self, step: &str, captured_at_unix_ms: u64, output: &str) -> Result<()> {
        let folder = (self.inbox)();
        std::fs::create_dir_all(&folder)?;
        let entry = json!({ "captured_at_unix_ms": captured_at_unix_ms, "output": output });
        std::fs::write(folder.join(format!("{step}.json")), entry.to_string())?;
        Ok(())
    }

    fn host_action(&mut self, index: usize, now: u64) -> Result<String> {
        match index {
            0 => {
                self.marks.install.get_or_insert(now);
                let banner = if self.planted(Defect::BannerAbsent) {
                    ""
                } else {
                    "banner: Discord verification is delayed; cached roles still apply\n"
                };
                let role = if self.planted(Defect::GuestDuringOutage) {
                    "guest"
                } else {
                    "admin"
                };
                let events = if self.planted(Defect::EventsUnavailable) {
                    503
                } else {
                    200
                };
                let page = format!(
                    "GET /api/v1/me 200 {}\n{banner}GET /api/v1/events {events}\n",
                    json!({"role": role, "membership_stale": true})
                );
                self.save_page("outage_page_reads", now + 310_000, &page)?;
                Ok("installed staging-discord-outage.conf; tbd-api.service restarted\n".into())
            }
            1 => {
                self.marks.aging.get_or_insert(now);
                let page = format!("GET /api/v1/me 200 {}", json!({"role": "guest"}));
                self.save_page("snapshot_aging", now + 30_000, &page)?;
                Ok("aged the main-guild snapshot by 49 h\n".into())
            }
            2 => {
                self.marks.removal.get_or_insert(now);
                Ok("removed staging-discord-outage.conf; tbd-api.service restarted\n".into())
            }
            _ => {
                self.marks.spend.get_or_insert(now);
                Ok(format!(
                    "spend: next_refresh_at_unix_ms={} start_at_unix_ms={now}\n{}",
                    now + 300,
                    member_line(MAIN_GUILD, now + 310, "rate_limited", None)
                ))
            }
        }
    }

    fn partner_member_read(&mut self, now: u64) -> String {
        let (held, answered) = if self.marks.removal.is_some() {
            let asked = *self.marks.restore_asked.get_or_insert(now);
            match after(Some(asked), 90_000, now) {
                Some(_) => (true, asked + 88_000),
                None => (false, now),
            }
        } else {
            let asked = *self.marks.removal_asked.get_or_insert(now);
            match after(Some(asked), 120_000, now) {
                Some(_) => (false, asked + 118_000),
                None => (true, now),
            }
        };
        let roles = if held {
            vec!["900", PARTNER_ROLE]
        } else {
            vec!["900"]
        };
        member_line(PARTNER_GUILD, answered, "member", Some(roles))
    }

    /// When the release of the registration is written.
    fn release_at(&self) -> Option<u64> {
        let lag = if self.planted(Defect::LateRelease) {
            61_000
        } else {
            28_000
        };
        self.marks.removal_asked.map(|asked| asked + 118_000 + lag)
    }

    fn main_snapshot(&self, now: u64) -> String {
        let marks = &self.marks;
        let recovered = !self.planted(Defect::NoVerificationAfterRateLimit);
        let (verified, error) =
            if let Some(at) = after(marks.spend, 40_000, now).filter(|_| recovered) {
                (at, "")
            } else if let (Some(_), Some(at)) = (
                after(marks.spend, 2_000, now),
                after(marks.removal, 30_000, now),
            ) {
                (at, "rate limited by Discord")
            } else if let Some(at) = after(marks.removal, 30_000, now) {
                (at, "")
            } else if let Some(aged) = marks.aging {
                (
                    aged - u64::from(STAGED_SNAPSHOT_AGE_HOURS) * HOUR_MS,
                    OUTAGE_ERROR,
                )
            } else if after(marks.install, 20_000, now).is_some() {
                (T0 - 30_000, OUTAGE_ERROR)
            } else {
                (T0 - 30_000, "")
            };
        format!("member|{verified}|{error}|{}|{now}\n", verified + HOUR_MS)
    }

    fn journal(&self, now: u64) -> String {
        let mut lines = journal_line(T0 - 600_000, "verified");
        if let Some(at) = after(self.marks.install, 20_000, now)
            && !self.planted(Defect::NoUnavailableOutcome)
        {
            lines.push_str(&journal_line(at, "unavailable"));
        }
        if let Some(at) = after(self.marks.spend, 2_000, now) {
            lines.push_str(&journal_line(at, "rate_limited"));
        }
        lines
    }

    fn metrics(&self, now: u64) -> String {
        let mut text = format!("# TYPE {COUNTER} counter\n{COUNTER}{{outcome=\"verified\"}} 12\n");
        if after(self.marks.install, 20_000, now).is_some()
            && !self.planted(Defect::NoUnavailableOutcome)
        {
            text.push_str(&format!("{COUNTER}{{outcome=\"unavailable\"}} 3\n"));
        }
        let spent = after(self.marks.spend, 2_000, now).is_some();
        let limited = if spent && !self.planted(Defect::CounterNotMoving) {
            3
        } else {
            2
        };
        text.push_str(&format!(
            "{COUNTER}{{outcome=\"rate_limited\"}} {limited}\n"
        ));
        text
    }

    fn database(&mut self, command: &RemoteCommand, now: u64) -> Result<String> {
        let sql = command.stdin.as_deref().unwrap_or_default();
        let asks = |query: &CommittedQuery| sql == format!("{};\n", query.sql);
        let binds = |fragment: &str| command.command_line.contains(fragment);
        let release = self.release_at().filter(|at| now >= *at);
        Ok(if asks(&PARTNER_EVENT) {
            if self.marks.event_asked.is_none() {
                self.marks.event_asked = Some(now);
                let answer = if self.planted(Defect::RegistrationNotRefused) {
                    json!({"status": 201, "state": "registered"})
                } else {
                    json!({"status": 409, "code": "MEMBERSHIP_VERIFICATION_REQUIRED"})
                };
                let page = format!("POST /api/v1/events/{EVENT_ID}/registrations {answer}");
                self.save_page("partner_registration", now + 50_000, &page)?;
            }
            match after(self.marks.event_asked, 60_000, now) {
                Some(at) => format!("{EVENT_ID}|{}\n", at - 5_000),
                None => String::new(),
            }
        } else if asks(&PARTNER_SNAPSHOT) {
            let verified = self.marks.event_asked.unwrap_or(T0) + 52_000;
            format!("member|{verified}||{}|{now}\n", verified + HOUR_MS)
        } else if asks(&OPERATOR_REGISTRATION) {
            match release {
                Some(at) => format!("{REGISTRATION_ID}|withdrawn|eligibility_lost|{at}\n"),
                None => format!("{REGISTRATION_ID}|registered||0\n"),
            }
        } else if sql.contains("FROM users WHERE discord_id") {
            "admin\n".into()
        } else if asks(&AUDIT_ROWS_SINCE) && binds("action=event.reservation_released") {
            match release.filter(|_| !self.planted(Defect::ReleaseUnaudited)) {
                Some(at) => format!("1|{at}\n"),
                None => "0|0\n".into(),
            }
        } else if asks(&AUDIT_ROWS_SINCE) && binds("action=membership.grace_extended") {
            match self.override_created(now) {
                Some(at) => format!("1|{at}\n"),
                None => "0|0\n".into(),
            }
        } else if asks(&MAIN_SNAPSHOT) {
            self.main_snapshot(now)
        } else if asks(&GRACE_OVERRIDE) {
            if self.marks.override_asked.is_none() {
                self.marks.override_asked = Some(now);
                let page = json!({"role": "admin", "membership_override_active": true});
                self.save_page(
                    "grace_override",
                    now + 70_000,
                    &format!("GET /api/v1/me 200 {page}"),
                )?;
            }
            match self.override_created(now) {
                Some(at) => format!("{at}|{}|{OPERATOR}\n", at + 48 * HOUR_MS),
                None => String::new(),
            }
        } else if asks(&PARTNER_ROLE_ROWS) {
            let synced = after(self.marks.restore_asked, 98_000, now).is_some()
                && !self.planted(Defect::PartnerRoleNotSynced);
            format!("{}\n", u8::from(synced))
        } else if asks(&EVENT_DELETION) {
            let asked = *self.marks.deletion_asked.get_or_insert(now);
            format!("{}\n", after(Some(asked), 45_000, now).unwrap_or(0))
        } else {
            bail!("no recorded answer for the database read {sql:?}")
        })
    }

    fn override_created(&self, now: u64) -> Option<u64> {
        after(self.marks.override_asked, 60_000, now)
            .filter(|_| !self.planted(Defect::OverrideMissing))
    }
}

impl HostCommandRunner for RecordedDiscord {
    fn run(&mut self, command: &RemoteCommand) -> Result<CommandOutput> {
        self.calls.push(command.clone());
        let now = self.clock.now_unix_ms();
        let stdout = if let Some(index) = self.actions.iter().position(|a| a == command) {
            self.host_action(index, now)?
        } else if *command == self.partner_read {
            self.partner_member_read(now)
        } else if command.command_line.contains("journalctl") {
            self.journal(now)
        } else if command
            .stdin
            .as_deref()
            .is_some_and(|s| s.contains("/metrics"))
        {
            self.metrics(now)
        } else if command.observer == "database" {
            self.database(command, now)?
        } else {
            bail!("no recorded answer for {} at {now}", command.observer)
        };
        Ok(CommandOutput {
            exit_code: 0,
            stdout,
            stderr: String::new(),
        })
    }
}

/// A `discord-member-read {json}` line as the host tool prints it.
pub(crate) fn member_line(
    guild_id: &str,
    answered: u64,
    outcome: &str,
    roles: Option<Vec<&str>>,
) -> String {
    let record = json!({
        "request": 1, "guild": "partner", "guild_id": guild_id, "discord_id": OPERATOR,
        "sent_at_unix_ms": answered - 150, "answered_at_unix_ms": answered, "http_status": 200,
        "outcome": outcome, "roles": roles, "retry_after_ms": null, "global": null, "reason": null,
    });
    format!("discord-member-read {record}\n")
}

/// One `short-unix` journal line of the API's `discord_reconciliation` event, colored as the
/// journal keeps it.
fn journal_line(unix_ms: u64, outcome: &str) -> String {
    format!(
        "{}.{:06} dooley tbd-api[4242]: \u{1b}[32m INFO\u{1b}[0m discord_reconciliation: reconciled \
         outcome=\"{outcome}\" guild_scope=\"main\" retry_after_ms=0 revision=7\n",
        unix_ms / 1000,
        (unix_ms % 1000) * 1000
    )
}
