//! The load procedure's plan: its ten declared cases and the five steps that decide them.
//!
//! **Role:** builds the [`ProcedurePlan`] of `staging_load`: `population` (the seeded accounts,
//! the fixture events and the token file), `keying_probe` (one invalid refresh per source address
//! and its strict buckets), `game_operations_baseline` (the API's counters before the load),
//! `member_load` (the engine's report and the heartbeat census) and `game_operations_delta` (the
//! counters after it).
//!
//! **Position:** called by `LoadProcedure::plan`; the steps run in `load_run.rs`, which performs
//! the workstation actions of `population`, `keying_probe` and `member_load` before their
//! effects are judged.
//!
//! **Signals & state:** none; the probes capture the settings and the committed data they need.
//!
//! **Invariants:** every declared case is decided by at least one effect; host probes only read;
//! the member load's effects count from the step's start past the ramp and the measured window,
//! and the hard stop leaves room for the reads around it.

use std::net::IpAddr;

use anyhow::Result;

use super::committed_load_data::CommittedLoadData;
use super::game_operations::{CENSUS_MEASUREMENT, judge_baseline, judge_census, judge_delta};
use super::load_preconditions::{
    ACCOUNT_FILE_MEASUREMENT, KEYING_MEASUREMENT, judge_account_file, judge_fixture_events,
    judge_keying_answers, judge_population, judge_strict_buckets,
};
use super::load_queries::{LOAD_FIXTURE_EVENTS, STRICT_RATE_LIMIT_BUCKETS, SYNTHETIC_POPULATION};
use super::load_report_judges::{
    LoadThresholds, REPORT_MEASUREMENT, ReportJudge, concurrency, member_accounts, p95_json_reads,
    p95_json_writes, refresh_pacing, report_judge, sustained_rate, zero_unexpected_errors,
};
use crate::commands::staging::procedure_runner::procedure::ProcedurePlan;
use crate::commands::staging::procedure_runner::step::{
    Deadline, DeclaredCase, EffectPredicate, Probe, Step, StepContext, StepId, StepKind,
};
use crate::commands::staging::remote_observers::database_reader::{CommittedQuery, select};
use crate::commands::staging::remote_observers::metrics_reader::exposition;
use crate::commands::staging::staging_settings::StagingSettings;
use crate::verifications::api_readiness::operational_recording::CaseName;

/// The step that reads the seeded population.
pub(crate) const POPULATION_STEP: &str = "population";
/// The step that sends the keying refreshes.
pub(crate) const KEYING_STEP: &str = "keying_probe";
/// The step that reads the API's counters before the load.
pub(crate) const BASELINE_STEP: &str = "game_operations_baseline";
/// The step that runs the member load.
pub(crate) const MEMBER_LOAD_STEP: &str = "member_load";
/// The step that reads the API's counters after the load.
pub(crate) const DELTA_STEP: &str = "game_operations_delta";
/// The declared cases, in declaration order.
pub(crate) const LOAD_CASES: [&str; 10] = [
    "population_seeded",
    "refresh_paced",
    "sustained_rate",
    "concurrency",
    "member_accounts",
    "zero_unexpected_errors",
    "p95_json_reads",
    "p95_json_writes",
    "game_servers_heartbeating",
    "game_operations_measured",
];
/// Seconds between two polls of a pending probe.
const POLL_INTERVAL_SECONDS: u64 = 5;
/// Seconds a read before or after the load may take to hold.
const READ_DEADLINE_SECONDS: u64 = 180;
/// Seconds past the ramp and the measured window the member load's effects may take.
const MEMBER_LOAD_MARGIN_SECONDS: u64 = 600;
/// Seconds of the hard stop past the ramp and the measured window.
const HARD_STOP_MARGIN_SECONDS: u64 = 1_200;

/// The member-load effects judged on the report: effect id, case, judge.
const REPORT_EFFECTS: [(&str, &str, ReportJudge); 7] = [
    ("sustained_rate", "sustained_rate", sustained_rate),
    ("concurrency", "concurrency", concurrency),
    ("member_accounts", "member_accounts", member_accounts),
    (
        "zero_unexpected_errors",
        "zero_unexpected_errors",
        zero_unexpected_errors,
    ),
    ("p95_json_reads", "p95_json_reads", p95_json_reads),
    ("p95_json_writes", "p95_json_writes", p95_json_writes),
    ("refresh_pacing", "refresh_paced", refresh_pacing),
];

/// The source addresses of `settings` that parse.
pub(crate) fn source_addresses(settings: &StagingSettings) -> Vec<IpAddr> {
    settings
        .load_source_addresses
        .iter()
        .filter_map(|address| address.trim().parse().ok())
        .collect()
}

/// The plan of one `staging_load` run.
pub(crate) fn load_plan(
    data: &CommittedLoadData,
    settings: &StagingSettings,
) -> Result<ProcedurePlan> {
    let workload = &data.workload;
    let addresses = source_addresses(settings);
    let run_seconds = (workload.ramp_seconds + workload.measured_seconds).ceil() as u64;
    let container = settings.database_container.clone();
    let read = move |query: CommittedQuery| {
        let container = container.clone();
        move |_: &StepContext<'_>| select(&container, &query, &[])
    };
    let (env_file, api_origin) = (settings.api_env_file(), settings.api_origin.clone());
    let metrics = move |_: &StepContext<'_>| Ok(exposition(&env_file, &api_origin));
    let population = step(
        POPULATION_STEP,
        "nobody acts: the harness reads the seeded accounts, the [Load fixture] events and the \
         token file's Discord ids",
        vec![
            effect(
                "synthetic_accounts",
                "the synthetic population",
                Probe::host(read(SYNTHETIC_POPULATION), judge_population(data)?),
                READ_DEADLINE_SECONDS,
                "population_seeded",
            )?,
            effect(
                "fixture_events",
                "the fixture events and their slots",
                Probe::host(read(LOAD_FIXTURE_EVENTS), judge_fixture_events(data)),
                READ_DEADLINE_SECONDS,
                "population_seeded",
            )?,
            effect(
                "token_file",
                "the token file",
                Probe::measurement(ACCOUNT_FILE_MEASUREMENT, judge_account_file(data)?),
                READ_DEADLINE_SECONDS,
                "population_seeded",
            )?,
        ],
    )?;
    let keying = step(
        KEYING_STEP,
        "the harness sends one invalid refresh from each source address; each must be answered \
         401 and key its own strict bucket",
        vec![
            effect(
                "refresh_answers",
                "an invalid refresh answered 401 per source address",
                Probe::measurement(KEYING_MEASUREMENT, judge_keying_answers(addresses.clone())),
                READ_DEADLINE_SECONDS,
                "refresh_paced",
            )?,
            effect(
                "strict_buckets",
                "one strict bucket per source address",
                Probe::host(
                    read(STRICT_RATE_LIMIT_BUCKETS),
                    judge_strict_buckets(addresses),
                ),
                READ_DEADLINE_SECONDS,
                "refresh_paced",
            )?,
        ],
    )?;
    let baseline = step(
        BASELINE_STEP,
        "nobody acts: the harness reads the API's game-route counters before the load",
        vec![effect(
            "metrics_baseline",
            "the game-route counters before the load",
            Probe::host(metrics.clone(), judge_baseline),
            READ_DEADLINE_SECONDS,
            "game_operations_measured",
        )?],
    )?;
    let mut member_effects = Vec::new();
    for (id, case, judge) in REPORT_EFFECTS {
        member_effects.push(effect(
            id,
            &format!("the load engine's report: {id}"),
            Probe::measurement(
                REPORT_MEASUREMENT,
                report_judge(judge, LoadThresholds::ACCEPTANCE),
            ),
            run_seconds + MEMBER_LOAD_MARGIN_SECONDS,
            case,
        )?);
    }
    member_effects.push(effect(
        "heartbeat_census",
        "every fleet server heartbeating in each census sample",
        Probe::measurement(CENSUS_MEASUREMENT, judge_census),
        run_seconds + MEMBER_LOAD_MARGIN_SECONDS,
        "game_servers_heartbeating",
    )?);
    let member_load = step(
        MEMBER_LOAD_STEP,
        &format!(
            "the harness drives {} clients from {} source addresses at {} requests a second for a \
             {} s ramp and {} measured seconds, and reads the heartbeat census every minute",
            workload.clients,
            workload.source_address_count,
            workload.requests_per_second,
            workload.ramp_seconds,
            workload.measured_seconds
        ),
        member_effects,
    )?;
    let delta = step(
        DELTA_STEP,
        "nobody acts: the harness reads the API's game-route counters after the load",
        vec![effect(
            "metrics_delta",
            "the game-route requests since the baseline",
            Probe::host(metrics, judge_delta),
            READ_DEADLINE_SECONDS,
            "game_operations_measured",
        )?],
    )?;
    Ok(ProcedurePlan {
        declared_cases: LOAD_CASES
            .iter()
            .map(|case| DeclaredCase::runs(case))
            .collect::<Result<_>>()?,
        steps: vec![population, keying, baseline, member_load, delta],
        hard_stop_seconds: run_seconds + HARD_STOP_MARGIN_SECONDS,
        poll_interval_seconds: POLL_INTERVAL_SECONDS,
    })
}

fn step(id: &str, instruction: &str, effects: Vec<EffectPredicate>) -> Result<Step> {
    Ok(Step {
        id: StepId::new(id)?,
        kind: StepKind::Observation,
        instruction: instruction.to_string(),
        request: None,
        effects,
    })
}

fn effect(
    id: &str,
    description: &str,
    probe: Probe,
    deadline_seconds: u64,
    case: &str,
) -> Result<EffectPredicate> {
    Ok(EffectPredicate {
        id: id.to_string(),
        description: description.to_string(),
        probe,
        deadline: Deadline::from_step_start(deadline_seconds),
        case: CaseName::new(case)?,
    })
}
