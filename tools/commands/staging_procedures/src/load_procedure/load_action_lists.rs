//! The load procedure's numbered real actions, its recovery actions, and its preflight checks.
//!
//! **Role:** what `staging action-list load [--recovery]` prints and what `staging preflight`
//! checks for the load procedure: the trusted-proxy range, the empty bot token, the source
//! addresses, and the keying probe (the refreshes from the workstation, then their buckets).
//!
//! **Position:** called by `LoadProcedure`'s `action_list`, `recovery_action_list` and
//! `preflight_checks`; rendered by `operator_coordination/action_list.rs`, run by
//! `support_commands/preflight.rs`.
//!
//! **Signals & state:** none; values built from the settings and the committed data.
//!
//! **Invariants:** every action names the exact command it runs, in the order it happens; the
//! host checks only read; the keying check's local refreshes run before the bucket check reads
//! their rows.

use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

use staging_load_plan::verify_source_addresses;

use super::committed_load_data::CommittedLoadData;
use super::load_fixture_orchestration::host_account_file;
use super::load_preconditions::{
    env_key_presence, keying_answers_hold, send_keying_refreshes, strict_buckets_hold,
    trusted_proxies_hold, trusted_proxies_read,
};
use super::load_queries::STRICT_RATE_LIMIT_BUCKETS;
use super::load_steps::source_addresses;
use super::workstation_load::WorkstationLoad;
use crate::operator_coordination::action_list::PlannedAction;
use crate::remote_observers::database_reader::select;
use crate::staging_settings::StagingSettings;
use crate::support_commands::preflight::PreflightCheck;

/// The numbered actions of one load recording.
pub(crate) fn action_list(
    settings: &StagingSettings,
    data: Option<&CommittedLoadData>,
    token_file: Option<&Path>,
) -> Vec<PlannedAction> {
    let token = token_file.map_or_else(
        || "<token file>".to_string(),
        |path| path.display().to_string(),
    );
    let secondary: Vec<&String> = settings.load_source_addresses.iter().skip(1).collect();
    let (accounts, role, mission) = data.map_or_else(
        || (0, String::new(), String::new()),
        |data| {
            let population = &data.population;
            (
                population.accounts,
                population.discord_role.clone(),
                population.fixture_events.mission_title.clone(),
            )
        },
    );
    let mut add = PlannedAction::new(
        "operator",
        "add the secondary source addresses to the workstation's interface",
    );
    let mut remove = PlannedAction::new("operator", "remove the secondary source addresses");
    for address in &secondary {
        add = add.detail(format!("sudo ip addr add {address}/24 dev <interface>"));
        remove = remove.detail(format!("sudo ip addr del {address}/24 dev <interface>"));
    }
    vec![
        PlannedAction::new("harness", "back up the staging database before the seeding")
            .detail("cargo xtask staging backup --label before-load"),
        add,
        PlannedAction::new("harness", "check the load preconditions")
            .detail("cargo xtask staging preflight")
            .detail("TRUSTED_PROXIES, an empty DISCORD_BOT_TOKEN, the source addresses, and one invalid refresh per address answered 401 with its strict bucket"),
        PlannedAction::new("harness", "seed the synthetic population, then the fixture events")
            .detail(format!("cargo xtask staging seed-load --token-file {token}"))
            .detail(format!("staging-fixtures seed-load-population --accounts {accounts} --role {role} --account-file {}", host_account_file(settings)))
            .detail(format!("staging-fixtures seed-load-fixture-events --mission <id of the live mission {mission:?}>"))
            .detail(format!("the account file moves into {token} (mode 600); the host copy is removed")),
        PlannedAction::new("harness", "record the load receipt")
            .detail(format!("cargo xtask staging load --record --token-file {token}"))
            .detail("reads the population, sends the keying refreshes, reads the game-route counters, drives the member load with a heartbeat census every minute, reads the counters again"),
        PlannedAction::new("harness", "clean the fixture events, then the population, then any host account file")
            .detail("cargo xtask staging clean-load"),
        PlannedAction::new("operator", "delete the token file").detail(format!("rm -f {token}")),
        remove,
    ]
}

/// The actions that put the host back after a stopped load run.
pub(crate) fn recovery_action_list(token_file: Option<&Path>) -> Vec<PlannedAction> {
    let token = token_file.map_or_else(
        || "<token file>".to_string(),
        |path| path.display().to_string(),
    );
    vec![
        PlannedAction::new(
            "harness",
            "delete the synthetic population and the fixture events",
        )
        .detail("cargo xtask staging clean-load"),
        PlannedAction::new("operator", "delete the token file").detail(format!("rm -f {token}")),
    ]
}

/// The load procedure's preconditions.
pub(crate) fn preflight_checks(
    settings: &StagingSettings,
    expected_addresses: Option<u32>,
    workstation: &Arc<dyn WorkstationLoad>,
) -> Vec<PreflightCheck> {
    let addresses: Vec<IpAddr> = source_addresses(settings);
    let env_file = settings.api_env_file();
    let origin = settings.load_target_origin.clone();
    let named = settings.load_source_addresses.len();
    let mut checks = Vec::new();
    let sources = addresses.clone();
    checks.push(PreflightCheck::host(
        "load: TRUSTED_PROXIES covers the Caddy peer and no source address",
        trusted_proxies_read(&env_file),
        move |output| trusted_proxies_hold(output.stdout.trim(), &sources),
    ));
    checks.push(PreflightCheck::host(
        "load: DISCORD_BOT_TOKEN is empty",
        env_key_presence(&env_file, "DISCORD_BOT_TOKEN"),
        |output| match output.stdout.trim() {
            "empty" => Ok("the bot token is empty".to_string()),
            seen => Err(format!(
                "the bot token is {seen}; the reconciler would demote every synthetic account"
            )),
        },
    ));
    let local = addresses.clone();
    checks.push(PreflightCheck::local("load: the source addresses belong to this workstation", move || {
        if local.len() != named || expected_addresses.is_some_and(|count| count as usize != local.len()) {
            return Err(format!("TBD_LOAD_SOURCE_ADDRESSES names {named} addresses ({} parse); the workload needs {expected_addresses:?}", local.len()));
        }
        verify_source_addresses(&local)
            .map(|()| format!("{} source addresses bind", local.len()))
            .map_err(|error| format!("{error:#}"))
    }));
    let keying = Arc::clone(workstation);
    let keyed = addresses.clone();
    checks.push(PreflightCheck::local(
        "load: keying probe: one invalid refresh per source address is answered 401",
        move || {
            let origin = origin
                .clone()
                .ok_or_else(|| "TBD_LOAD_TARGET_ORIGIN is not set".to_string())?;
            keying_answers_hold(&send_keying_refreshes(&keying, &origin, &keyed), &keyed)
        },
    ));
    match select(
        &settings.database_container,
        &STRICT_RATE_LIMIT_BUCKETS,
        &[],
    ) {
        Ok(command) => checks.push(PreflightCheck::host(
            "load: keying probe: one strict bucket per source address",
            command,
            move |output| strict_buckets_hold(&output.stdout, &addresses, 0),
        )),
        Err(error) => checks.push(PreflightCheck::local(
            "load: keying probe: one strict bucket per source address",
            move || Err(format!("{error:#}")),
        )),
    }
    checks
}
