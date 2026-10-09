//! **Role:** Domain regression cases.
//! **Position:** `mission_model::objectives::tasks::tests::cases_1` in the `mission_model` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn a_three_tier_block_parses() {
    let got = parse(&three_tiers()).expect("parses");
    assert_eq!(got.len(), 3);
    assert_eq!(got[0].tier, TaskTier::Primary);
    assert_eq!(got[1].tier, TaskTier::Secondary);
    assert_eq!(got[2].tier, TaskTier::Optional);
    assert!(got.iter().all(|t| t.state == TaskState::Assigned));
    assert_eq!(
        got[0].trigger_id.as_ref().map(|id| id.as_str()),
        Some("trg-hill")
    );
    assert_eq!(
        got[0].marker_id.as_ref().map(|id| id.as_str()),
        Some("attack")
    );
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
        err.to_string().contains("illegal task transition"),
        "the refusal must name the class of mistake: {err}"
    );
    assert!(
        err.to_string().contains("succeeded"),
        "the refusal must name the from-state: {err}"
    );
    assert!(
        err.to_string().contains("assigned"),
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
    assert!(err.to_string().contains("unique"), "{err}");
    assert!(err.to_string().contains("t1"), "{err}");
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
fn a_legal_schedule_round_trips() {
    let got = parse(&timed(600, 300)).expect("parses");
    let sched = got[0].schedule.expect("schedule");
    assert_eq!(sched.start_after_s, 600);
    assert_eq!(sched.window_s, 300);
}

#[test]
fn a_zero_window_is_refused() {
    let err = parse(&timed(0, 0)).expect_err("window 0");
    assert!(err.to_string().contains("windowS"), "{err}");
    assert!(err.to_string().contains("> 0"), "{err}");
    assert!(!window_is_legal(0), "the predicate itself must refuse 0");
    validate_schedule(0, 0, None).expect_err("window 0");
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
    assert!(err.to_string().contains("within mission length"), "{err}");
    assert!(err.to_string().contains("5400"), "{err}");
}
