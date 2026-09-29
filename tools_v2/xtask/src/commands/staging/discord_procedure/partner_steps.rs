//! Steps 1 and 2 of the Discord procedure: the partner-only registration and the loss of the
//! partner role, with the 60-second propagation measure; and the probe builders every step uses.
//!
//! **Role:** builds the `partner_registration` step (`partner_membership`) and the
//! `partner_role_removal` step (`eligibility_release`, `partner_role_propagation_within_60_seconds`),
//! and the shared helpers: database and bot-read probes, measured ids, effects.
//!
//! **Position:** called by `DiscordProcedure::plan` in `mod.rs`; the other step modules use the
//! helpers; the probes read through `membership_queries.rs` and the bot member read of
//! `remote_observers/discord_member_reader.rs`.
//!
//! **Signals & state:** none; the steps measure `partner_event_id` and `registration_id` for the
//! later steps.
//!
//! **Invariants:** the propagation deadline counts from the first bot read that shows the role
//! gone (its answer time on the host's clock) to the release's `withdrawn_at` on the database's
//! clock, both of the staging host; a bot read showing the operator outside the partner guild
//! contradicts the step, which removes a role and nothing else.

use anyhow::{Context, Result};
use serde_json::Value;

use super::discord_cases::{
    ELIGIBILITY_RELEASE, PARTNER_MEMBERSHIP, PROPAGATION_WITHIN_60_SECONDS, case,
};
use super::membership_queries::{
    AUDIT_ROWS_SINCE, DiscordTargets, OPERATOR_REGISTRATION, PARTNER_EVENT, PARTNER_SNAPSHOT,
    first_row, millis, snapshot,
};
use crate::commands::staging::procedure_runner::step::{
    Deadline, EffectPredicate, Probe, ProbeVerdict, RequestPredicate, Step, StepContext, StepId,
    StepKind,
};
use crate::commands::staging::remote_observers::database_reader::CommittedQuery;
use crate::commands::staging::remote_observers::discord_member_reader::{
    self, GuildScope, member_read,
};

/// The title the orchestrator gives the partner event.
pub(crate) const PARTNER_EVENT_TITLE: &str = "[Discord staging] Partner event";
/// How long the browser actions of a step may take from their request row.
pub(super) const BROWSER_ACTION_SECONDS: u64 = 900;
/// The propagation measure.
const PROPAGATION_SECONDS: u64 = 60;
/// The refusal code the first registration meets.
const VERIFICATION_REQUIRED: &str = "MEMBERSHIP_VERIFICATION_REQUIRED";
/// The audit action of a released reservation.
const RELEASE_AUDIT_ACTION: &str = "event.reservation_released";

/// The operator's site role: `role`.
const OPERATOR_ROLE: CommittedQuery = CommittedQuery {
    name: "operator_role",
    sql: "SELECT role::text FROM users WHERE discord_id = :'discord_id'",
    parameters: &["discord_id"],
};

/// Binds a probe's parameters from the targets and the step context.
pub(super) type Bind = fn(&DiscordTargets, &StepContext<'_>) -> Result<Vec<(&'static str, String)>>;

/// A probe polling the committed `query` with the values `bind` supplies.
pub(super) fn database_probe(
    targets: &DiscordTargets,
    query: CommittedQuery,
    bind: Bind,
    judge: impl Fn(&str, &StepContext<'_>) -> ProbeVerdict + 'static,
) -> Probe {
    let targets = targets.clone();
    Probe::host(
        move |context| targets.select(&query, &bind(&targets, context)?),
        judge,
    )
}

/// A probe polling the bot's read of the operator in `guild`.
pub(super) fn member_probe(
    targets: &DiscordTargets,
    guild: GuildScope,
    judge: impl Fn(&str, &StepContext<'_>) -> ProbeVerdict + 'static,
) -> Probe {
    let settings = targets.settings.clone();
    Probe::host(
        move |_| Ok(discord_member_reader::member(&settings, guild)),
        judge,
    )
}

/// The string an earlier probe measured as `key`.
pub(super) fn measured(context: &StepContext<'_>, key: &str) -> Result<String> {
    context
        .measurements
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .with_context(|| format!("{key} was not measured by an earlier step"))
}

/// An effect deciding `case_name`.
pub(super) fn effect(
    id: &str,
    description: &str,
    probe: Probe,
    deadline: Deadline,
    case_name: &str,
) -> Result<EffectPredicate> {
    Ok(EffectPredicate {
        id: id.to_string(),
        description: description.to_string(),
        probe,
        deadline,
        case: case(case_name)?,
    })
}

/// The operator and the partner guild.
pub(super) fn operator_in_partner(
    targets: &DiscordTargets,
    _: &StepContext<'_>,
) -> Result<Vec<(&'static str, String)>> {
    Ok(vec![
        ("discord_id", targets.operator.clone()),
        ("partner_guild_id", targets.partner_guild.clone()),
    ])
}

fn registration_on_event(
    targets: &DiscordTargets,
    context: &StepContext<'_>,
) -> Result<Vec<(&'static str, String)>> {
    Ok(vec![
        ("event_id", measured(context, "partner_event_id")?),
        ("discord_id", targets.operator.clone()),
    ])
}

/// Step 1: the partner event, the refused registration that enrols the partner snapshot, and
/// the registration that follows.
pub(super) fn partner_registration(targets: &DiscordTargets) -> Result<Step> {
    let within = Deadline::from_request_row(BROWSER_ACTION_SECONDS);
    let request = RequestPredicate {
        description: format!("the partner group of \"{PARTNER_EVENT_TITLE}\""),
        probe: database_probe(
            targets,
            PARTNER_EVENT,
            |targets, _| {
                Ok(vec![
                    ("title", PARTNER_EVENT_TITLE.to_string()),
                    ("partner_guild_id", targets.partner_guild.clone()),
                ])
            },
            |text, _| match first_row(text) {
                Some(row) if row.len() == 2 => match millis(row.get(1)) {
                    Some(at) => ProbeVerdict::Satisfied(
                        ProbeVerdict::satisfied(format!("partner event {} created", row[0]))
                            .at(at)
                            .measure("partner_event_id", row[0].clone()),
                    ),
                    None => ProbeVerdict::Pending(format!("unreadable row {row:?}")),
                },
                _ => ProbeVerdict::Pending("no partner event with its partner group yet".into()),
            },
        ),
    };
    let effects = vec![
        effect(
            "registration_refused",
            "the first registration refused MEMBERSHIP_VERIFICATION_REQUIRED",
            Probe::browser_inbox(|text, _| {
                if text.contains(VERIFICATION_REQUIRED) {
                    ProbeVerdict::Satisfied(ProbeVerdict::satisfied(
                        "the registration answer names MEMBERSHIP_VERIFICATION_REQUIRED",
                    ))
                } else {
                    ProbeVerdict::Contradicted(format!(
                        "the saved answer does not name {VERIFICATION_REQUIRED}"
                    ))
                }
            }),
            within,
            PARTNER_MEMBERSHIP,
        )?,
        effect(
            "partner_snapshot_verified",
            "the operator's partner-guild snapshot is a verified membership",
            database_probe(
                targets,
                PARTNER_SNAPSHOT,
                operator_in_partner,
                |text, _| match snapshot(text) {
                    Some(row) if row.status == "member" && row.verified_ms > 0 => {
                        ProbeVerdict::Satisfied(
                            ProbeVerdict::satisfied(format!(
                                "partner snapshot member, verified at {}",
                                row.verified_ms
                            ))
                            .at(row.verified_ms),
                        )
                    }
                    Some(row) => ProbeVerdict::Pending(format!("partner snapshot {}", row.status)),
                    None => ProbeVerdict::Pending("no partner snapshot yet".into()),
                },
            ),
            within,
            PARTNER_MEMBERSHIP,
        )?,
        effect(
            "registration_active",
            "the operator's registration on the partner event holds",
            database_probe(
                targets,
                OPERATOR_REGISTRATION,
                registration_on_event,
                |text, _| match first_row(text) {
                    Some(row) if row.get(1).map(String::as_str) == Some("registered") => {
                        ProbeVerdict::Satisfied(
                            ProbeVerdict::satisfied(format!("registration {} registered", row[0]))
                                .measure("registration_id", row[0].clone()),
                        )
                    }
                    Some(row) => ProbeVerdict::Pending(format!("registration row {row:?}")),
                    None => ProbeVerdict::Pending("no registration yet".into()),
                },
            ),
            within,
            PARTNER_MEMBERSHIP,
        )?,
        effect(
            "site_role_unchanged",
            "the operator's site role stays admin",
            database_probe(
                targets,
                OPERATOR_ROLE,
                |targets, _| Ok(vec![("discord_id", targets.operator.clone())]),
                |text, _| match first_row(text).and_then(|row| row.first().cloned()) {
                    Some(role) if role == "admin" => {
                        ProbeVerdict::Satisfied(ProbeVerdict::satisfied("site role admin"))
                    }
                    Some(role) => ProbeVerdict::Contradicted(format!("site role {role}")),
                    None => ProbeVerdict::Pending("no user row".into()),
                },
            ),
            within,
            PARTNER_MEMBERSHIP,
        )?,
    ];
    Ok(Step {
        id: StepId::new("partner_registration")?,
        kind: StepKind::ChromeAction,
        instruction: format!(
            "create the partner-only event \"{PARTNER_EVENT_TITLE}\" with its partner group \
             (guild {}, role {}); register the operator, save the refused answer \
             (MEMBERSHIP_VERIFICATION_REQUIRED) in the browser inbox, then register again",
            targets.partner_guild, targets.partner_role
        ),
        request: Some(request),
        effects,
    })
}

/// The registration released `eligibility_lost`: `Satisfied` at its `withdrawn_at`.
fn released(text: &str, _: &StepContext<'_>) -> ProbeVerdict {
    match first_row(text) {
        Some(row) if row.get(2).map(String::as_str) == Some("eligibility_lost") => {
            match millis(row.get(3)).filter(|at| *at > 0) {
                Some(at) => ProbeVerdict::Satisfied(
                    ProbeVerdict::satisfied(format!(
                        "registration {} {} eligibility_lost at {at}",
                        row[0], row[1]
                    ))
                    .at(at),
                ),
                None => ProbeVerdict::Pending("released without withdrawn_at".into()),
            }
        }
        Some(row) if row.get(1).map(String::as_str) == Some("withdrawn") => {
            ProbeVerdict::Contradicted(format!("released for another reason: {row:?}"))
        }
        Some(row) => ProbeVerdict::Pending(format!("registration {}", row[1..].join("|"))),
        None => ProbeVerdict::Pending("no registration".into()),
    }
}

/// Step 2: the partner role removed in Discord, the release, its audit row, and the 60-second
/// propagation measure.
pub(super) fn partner_role_removal(targets: &DiscordTargets) -> Result<Step> {
    let role = targets.partner_role.clone();
    let request = RequestPredicate {
        description: "the first bot read that shows the partner role gone".into(),
        probe: member_probe(
            targets,
            GuildScope::Partner,
            move |text, _| match member_read(text) {
                Some(read) => match (read.outcome.as_str(), read.holds_role(&role)) {
                    ("member", Some(false)) => ProbeVerdict::Satisfied(
                        ProbeVerdict::satisfied(format!(
                            "bot read answered at {} shows role {role} gone",
                            read.answered_at_unix_ms
                        ))
                        .at(read.answered_at_unix_ms)
                        .measure("role_gone_seen_ms", read.answered_at_unix_ms),
                    ),
                    ("nonmember", _) => ProbeVerdict::Contradicted(
                        "the operator left the partner guild; the step removes a role only".into(),
                    ),
                    (outcome, _) => ProbeVerdict::Pending(format!("bot read {outcome}, role held")),
                },
                None => ProbeVerdict::Pending("no member read line".into()),
            },
        ),
    };
    let release_audit: Bind = |_, context| {
        Ok(vec![
            ("action", RELEASE_AUDIT_ACTION.to_string()),
            ("target_type", "event_registration".to_string()),
            ("target_id", measured(context, "registration_id")?),
            ("since_ms", context.step_started_unix_ms.to_string()),
        ])
    };
    let effects = vec![
        effect(
            "release_within_60_seconds",
            "the reservation released within 60 s of the bot read",
            database_probe(
                targets,
                OPERATOR_REGISTRATION,
                registration_on_event,
                released,
            ),
            Deadline::from_request_row(PROPAGATION_SECONDS),
            PROPAGATION_WITHIN_60_SECONDS,
        )?,
        effect(
            "reservation_released",
            "the reservation released eligibility_lost",
            database_probe(
                targets,
                OPERATOR_REGISTRATION,
                registration_on_event,
                released,
            ),
            Deadline::from_request_row(BROWSER_ACTION_SECONDS),
            ELIGIBILITY_RELEASE,
        )?,
        effect(
            "release_audited",
            "an event.reservation_released audit row for the registration",
            database_probe(targets, AUDIT_ROWS_SINCE, release_audit, audit_row),
            Deadline::from_request_row(BROWSER_ACTION_SECONDS),
            ELIGIBILITY_RELEASE,
        )?,
    ];
    Ok(Step {
        id: StepId::new("partner_role_removal")?,
        kind: StepKind::ChromeAction,
        instruction: format!(
            "in Discord, remove role {} from member {} in the partner guild {}",
            targets.partner_role, targets.operator, targets.partner_guild
        ),
        request: Some(request),
        effects,
    })
}

/// At least one audit row since the step's start: `Satisfied` at the newest.
pub(super) fn audit_row(text: &str, _: &StepContext<'_>) -> ProbeVerdict {
    let row = first_row(text).unwrap_or_default();
    match (millis(row.first()), millis(row.get(1))) {
        (Some(count), Some(at)) if count > 0 => ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(format!("{count} audit row(s), newest at {at}")).at(at),
        ),
        _ => ProbeVerdict::Pending("no audit row yet".into()),
    }
}
