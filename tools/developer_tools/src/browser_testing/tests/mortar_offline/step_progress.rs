use super::*;

#[test]
fn mortar_offline_a_fresh_run_fails_at_the_preflight() {
    assert_eq!(StepProgress::default().failing_step(), "preflight");
}

#[test]
fn mortar_offline_a_failure_names_the_step_after_the_last_passed_one() {
    let steps = StepProgress::default();
    for step in &STEPS[..13] {
        steps.pass(step);
    }
    assert_eq!(steps.failing_step(), "solution_matches_native");
    steps.pass("solution_matches_native");
    assert_eq!(steps.failing_step(), "tiles_drew");
}

#[test]
fn mortar_offline_a_run_past_the_last_step_fails_in_teardown() {
    let steps = StepProgress::default();
    steps.pass("tiles_drew");
    assert_eq!(steps.failing_step(), "teardown");
}

#[test]
#[should_panic(expected = "is not a mortar-offline step")]
fn mortar_offline_an_unknown_step_is_a_programming_error() {
    StepProgress::default().pass("unknown");
}

#[test]
fn mortar_offline_the_api_down_steps_run_after_the_pack_and_before_the_listener_stops() {
    let index = |step: &str| STEPS.iter().position(|s| *s == step).expect(step);
    let api_down = [
        "api_down_behind_proxy",
        "catalog_list_from_worker_cache",
        "catalog_from_saved_copy",
        "pack_kept_ready",
        "api_down_solution_matches_native",
    ];
    for pair in api_down.windows(2) {
        assert_eq!(index(pair[0]) + 1, index(pair[1]), "{pair:?}");
    }
    assert_eq!(index("worker_active") + 1, index(api_down[0]));
    assert_eq!(index(api_down[4]) + 1, index("listener_stopped"));
    let unique: std::collections::HashSet<&str> = STEPS.iter().copied().collect();
    assert_eq!(unique.len(), STEPS.len());
}
