//! Steps 3 to 6 of the Discord procedure: the outage, the browser reads during it, the aged
//! snapshot and the administrator override, and the recovery with the partner role restored.
//!
//! **Role:** builds `outage_install` (`network_outage`), `outage_page_reads` (`staleness_warning`,
//! `cached_grace`, `non_blocking_during_outage`), `snapshot_aging` and `grace_override`
//! (`admin_override`), `outage_removal` and `partner_role_restore` (`outage_recovery`), and the
//! reader of the API's `discord_reconciliation` log lines.
//!
//! **Position:** called by `DiscordProcedure::plan` in `mod.rs`; the drop-in comes from
//! `remote_actions/outage_dropin.rs`, the aging from `remote_actions/host_fixture_commands.rs`;
//! the probes read the database, `/metrics`, the API unit's journal, the bot and the browser inbox.
//!
//! **Signals & state:** none.
//!
//! **Invariants:** the aged snapshot is a staged precondition, recorded in its observation as
//! staged and never as elapsed time, and staged while the drop-in keeps Discord away so no
//! successful refresh replaces it; the reconciliation counters are in-process, so counts read
//! after the API restart that installs the drop-in are counts of the outage; a browser judge
//! reads `/api/v1/me` and banner text only, never a token.

use crate::error::Result;

use super::discord_cases::{
    ADMIN_OVERRIDE, CACHED_GRACE, NETWORK_OUTAGE, NON_BLOCKING_DURING_OUTAGE, OUTAGE_RECOVERY,
    STALENESS_WARNING,
};
use super::membership_queries::{
    AUDIT_ROWS_SINCE, DiscordTargets, GRACE_OVERRIDE, MAIN_SNAPSHOT, PARTNER_ROLE_ROWS, first_row,
    millis, snapshot,
};
use super::partner_steps::{
    BROWSER_ACTION_SECONDS, Bind, audit_row, database_probe, effect, member_probe,
    operator_in_partner,
};
use super::reconciliation_readers::{counted_outcome, logged_outcome};
use crate::procedure_runner::step::{
    Deadline, Probe, ProbeVerdict, RequestPredicate, Step, StepId, StepKind,
};
use crate::remote_actions::host_fixture_commands::age_membership_snapshot;
use crate::remote_actions::outage_dropin;
use crate::remote_observers::discord_member_reader::{GuildScope, member_read};

/// The age the operator's main-guild snapshot is staged at: past the 48 h grace.
pub(crate) const STAGED_SNAPSHOT_AGE_HOURS: u32 = 49;
/// How long the API may take after a restart to record an outcome.
const RESTART_EFFECT_SECONDS: u64 = 300;
/// The staleness banner's opening words.
const STALENESS_BANNER: &str = "Discord verification is delayed";
const HOUR_MS: u64 = 3_600_000;
/// Browser text with escapes and white space removed, for JSON field matching.
fn compact(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace() && *character != '\\')
        .collect()
}

/// A browser probe that holds when the saved text shows every JSON `fields` pair (`"key":value`).
fn page_shows(fields: &'static [&'static str], banner: Option<&'static str>) -> Probe {
    Probe::browser_inbox(move |text, _| {
        let json = compact(text);
        let missing: Vec<&str> = fields
            .iter()
            .copied()
            .filter(|pair| !json.contains(pair))
            .chain(banner.filter(|words| !text.contains(words)))
            .collect();
        if missing.is_empty() {
            ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!("page shows {fields:?}")))
        } else {
            ProbeVerdict::Contradicted(format!("the saved page read lacks {missing:?}"))
        }
    })
}

fn step(
    id: &str,
    kind: StepKind,
    instruction: String,
    request: Option<RequestPredicate>,
) -> Result<Step> {
    Ok(Step {
        id: StepId::new(id)?,
        kind,
        instruction,
        request,
        effects: Vec::new(),
    })
}

/// Steps 3 and 4: the drop-in and the API restart, then the browser reads during the outage.
pub(super) fn outage(targets: &DiscordTargets) -> Result<Vec<Step>> {
    let after_restart = Deadline::from_step_start(RESTART_EFFECT_SECONDS);
    let mut install = step(
        "outage_install",
        StepKind::HostAction(outage_dropin::install(&targets.settings)),
        "none: the harness installs the HTTPS_PROXY drop-in and restarts the API".into(),
        None,
    )?;
    install.effects = vec![
        effect(
            "unavailable_counted",
            "the API counts unavailable reconciliations",
            counted_outcome(targets, "unavailable", None),
            after_restart,
            NETWORK_OUTAGE,
        )?,
        effect(
            "unavailable_logged",
            "the API logs an unavailable main-guild reconciliation",
            logged_outcome(targets, "unavailable"),
            after_restart,
            NETWORK_OUTAGE,
        )?,
        effect(
            "last_error_set",
            "the main snapshot keeps its membership and records the failure",
            database_probe(
                targets,
                MAIN_SNAPSHOT,
                operator_in_partner,
                |text, context| match snapshot(text) {
                    Some(row)
                        if row.status == "member"
                            && !row.last_error.is_empty()
                            && row.verified_ms < context.step_started_unix_ms =>
                    {
                        ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!(
                            "member, verified {} (before the outage), last_error {:?}",
                            row.verified_ms, row.last_error
                        )))
                    }
                    Some(row)
                        if row.status != "member"
                            || row.verified_ms >= context.step_started_unix_ms =>
                    {
                        ProbeVerdict::Contradicted(format!(
                            "membership changed during the outage: {} verified {}",
                            row.status, row.verified_ms
                        ))
                    }
                    Some(_) => ProbeVerdict::Pending("no last_error yet".into()),
                    None => ProbeVerdict::Pending("no main snapshot".into()),
                },
            ),
            after_restart,
            NETWORK_OUTAGE,
        )?,
    ];
    let reads = Deadline::from_step_start(BROWSER_ACTION_SECONDS);
    let mut page_reads = step(
        "outage_page_reads",
        StepKind::ChromeAction,
        "once the staleness banner shows, save the /api/v1/me response body, the banner text and \
         the /api/v1/events request line with its status in one browser inbox entry"
            .into(),
        None,
    )?;
    page_reads.effects = vec![
        effect(
            "staleness_warning",
            "/me says stale and the banner shows",
            page_shows(&["\"membership_stale\":true"], Some(STALENESS_BANNER)),
            reads,
            STALENESS_WARNING,
        )?,
        effect(
            "cached_grace",
            "the cached admin role still applies",
            page_shows(&["\"role\":\"admin\"", "\"membership_stale\":true"], None),
            reads,
            CACHED_GRACE,
        )?,
        effect(
            "events_answered",
            "/api/v1/events answered 200 during the outage",
            Probe::browser_inbox(|text, _| {
                match text
                    .lines()
                    .find(|line| line.contains("/api/v1/events") && line.contains("200"))
                {
                    Some(line) => ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!(
                        "events read: {}",
                        line.trim()
                    ))),
                    None => ProbeVerdict::Contradicted(
                        "the saved page read has no /api/v1/events line with 200".into(),
                    ),
                }
            }),
            reads,
            NON_BLOCKING_DURING_OUTAGE,
        )?,
    ];
    Ok(vec![install, page_reads])
}

/// Step 5: the staged age, the guest `/me`, then the override granted in the browser.
pub(super) fn override_steps(targets: &DiscordTargets) -> Result<Vec<Step>> {
    let within = Deadline::from_step_start(BROWSER_ACTION_SECONDS);
    let mut aging = step(
        "snapshot_aging",
        StepKind::HostAction(age_membership_snapshot(&targets.settings, &targets.operator, STAGED_SNAPSHOT_AGE_HOURS)),
        "save the /api/v1/me response body (role guest) in the browser inbox once the harness has aged the snapshot".into(),
        None,
    )?;
    aging.effects = vec![
        effect(
            "snapshot_aged",
            "the main snapshot is staged 49 h old",
            database_probe(
                targets,
                MAIN_SNAPSHOT,
                operator_in_partner,
                |text, _| match snapshot(text) {
                    Some(row)
                        if row.verified_ms > 0
                            && row.now_ms.saturating_sub(row.verified_ms)
                                >= u64::from(STAGED_SNAPSHOT_AGE_HOURS) * HOUR_MS - 1000 =>
                    {
                        ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!(
                            "staged precondition: verified_at set {STAGED_SNAPSHOT_AGE_HOURS} h back by age-membership-snapshot (not elapsed time); age {} ms",
                            row.now_ms - row.verified_ms
                        )))
                    }
                    Some(row) => ProbeVerdict::Pending(format!(
                        "snapshot age {} ms",
                        row.now_ms.saturating_sub(row.verified_ms)
                    )),
                    None => ProbeVerdict::Pending("no main snapshot".into()),
                },
            ),
            within,
            ADMIN_OVERRIDE,
        )?,
        effect(
            "guest_shown",
            "/me shows the operator as a guest",
            page_shows(&["\"role\":\"guest\""], None),
            within,
            ADMIN_OVERRIDE,
        )?,
    ];
    let request = RequestPredicate {
        description: "the operator's grace override row".into(),
        probe: database_probe(
            targets,
            GRACE_OVERRIDE,
            operator_in_partner,
            |text, context| {
                let row = first_row(text).unwrap_or_default();
                match (millis(row.first()), millis(row.get(1))) {
                    (Some(created), Some(expires)) if created >= context.step_started_unix_ms => {
                        ProbeVerdict::Satisfied(
                            ProbeVerdict::satisfied(format!(
                                "override created {created}, expires {expires}, by {}",
                                row.get(2).cloned().unwrap_or_default()
                            ))
                            .at(created)
                            .measure("override_hours_ms", expires - created),
                        )
                    }
                    _ => ProbeVerdict::Pending("no override granted in this step yet".into()),
                }
            },
        ),
    };
    let mut grant = step(
        "grace_override",
        StepKind::ChromeAction,
        "grant the override from the banner form (Extend for 48 hours) with a reason, then save \
         the /api/v1/me response body in the browser inbox"
            .into(),
        Some(request),
    )?;
    let granted = Deadline::from_request_row(BROWSER_ACTION_SECONDS);
    let grace_audit: Bind = |targets, context| {
        Ok(vec![
            ("action", "membership.grace_extended".to_string()),
            ("target_type", "user".to_string()),
            ("target_id", targets.operator.clone()),
            ("since_ms", context.step_started_unix_ms.to_string()),
        ])
    };
    grant.effects = vec![
        effect(
            "override_lasts_48_hours",
            "the override row expires 48 h after it was granted",
            database_probe(targets, GRACE_OVERRIDE, operator_in_partner, |text, _| {
                let row = first_row(text).unwrap_or_default();
                match (millis(row.first()), millis(row.get(1))) {
                    (Some(created), Some(expires))
                        if expires.abs_diff(created + 48 * HOUR_MS) <= 60_000 =>
                    {
                        ProbeVerdict::Satisfied(ProbeVerdict::satisfied(format!(
                            "override {created} to {expires}"
                        )))
                    }
                    (Some(created), Some(expires)) => ProbeVerdict::Contradicted(format!(
                        "override lasts {} ms",
                        expires.saturating_sub(created)
                    )),
                    _ => ProbeVerdict::Pending("no override row".into()),
                }
            }),
            granted,
            ADMIN_OVERRIDE,
        )?,
        effect(
            "grace_extended_audited",
            "a membership.grace_extended audit row",
            database_probe(targets, AUDIT_ROWS_SINCE, grace_audit, audit_row),
            granted,
            ADMIN_OVERRIDE,
        )?,
        effect(
            "override_shown",
            "/me shows the override active and the admin role",
            page_shows(
                &["\"membership_override_active\":true", "\"role\":\"admin\""],
                None,
            ),
            granted,
            ADMIN_OVERRIDE,
        )?,
    ];
    Ok(vec![aging, grant])
}

/// Step 6: the drop-in removed with the API restarted, then the partner role restored.
pub(super) fn recovery_steps(targets: &DiscordTargets) -> Result<Vec<Step>> {
    let mut removal = step(
        "outage_removal",
        StepKind::HostAction(outage_dropin::remove(&targets.settings)),
        "none: the harness removes the drop-in and restarts the API".into(),
        None,
    )?;
    removal.effects = vec![effect(
        "fresh_verification",
        "the main snapshot is verified again after the restart",
        database_probe(
            targets,
            MAIN_SNAPSHOT,
            operator_in_partner,
            |text, context| match snapshot(text) {
                Some(row)
                    if row.status == "member"
                        && row.last_error.is_empty()
                        && row.verified_ms >= context.step_started_unix_ms =>
                {
                    ProbeVerdict::Satisfied(
                        ProbeVerdict::satisfied(format!("member, verified_at {}", row.verified_ms))
                            .at(row.verified_ms),
                    )
                }
                Some(row) => ProbeVerdict::Pending(format!(
                    "{} verified {} last_error {:?}",
                    row.status, row.verified_ms, row.last_error
                )),
                None => ProbeVerdict::Pending("no main snapshot".into()),
            },
        ),
        Deadline::from_step_start(RESTART_EFFECT_SECONDS),
        OUTAGE_RECOVERY,
    )?];
    let role = targets.partner_role.clone();
    let request = RequestPredicate {
        description: "the first bot read that shows the partner role back".into(),
        probe: member_probe(
            targets,
            GuildScope::Partner,
            move |text, _| match member_read(text) {
                Some(read) if read.holds_role(&role) == Some(true) => ProbeVerdict::Satisfied(
                    ProbeVerdict::satisfied(format!(
                        "bot read answered at {} shows role {role}",
                        read.answered_at_unix_ms
                    ))
                    .at(read.answered_at_unix_ms),
                ),
                Some(read) => {
                    ProbeVerdict::Pending(format!("bot read {}, role not held", read.outcome))
                }
                None => ProbeVerdict::Pending("no member read line".into()),
            },
        ),
    };
    let mut restore = step(
        "partner_role_restore",
        StepKind::ChromeAction,
        format!(
            "in Discord, give role {} back to member {} in the partner guild {}",
            targets.partner_role, targets.operator, targets.partner_guild
        ),
        Some(request),
    )?;
    let with_role: Bind = |targets, _| {
        Ok(vec![
            ("discord_id", targets.operator.clone()),
            ("partner_guild_id", targets.partner_guild.clone()),
            ("role_id", targets.partner_role.clone()),
        ])
    };
    restore.effects = vec![effect(
        "partner_role_synced",
        "the platform's copy of the partner roles holds the role again",
        database_probe(
            targets,
            PARTNER_ROLE_ROWS,
            with_role,
            |text, _| match millis(first_row(text).unwrap_or_default().first()) {
                Some(count) if count > 0 => {
                    ProbeVerdict::Satisfied(ProbeVerdict::satisfied("partner role held again"))
                }
                _ => ProbeVerdict::Pending("partner role not synced yet".into()),
            },
        ),
        Deadline::from_request_row(RESTART_EFFECT_SECONDS),
        OUTAGE_RECOVERY,
    )?];
    Ok(vec![removal, restore])
}
