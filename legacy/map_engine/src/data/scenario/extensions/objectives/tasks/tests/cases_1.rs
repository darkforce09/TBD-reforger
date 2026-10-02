//! Role: Domain regression cases.
//! Position: `mission/extensions/objectives/tasks/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn a_three_tier_block_parses() {
    let got = parse(&three_tiers()).expect("parses");
    assert_eq!(got.len(), 3);
    assert_eq!(got[0].tier, TaskTier::Primary);
    assert_eq!(got[1].tier, TaskTier::Secondary);
    assert_eq!(got[2].tier, TaskTier::Optional);
    assert!(got.iter().all(|t| t.state == TaskState::Assigned));
    assert_eq!(got[0].trigger_id.as_deref(), Some("trg-hill"));
    assert_eq!(got[0].marker_id.as_deref(), Some("attack"));
    assert_eq!(
        got[2].description.as_deref(),
        Some("No trigger — stays assigned.")
    );
    assert!(got[2].trigger_id.is_none());
}

#[test]
fn assigned_to_succeeded_is_legal() {
    assert_eq!(
        transition(TaskState::Assigned, TaskState::Succeeded).expect("legal"),
        TaskState::Succeeded
    );
}

#[test]
fn assigned_to_failed_is_legal() {
    assert_eq!(
        transition(TaskState::Assigned, TaskState::Failed).expect("legal"),
        TaskState::Failed
    );
}

#[test]
fn succeeded_to_assigned_is_illegal() {
    let err = transition(TaskState::Succeeded, TaskState::Assigned)
        .expect_err("succeeded → assigned must be refused");
    assert!(
        err.contains("illegal task transition"),
        "the refusal must name the class of mistake: {err}"
    );
    assert!(
        err.contains("succeeded"),
        "the refusal must name the from-state: {err}"
    );
    assert!(
        err.contains("assigned"),
        "the refusal must name the to-state: {err}"
    );
    assert!(
        !is_legal_transition(TaskState::Succeeded, TaskState::Assigned),
        "the table itself must not list succeeded → assigned"
    );
}

#[test]
fn every_other_pair_is_illegal() {
    let states = [TaskState::Assigned, TaskState::Succeeded, TaskState::Failed];
    for from in states {
        for to in states {
            let legal = is_legal_transition(from, to);
            if from == TaskState::Assigned && matches!(to, TaskState::Succeeded | TaskState::Failed)
            {
                assert!(legal, "{from:?} → {to:?} must be legal");
            } else {
                assert!(!legal, "{from:?} → {to:?} must be illegal");
                transition(from, to).expect_err("refused");
            }
        }
    }
}

#[test]
fn a_duplicate_id_is_refused() {
    let err = parse(&json!([
        {"id": "t1", "title": "A", "tier": "primary", "state": "assigned"},
        {"id": "t1", "title": "B", "tier": "secondary", "state": "assigned"}
    ]))
    .expect_err("duplicate id");
    assert!(err.contains("unique"), "{err}");
    assert!(err.contains("t1"), "{err}");
}

#[test]
fn an_unknown_property_is_refused() {
    let err = parse(&json!([{
        "id": "t1", "title": "A", "tier": "primary", "state": "assigned", "endOn": ["time_limit"]
    }]))
    .expect_err("endOn is not a task field");
    assert!(err.contains("endOn"), "{err}");
    assert!(err.contains("additionalProperties"), "{err}");
}

#[test]
fn a_blank_optional_is_refused_rather_than_stored() {
    let err = parse(&json!([{
        "id": "t1", "title": "A", "tier": "primary", "state": "assigned", "triggerId": "  "
    }]))
    .expect_err("blank triggerId");
    assert!(err.contains("blank"), "{err}");
}

#[test]
fn a_tier_outside_the_vocabulary_is_refused() {
    let err = parse(&json!([{
        "id": "t1", "title": "A", "tier": "main", "state": "assigned"
    }]))
    .expect_err("main is not a tier");
    assert!(err.contains("main"), "{err}");
    assert!(err.contains("primary"), "{err}");
}

#[test]
fn not_an_array_is_refused() {
    let err = parse(&json!({"id": "t1"})).expect_err("object is not an array");
    assert!(err.contains("array"), "{err}");
}

#[test]
fn tasks_is_registered_and_not_document_modelled() {
    assert!(
        is_authored_block("tasks"),
        "T-936.2's row must be in AUTHORED_BLOCKS or the carrier never emits it"
    );
    assert!(
        !crate::data::scenario::extensions::DOCUMENT_OWNED_BLOCKS.contains(&"tasks"),
        "tasks is optional — it rides ExtensionBlocks, it does not get a typed ModMission field"
    );
}

#[test]
fn copy_promotes_tasks_verbatim_from_the_environment_bag() {
    let tasks = three_tiers();
    let env = json!({"weather": "clear", "tasks": tasks});
    let mut dst = Map::new();
    let copied = copy_authored_blocks(&env, &mut dst);
    assert!(
        copied.contains(&"tasks"),
        "tasks must leave the bag: {copied:?}"
    );
    assert_eq!(dst["tasks"], tasks, "verbatim");
    assert!(
        !dst.contains_key("weather"),
        "the bag's own keys stay in the bag"
    );
}

#[test]
fn from_payload_carries_a_valid_block_and_refuses_a_malformed_one() {
    let (carried, refusals) = ExtensionBlocks::from_payload(&json!({"tasks": three_tiers()}));
    assert!(refusals.is_empty(), "{refusals:?}");
    assert_eq!(carried.get("tasks"), Some(&three_tiers()));

    let (carried, refusals) = ExtensionBlocks::from_payload(&json!({
        "tasks": [{"id": "t1", "title": "A", "tier": "primary", "state": "nope"}]
    }));
    assert!(carried.is_empty(), "a refused block must not ride the wire");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].0, "tasks");
    assert!(refusals[0].1.contains("nope"), "{}", refusals[0].1);
}

#[test]
fn a_three_tier_mission_copies_to_the_payload_root() {
    let tasks = three_tiers();
    let p = compile_env_with_tasks(&tasks);
    assert_eq!(
        p["tasks"], tasks,
        "AUTHORED_BLOCKS must promote tasks out of the env bag: {p:#}"
    );
    assert_eq!(p["tasks"].as_array().expect("array").len(), 3);
    assert_eq!(p["tasks"][0]["tier"], "primary");
    assert_eq!(p["tasks"][1]["tier"], "secondary");
    assert_eq!(p["tasks"][2]["tier"], "optional");

    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert_eq!(carried.get("tasks"), Some(&tasks));
}

#[test]
fn an_unauthored_payload_still_omits_the_tasks_key() {
    let p = compile_payload(
        &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}}).to_string(),
        "{}",
        false,
    );
    assert!(
        p.get("tasks").is_none(),
        "parity: no tasks authored ⇒ no tasks key: {p:#}"
    );
    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert!(carried.get("tasks").is_none());
}

#[test]
fn an_unlisted_environment_key_is_not_promoted() {
    let env = json!({"weather": "clear", "notAnAuthoredBlock": []});
    let mut dst = Map::new();
    let copied = copy_authored_blocks(&env, &mut dst);
    assert!(!copied.contains(&"notAnAuthoredBlock"), "{copied:?}");
    assert!(
        !dst.contains_key("notAnAuthoredBlock"),
        "an unlisted key stays parked: {dst:?}"
    );
}

#[test]
fn a_legal_schedule_round_trips() {
    let got = parse(&timed(600, 300)).expect("parses");
    let sched = got[0].schedule.expect("schedule");
    assert_eq!(sched.start_after_s, 600);
    assert_eq!(sched.window_s, 300);
}

#[test]
fn an_unscheduled_task_still_parses() {
    let got = parse(&json!([{
        "id": "t1", "title": "A", "tier": "primary", "state": "assigned"
    }]))
    .expect("parses");
    assert!(got[0].schedule.is_none());
}

#[test]
fn a_zero_window_is_refused() {
    let err = parse(&timed(0, 0)).expect_err("window 0");
    assert!(err.contains("windowS"), "{err}");
    assert!(err.contains("> 0"), "{err}");
    assert!(!window_is_legal(0), "the predicate itself must refuse 0");
    validate_schedule(0, 0, None).expect_err("window 0");
}

#[test]
fn a_negative_window_is_refused() {
    let err = parse(&timed(10, -1)).expect_err("negative window");
    assert!(err.contains("windowS"), "{err}");
}

#[test]
fn a_negative_start_is_refused() {
    let err = parse(&timed(-1, 60)).expect_err("negative start");
    assert!(err.contains("startAfterS"), "{err}");
}

#[test]
fn start_at_or_past_mission_length_is_refused() {
    validate_schedule(600, 60, Some(600)).expect_err("start == length");
    validate_schedule(601, 60, Some(600)).expect_err("start > length");
    validate_schedule(599, 60, Some(600)).expect("start inside");
    validate_schedule(0, 60, Some(5400)).expect("T+0 of a 90 min round");
    validate_schedule(100, 60, Some(0)).expect("no time limit");
    validate_schedule(100, 60, None).expect("length unknown at AUTHORED_BLOCKS");
    let err = validate_schedule(5400, 60, Some(5400)).expect_err("default round end");
    assert!(err.contains("within mission length"), "{err}");
    assert!(err.contains("5400"), "{err}");
}

#[test]
fn a_start_of_zero_is_legal() {
    let got = parse(&timed(0, 120)).expect("T+0");
    assert_eq!(got[0].schedule.unwrap().start_after_s, 0);
}

#[test]
fn a_schedule_that_is_not_an_object_is_refused() {
    let err = parse(&json!([{
        "id": "t1", "title": "A", "tier": "primary", "state": "assigned",
        "schedule": 600
    }]))
    .expect_err("number");
    assert!(err.contains("object"), "{err}");
}

#[test]
fn an_unknown_schedule_property_is_refused() {
    let err = parse(&json!([{
        "id": "t1", "title": "A", "tier": "primary", "state": "assigned",
        "schedule": {"startAfterS": 10, "windowS": 20, "endOn": "time_limit"}
    }]))
    .expect_err("endOn");
    assert!(err.contains("endOn"), "{err}");
}

#[test]
fn a_missing_window_key_is_refused() {
    let err = parse(&json!([{
        "id": "t1", "title": "A", "tier": "primary", "state": "assigned",
        "schedule": {"startAfterS": 10}
    }]))
    .expect_err("missing window");
    assert!(err.contains("windowS"), "{err}");
}
