//! Tests the audio emitters subject.

    use super::*;
    use serde_json::json;

    fn ae() -> Value {
        json!({"id": "ae-1", "x": 1.0, "z": 2.0, "sound": "SOUND_HINT", "radiusM": 10.0, "loop": true})
    }

    #[test]
    fn add_appends_a_valid_emitter() {
        let next = add_emitter(&[], &[]).expect("add");
        assert_eq!(next.len(), 1);
        validate(&block_from_parts(&next, &[]).expect("block")).expect("valid");
    }

    #[test]
    fn remove_drops_and_clearing_the_last_writes_null() {
        let next = remove_at(&[ae()], 0);
        assert!(next.is_empty());
        let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
        assert_eq!(cleared, json!({"audio": null}));
    }

    #[test]
    fn radius_zero_is_refused_in_the_panel() {
        let err = with_emitter_field(&[ae()], &[], 0, "radiusM", "0").expect_err("zero");
        assert!(err.contains("above zero"), "{err}");
    }

    #[test]
    fn an_unknown_event_is_refused() {
        let cue = default_cue(&[], &[]);
        let err = with_cue_field(&[cue], &[], 0, "event", "round_pause").expect_err("event");
        assert!(err.contains("round_pause"), "{err}");
    }

    #[test]
    fn duplicate_ids_are_refused() {
        let cue = json!({"id": "ae-1", "event": "mission_end", "track": "SOUND_HINT"});
        let err = refuse_block(&[ae()], &[cue]).expect_err("dup");
        assert!(err.contains("unique"), "{err}");
    }

    #[test]
    fn last_marker_fills_xz() {
        let next = apply_marker_xz(&[ae()], &[], 0, 6400.0, 1200.0).expect("xz");
        assert_eq!(next[0]["x"], 6400.0);
        assert_eq!(next[0]["z"], 1200.0);
        assert_eq!(
            xz_from_last_marker(&[(1.0, 2.0), (9.0, 8.0)]),
            Some((9.0, 8.0))
        );
        assert!(xz_from_last_marker(&[]).is_none());
    }

    #[test]
    fn the_pickers_offer_exactly_the_schema_vocabulary() {
        assert_eq!(
            MUSIC_EVENTS,
            [
                "mission_start",
                "task_succeeded",
                "task_failed",
                "mission_end"
            ]
        );
        for e in MUSIC_EVENTS {
            assert_ne!(event_label(e), "Unknown event");
        }
    }

    #[test]
    fn env_patch_sets_and_clears() {
        let set: Value = serde_json::from_str(&env_patch(block_from_parts(&[ae()], &[]).as_ref()))
            .expect("json");
        assert_eq!(set["audio"]["emitters"][0]["id"], "ae-1");
        let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
        assert_eq!(cleared, json!({"audio": null}));
    }

    #[test]
    fn the_reader_chain_names_every_hop() {
        let hops: Vec<&str> = AUDIO_READERS.iter().map(|(h, _)| *h).collect();
        assert_eq!(hops, ["compile", "flatten", "mod", "editor"]);
        for (hop, reader) in AUDIO_READERS {
            assert!(reader.len() > 30, "{hop}'s reader is not named: {reader}");
        }
    }

    #[test]
    fn place_on_map_reuses_the_marker_gesture() {
        const SRC: &str = include_str!("audio_emitters.rs");
        assert!(
            SRC.contains("begin_place_marker"),
            "placement must call begin_place_marker, not a new gesture"
        );
        assert!(SRC.contains(PLACE_MARKER_ICON));
        assert!(SRC.contains("arm_place_on_map"));
    }
