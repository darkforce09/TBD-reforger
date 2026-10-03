//! The Discord procedure's numbered real actions: the list the operator approves before a
//! recording, and the recovery list that puts the host and the guild back after a stopped run.
//!
//! **Role:** renders every action the run takes (the orchestrator's browser actions, the
//! harness's drop-in, aging and bucket spend with their exact commands) and the three recovery
//! actions (remove the drop-in and restart the API, restore the operator's partner role, delete
//! the partner event).
//!
//! **Position:** called by `DiscordProcedure::action_list` and
//! `DiscordProcedure::recovery_action_list` in `mod.rs`, which `staging_dispatch.rs` prints for
//! `staging action-list discord [--recovery]`.
//!
//! **Signals & state:** none; pure builders over the settings.
//!
//! **Invariants:** the list names each host command exactly as the run sends it (the same
//! builders make both), so anything the run does off the list is visibly unapproved; the bot
//! member reads the harness polls are named with the actions they time; no action touches the
//! operator's Command Staff role or the main guild's roles.

use super::partner_steps::PARTNER_EVENT_TITLE;
use super::rate_limit_steps::{self, BUCKET_HOLD_SECONDS, BUCKET_MAX_REQUESTS, SPEND_LEAD_MS};
use crate::operator_coordination::action_list::PlannedAction;
use crate::remote_actions::host_fixture_commands::age_membership_snapshot;
use crate::remote_actions::outage_dropin::{self, BLACKHOLE_PROXY};
use crate::remote_observers::remote_command::RemoteCommand;
use crate::staging_settings::StagingSettings;

use super::outage_steps::STAGED_SNAPSHOT_AGE_HOURS;

const BROWSER: &str = "orchestrator (browser)";
const HARNESS: &str = "harness";

/// The value a detail line shows for a `deploy.env` key that is not set.
fn setting(value: &Option<String>, key: &str) -> String {
    value.clone().unwrap_or_else(|| format!("<{key} unset>"))
}

/// `command` as detail lines: the command line, then each line of its stdin script.
fn command_details(action: PlannedAction, command: &RemoteCommand) -> PlannedAction {
    let mut action = action.detail(format!("on the host: {}", command.command_line));
    if let Some(script) = &command.stdin {
        for line in script.lines().filter(|line| !line.trim().is_empty()) {
            action = action.detail(format!("  {line}"));
        }
    }
    action
}

/// The actions of one recorded run, in the order they happen.
pub(super) fn run_actions(settings: &StagingSettings) -> Vec<PlannedAction> {
    let operator = setting(
        &settings.operator_discord_id,
        "TBD_STAGING_OPERATOR_DISCORD_ID",
    );
    let partner_guild = setting(&settings.partner_guild_id, "TBD_STAGING_PARTNER_GUILD_ID");
    let partner_role = setting(&settings.partner_role_id, "TBD_STAGING_PARTNER_ROLE_ID");
    let aging = age_membership_snapshot(settings, &operator, STAGED_SNAPSHOT_AGE_HOURS);
    let mut actions = vec![
        PlannedAction::new(
            BROWSER,
            "create the partner-only event and its partner group, register, register again",
        )
        .detail(format!(
            "Event Manager: create \"{PARTNER_EVENT_TITLE}\" restricted to the partner group of \
             guild {partner_guild} with role {partner_role}"
        ))
        .detail("event page: register the operator; the answer is MEMBERSHIP_VERIFICATION_REQUIRED")
        .detail("save that answer's body into the step's browser inbox entry; register again"),
        PlannedAction::new(
            BROWSER,
            "remove the operator's partner role in the partner guild",
        )
        .detail(format!(
            "Discord, partner guild {partner_guild}: remove role {partner_role} from member {operator}"
        ))
        .detail("the harness polls the bot's read of that member every poll interval until the role is gone"),
    ];
    actions.push(command_details(
        PlannedAction::new(
            HARNESS,
            format!(
                "install the outage drop-in (HTTPS_PROXY={BLACKHOLE_PROXY}) and restart the API"
            ),
        ),
        &outage_dropin::install(settings),
    ));
    actions.push(
        PlannedAction::new(BROWSER, "read /me, the staleness banner and the events page")
            .detail("save the /api/v1/me response body, the banner text and the /api/v1/events request line with its 200 into the step's inbox entry"),
    );
    actions.push(command_details(
        PlannedAction::new(
            HARNESS,
            format!(
                "age the operator's main-guild snapshot to {STAGED_SNAPSHOT_AGE_HOURS} h (a staged precondition)"
            ),
        ),
        &aging,
    ));
    actions.push(
        PlannedAction::new(BROWSER, "read /me, which shows the operator as a guest")
            .detail("save the /api/v1/me response body into the step's inbox entry"),
    );
    actions.push(
        PlannedAction::new(
            BROWSER,
            "grant the 48 h grace override from the banner form, then read /me",
        )
        .detail(format!(
            "override for member {operator} in the main guild, 48 hours, with a reason"
        ))
        .detail("save the /api/v1/me response body into the step's inbox entry"),
    );
    actions.push(command_details(
        PlannedAction::new(HARNESS, "remove the outage drop-in and restart the API"),
        &outage_dropin::remove(settings),
    ));
    actions.push(
        PlannedAction::new(BROWSER, "restore the operator's partner role in the partner guild")
            .detail(format!(
                "Discord, partner guild {partner_guild}: give role {partner_role} back to member {operator}"
            ))
            .detail("the harness polls the bot's read of that member until the role is back"),
    );
    let spend = PlannedAction::new(
        HARNESS,
        format!(
            "spend the main guild's Get Guild Member bucket from {SPEND_LEAD_MS} ms before the operator's next refresh, holding {BUCKET_HOLD_SECONDS} s, at most {BUCKET_MAX_REQUESTS} requests"
        ),
    );
    actions.push(match rate_limit_steps::bucket_spend(settings) {
        Ok(command) => command_details(spend, &command),
        Err(error) => spend.detail(format!("cannot be built: {error:#}")),
    });
    actions.push(
        PlannedAction::new(BROWSER, "delete the partner event")
            .detail(format!("Event Manager: delete \"{PARTNER_EVENT_TITLE}\"")),
    );
    actions
}

/// The actions that put everything back after a stopped run.
pub(super) fn recovery_actions(settings: &StagingSettings) -> Vec<PlannedAction> {
    let operator = setting(
        &settings.operator_discord_id,
        "TBD_STAGING_OPERATOR_DISCORD_ID",
    );
    let partner_guild = setting(&settings.partner_guild_id, "TBD_STAGING_PARTNER_GUILD_ID");
    let partner_role = setting(&settings.partner_role_id, "TBD_STAGING_PARTNER_ROLE_ID");
    vec![
        command_details(
            PlannedAction::new(
                HARNESS,
                "remove the HTTPS_PROXY outage drop-in and restart the API",
            ),
            &outage_dropin::remove(settings),
        )
        .detail("then `cargo xtask staging status` shows the drop-in absent"),
        PlannedAction::new(
            BROWSER,
            "restore the operator's partner role in the partner guild",
        )
        .detail(format!(
            "Discord, partner guild {partner_guild}: give role {partner_role} back to member {operator}"
        )),
        PlannedAction::new(BROWSER, "delete the partner event")
            .detail(format!("Event Manager: delete \"{PARTNER_EVENT_TITLE}\" if it still exists")),
    ]
}
