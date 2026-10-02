//! Steps 7 and 8 of the Discord procedure: the Get Guild Member bucket spent 0.3 s before the
//! operator's next refresh, and the partner event's deletion.
//!
//! **Role:** builds `rate_limit_baseline` and `bucket_spend` (`rate_limit`,
//! `rate_limit_recovery`), the host-side spend command, and `partner_event_deletion` (cleanup, no
//! case).
//!
//! **Position:** called by `DiscordProcedure::plan` and `operator_actions.rs`; the spend's tool
//! command comes from the one argv builder of the host tool's `spend-discord-member-bucket`,
//! `spend_discord_member_bucket` in `remote_actions/host_fixture_commands.rs`.
//!
//! **Signals & state:** none; the baseline step measures `rate_limited_before`.
//!
//! **Invariants:** the spend's start is computed on the host from the snapshot's
//! `next_refresh_at` (the database clock, the clock the tool waits on), so no workstation clock
//! enters the timing; the spend makes at most 50 requests; the recovery counts only a
//! verification after the logged rate-limited outcome; the deletion step decides no case.

use anyhow::Result;

use super::discord_cases::{RATE_LIMIT, RATE_LIMIT_RECOVERY};
use super::membership_queries::{
    DiscordTargets, EVENT_DELETION, MAIN_SNAPSHOT, first_row, millis, snapshot,
};
use super::partner_steps::{
    PARTNER_EVENT_TITLE, database_probe, effect, measured, operator_in_partner,
};
use super::reconciliation_readers::{OUTCOME_COUNTER, counted_outcome, logged_outcome};
use crate::commands::staging::procedure_runner::step::{
    Deadline, Probe, ProbeVerdict, RequestPredicate, Step, StepId, StepKind,
};
use crate::commands::staging::remote_actions::host_fixture_commands::{
    SpendStart, spend_discord_member_bucket,
};
use crate::commands::staging::remote_observers::database_reader::READ_ONLY_SESSION_OPTIONS;
use crate::commands::staging::remote_observers::metrics_reader;
use crate::commands::staging::remote_observers::remote_command::{RemoteCommand, shell_quote};
use crate::commands::staging::staging_settings::{
    STAGING_DATABASE, STAGING_DATABASE_USER, StagingSettings,
};

/// How long before `next_refresh_at` the spend starts.
pub(crate) const SPEND_LEAD_MS: u64 = 300;
/// How long the spend keeps the bucket spent.
pub(crate) const BUCKET_HOLD_SECONDS: u32 = 3;
/// The most requests the spend sends.
pub(crate) const BUCKET_MAX_REQUESTS: u32 = 50;
/// The rate-limit effects' deadline from the step's start: the refresh is at most 40 s away.
const RATE_LIMIT_SECONDS: u64 = 180;
/// The spend script's variable holding the start it computed on the host.
const SPEND_START_VARIABLE: &str = "start_ms";

/// The spend: reads the operator's main-guild `next_refresh_at` on the host, then runs the tool
/// from [`SPEND_LEAD_MS`] before it.
pub(crate) fn bucket_spend(settings: &StagingSettings) -> Result<RemoteCommand> {
    let operator = settings.operator()?;
    let tool = spend_discord_member_bucket(
        settings,
        SpendStart::HostVariable(SPEND_START_VARIABLE),
        BUCKET_HOLD_SECONDS,
        BUCKET_MAX_REQUESTS,
    )?;
    Ok(RemoteCommand::change_script(
        "host tool",
        format!(
            "set -euo pipefail\n\
             guild=\"$(sed -n 's/^DISCORD_GUILD_ID=//p' {env} | tail -n 1 | tr -d '\"\\r')\"\n\
             case \"$guild\" in ''|*[!0-9]*) echo 'DISCORD_GUILD_ID names no guild' >&2; exit 3;; esac\n\
             next_ms=\"$(docker exec -i -e {session} {container} psql -X -A -t -q -v ON_ERROR_STOP=1 -U {user} -d {database} -v {operator} -v \"guild_id=$guild\" <<'SQL'\n\
             SELECT floor(extract(epoch FROM next_refresh_at) * 1000)::bigint FROM discord_membership_snapshots WHERE discord_id = :'discord_id' AND guild_id = :'guild_id';\n\
             SQL\n\
             )\"\n\
             case \"$next_ms\" in ''|*[!0-9]*) echo 'the operator has no main-guild snapshot' >&2; exit 3;; esac\n\
             {start}=$((next_ms - {SPEND_LEAD_MS}))\n\
             echo \"spend: next_refresh_at_unix_ms=$next_ms start_at_unix_ms=${start}\"\n\
             {command}\n",
            env = shell_quote(&settings.api_env_file()),
            session = shell_quote(READ_ONLY_SESSION_OPTIONS),
            container = shell_quote(&settings.database_container),
            user = shell_quote(STAGING_DATABASE_USER),
            database = shell_quote(STAGING_DATABASE),
            operator = shell_quote(&format!("discord_id={operator}")),
            start = SPEND_START_VARIABLE,
            command = tool.command_line,
        ),
    ))
}

/// Step 7: the counter before the spend, then the spend and its effects.
pub(super) fn rate_limit_steps(targets: &DiscordTargets) -> Result<Vec<Step>> {
    let settings = targets.settings.clone();
    let baseline = Step {
        id: StepId::new("rate_limit_baseline")?,
        kind: StepKind::Observation,
        instruction: "none: the harness reads the rate-limited count before the spend".into(),
        request: None,
        effects: vec![effect(
            "counter_before_spend",
            "the rate-limited count before the spend",
            Probe::host(
                move |_| {
                    Ok(metrics_reader::exposition(
                        &settings.api_env_file(),
                        &settings.api_origin,
                    ))
                },
                |text, _| match metrics_reader::sample(
                    text,
                    OUTCOME_COUNTER,
                    &[("outcome", "rate_limited")],
                ) {
                    Some(count) => ProbeVerdict::Satisfied(
                        ProbeVerdict::satisfied(format!(
                            "rate_limited count {count} before the spend"
                        ))
                        .measure("rate_limited_before", count),
                    ),
                    None => ProbeVerdict::Pending("no rate_limited sample".into()),
                },
            ),
            Deadline::from_step_start(60),
            RATE_LIMIT,
        )?],
    };
    let within = Deadline::from_step_start(RATE_LIMIT_SECONDS);
    let spend = Step {
        id: StepId::new("bucket_spend")?,
        kind: StepKind::HostAction(bucket_spend(&targets.settings)?),
        instruction: "none: the harness spends the main guild's Get Guild Member bucket".into(),
        request: None,
        effects: vec![
            effect(
                "rate_limited_logged",
                "the API logs a rate-limited main-guild reconciliation",
                logged_outcome(targets, "rate_limited"),
                within,
                RATE_LIMIT,
            )?,
            effect(
                "rate_limited_counted",
                "the rate-limited count moved",
                counted_outcome(targets, "rate_limited", Some("rate_limited_before")),
                within,
                RATE_LIMIT,
            )?,
            effect(
                "verified_after_rate_limit",
                "the main snapshot verified again after the rate limit",
                database_probe(
                    targets,
                    MAIN_SNAPSHOT,
                    operator_in_partner,
                    |text, context| {
                        let limited = context
                            .measurements
                            .get("rate_limited_logged_ms")
                            .and_then(serde_json::Value::as_u64);
                        match (snapshot(text), limited) {
                            (Some(row), Some(at))
                                if row.status == "member"
                                    && row.last_error.is_empty()
                                    && row.verified_ms > at =>
                            {
                                ProbeVerdict::Satisfied(
                                    ProbeVerdict::satisfied(format!(
                                        "member, verified {} after the rate limit at {at}",
                                        row.verified_ms
                                    ))
                                    .at(row.verified_ms),
                                )
                            }
                            (_, None) => {
                                ProbeVerdict::Pending("no rate-limited outcome logged yet".into())
                            }
                            (Some(row), _) => ProbeVerdict::Pending(format!(
                                "{} verified {} last_error {:?}",
                                row.status, row.verified_ms, row.last_error
                            )),
                            (None, _) => ProbeVerdict::Pending("no main snapshot".into()),
                        }
                    },
                ),
                Deadline::from_step_start(RATE_LIMIT_SECONDS + 120),
                RATE_LIMIT_RECOVERY,
            )?,
        ],
    };
    Ok(vec![baseline, spend])
}

/// Step 8: the orchestrator deletes the partner event; observed, deciding no case.
pub(super) fn partner_event_deletion(targets: &DiscordTargets) -> Result<Step> {
    Ok(Step {
        id: StepId::new("partner_event_deletion")?,
        kind: StepKind::ChromeAction,
        instruction: format!("delete the partner event \"{PARTNER_EVENT_TITLE}\""),
        request: Some(RequestPredicate {
            description: "the partner event's deletion".into(),
            probe: database_probe(
                targets,
                EVENT_DELETION,
                |_, context| Ok(vec![("event_id", measured(context, "partner_event_id")?)]),
                |text, _| match millis(first_row(text).unwrap_or_default().first()) {
                    Some(at) if at > 0 => ProbeVerdict::Satisfied(
                        ProbeVerdict::satisfied(format!("partner event deleted at {at}")).at(at),
                    ),
                    _ => ProbeVerdict::Pending("the partner event still exists".into()),
                },
            ),
        }),
        effects: Vec::new(),
    })
}
