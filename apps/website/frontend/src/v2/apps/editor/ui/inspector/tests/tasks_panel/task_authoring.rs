//! Tests the tasks panel subject.

    use super::*;
    use serde_json::json;

    fn pri() -> Value {
        json!({"id": "task-1", "title": "Seize", "tier": "primary", "state": "assigned"})
    }

    #[test]
    fn add_appends_a_primary_assigned_task_with_a_fresh_id() {
        let next = add_task(&[pri()]);
        assert_eq!(next.len(), 2);
        assert_eq!(next[1]["id"], "task-2");
        assert_eq!(next[1]["tier"], "primary");
        assert_eq!(next[1]["state"], "assigned");
        website_map_engine::data::scenario::tasks::validate(&Value::Array(next))
            .expect("the panel must not author a block the compile refuses");
    }

    #[test]
    fn remove_drops_one_row_and_clearing_the_last_writes_null() {
        let next = remove_task(&[pri()], 0);
        assert!(next.is_empty());
        let cleared: Value = serde_json::from_str(&env_patch(Some(&next))).expect("json");
        assert_eq!(cleared, json!({"tasks": null}));
    }

    #[test]
    fn reorder_swaps_neighbours_and_is_undoable_as_one_array() {
        let a = pri();
        let b = json!({"id": "task-2", "title": "Cache", "tier": "secondary", "state": "assigned"});
        let up = move_task(&[a.clone(), b.clone()], 1, -1);
        assert_eq!(up[0]["id"], "task-2");
        assert_eq!(up[1]["id"], "task-1");
        let down = move_task(&up, 0, 1);
        assert_eq!(down[0]["id"], "task-1");
        // Moving past the ends is a no-op, not a wrap.
        assert_eq!(move_task(&down, 0, -1), down);
        assert_eq!(move_task(&down, 1, 1), down);
    }

    #[test]
    fn the_pickers_offer_exactly_the_schema_vocabularies() {
        assert_eq!(TIERS, ["primary", "secondary", "optional"]);
        assert_eq!(STATES, ["assigned", "succeeded", "failed"]);
        for t in TIERS {
            assert_ne!(tier_label(t), "Unknown tier");
        }
        for s in STATES {
            assert_ne!(state_label(s), "Unknown state");
        }
    }

    #[test]
    fn with_field_sets_tier_trigger_and_marker_and_strips_blank_optionals() {
        let base = vec![pri()];
        let next = with_field(&base, 0, "tier", "secondary").expect("tier");
        assert_eq!(next[0]["tier"], "secondary");
        let next = with_field(&next, 0, "triggerId", "trg-hill").expect("trigger");
        assert_eq!(next[0]["triggerId"], "trg-hill");
        let next = with_field(&next, 0, "markerId", "attack").expect("marker");
        assert_eq!(next[0]["markerId"], "attack");
        let next = with_field(&next, 0, "triggerId", "  ").expect("blank optional");
        assert!(next[0].get("triggerId").is_none());
        website_map_engine::data::scenario::tasks::validate(&Value::Array(next)).expect("valid");
    }

    #[test]
    fn a_blank_title_is_refused() {
        let err = with_field(&[pri()], 0, "title", "   ").expect_err("blank title");
        assert!(err.contains("blank"), "{err}");
    }

    #[test]
    fn a_duplicate_id_is_refused() {
        let rows = vec![
            pri(),
            json!({"id": "task-2", "title": "B", "tier": "optional", "state": "assigned"}),
        ];
        let err = with_field(&rows, 1, "id", "task-1").expect_err("clash");
        assert!(err.contains("already used"), "{err}");
    }

    #[test]
    fn a_full_authoring_pass_produces_a_block_the_compile_accepts() {
        let mut rows = add_task(&[]);
        rows = with_field(&rows, 0, "title", "Seize the hill").expect("title");
        rows = with_field(&rows, 0, "tier", "primary").expect("tier");
        rows = with_field(&rows, 0, "triggerId", "trg-hill").expect("trigger");
        rows = with_field(&rows, 0, "markerId", "attack").expect("marker");
        rows = add_task(&rows);
        rows = with_field(&rows, 1, "title", "Find the cache").expect("t2");
        rows = with_field(&rows, 1, "tier", "secondary").expect("sec");
        rows = add_task(&rows);
        rows = with_field(&rows, 2, "title", "Radio check").expect("t3");
        rows = with_field(&rows, 2, "tier", "optional").expect("opt");
        rows = move_task(&rows, 2, -1);
        assert_eq!(rows[1]["tier"], "optional");
        website_map_engine::data::scenario::tasks::validate(&Value::Array(rows))
            .expect("the panel must not author a block the compile refuses");
    }

    #[test]
    fn clearing_writes_an_explicit_null_patch() {
        let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
        assert_eq!(cleared, json!({"tasks": null}));
        let set: Value = serde_json::from_str(&env_patch(Some(&[pri()]))).expect("json");
        assert_eq!(set["tasks"][0]["id"], "task-1");
    }

    #[test]
    fn the_reader_chain_names_every_hop() {
        let hops: Vec<&str> = TASKS_READERS.iter().map(|(h, _)| *h).collect();
        assert_eq!(hops, ["compile", "flatten", "mod", "editor"]);
        for (hop, reader) in TASKS_READERS {
            assert!(reader.len() > 30, "{hop}'s reader is not named: {reader}");
        }
    }

    #[test]
    fn with_schedule_writes_start_and_window() {
        let next = with_schedule(&[pri()], 0, "600", "300", 5400).expect("legal");
        assert_eq!(next[0]["schedule"]["startAfterS"], 600);
        assert_eq!(next[0]["schedule"]["windowS"], 300);
        assert_eq!(schedule_seconds(&next[0], "startAfterS"), "600");
        assert_eq!(schedule_seconds(&next[0], "windowS"), "300");
        assert_eq!(schedule_seconds(&pri(), "startAfterS"), "");
        website_map_engine::data::scenario::tasks::validate(&Value::Array(next))
            .expect("the panel must not author a block the compile refuses");
        assert_eq!(FLOW_DEFAULT_TIMELIMIT_S, 5400);
    }

    #[test]
    fn a_zero_window_is_refused_with_the_reason() {
        let err = with_schedule(&[pri()], 0, "10", "0", 5400).expect_err("window 0");
        assert!(err.contains("windowS"), "{err}");
        assert!(err.contains("> 0"), "{err}");
    }

    #[test]
    fn start_past_mission_length_is_refused_with_the_reason() {
        let err = with_schedule(&[pri()], 0, "5400", "60", 5400).expect_err("at end");
        assert!(err.contains("within mission length"), "{err}");
        assert!(err.contains("5400"), "{err}");
    }

    #[test]
    fn clearing_both_fields_removes_the_schedule() {
        let timed = with_schedule(&[pri()], 0, "120", "60", 5400).expect("set");
        let cleared = with_schedule(&timed, 0, "", "", 5400).expect("clear");
        assert!(cleared[0].get("schedule").is_none());
    }

    #[test]
    fn a_half_filled_schedule_is_refused() {
        let err = with_schedule(&[pri()], 0, "120", "", 5400).expect_err("window missing");
        assert!(err.contains("windowS"), "{err}");
        let err = with_schedule(&[pri()], 0, "", "60", 5400).expect_err("start missing");
        assert!(err.contains("startAfterS"), "{err}");
    }
