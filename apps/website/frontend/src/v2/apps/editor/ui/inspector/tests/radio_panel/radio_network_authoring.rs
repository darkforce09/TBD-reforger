//! Tests the radio panel subject.

    use super::*;
    use serde_json::json;

    fn cmd() -> Value {
        json!({
            "id": "net:blufor_cmd",
            "label": "Command",
            "freqMHz": 30.0,
            "faction": "blufor",
            "range": "long"
        })
    }

    #[test]
    fn add_appends_a_unique_id_and_frequency() {
        let next = add_net(&[cmd()]).expect("add");
        assert_eq!(next.len(), 2);
        assert_eq!(next[1]["id"], "net:ch_2");
        assert_eq!(next[1]["freqMHz"], 30.5);
        validate(&plan_from_nets(&next).expect("plan"))
            .expect("the panel must not author a refused block");
    }

    #[test]
    fn remove_drops_one_row_and_clearing_the_last_writes_null() {
        let next = remove_net(&[cmd()], 0);
        assert!(next.is_empty());
        let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
        assert_eq!(cleared, json!({"radioPlan": null}));
    }

    #[test]
    fn a_duplicate_frequency_is_refused_in_the_panel() {
        let rows = vec![
            cmd(),
            json!({"id": "net:blufor_alpha", "label": "Alpha", "freqMHz": 31.0}),
        ];
        let err = with_field(&rows, 1, "freqMHz", "30").expect_err("clash");
        assert!(err.contains("already used"), "{err}");
        assert!(err.contains("30"), "{err}");
    }

    #[test]
    fn an_out_of_range_frequency_is_refused_in_the_panel() {
        let err = with_field(&[cmd()], 0, "freqMHz", "20").expect_err("low");
        assert!(err.contains("outside"), "{err}");
        let err = with_field(&[cmd()], 0, "freqMHz", "900").expect_err("high");
        assert!(err.contains("outside"), "{err}");
    }

    #[test]
    fn reset_to_derived_writes_an_explicit_null_patch() {
        let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
        assert_eq!(cleared, json!({"radioPlan": null}));
        let set: Value =
            serde_json::from_str(&env_patch(plan_from_nets(&[cmd()]).as_ref())).expect("json");
        assert_eq!(set["radioPlan"]["nets"][0]["id"], "net:blufor_cmd");
    }

    #[test]
    fn a_full_authoring_pass_produces_a_block_the_compile_accepts() {
        let mut rows = add_net(&[]).expect("first");
        rows = with_field(&rows, 0, "label", "Command").expect("label");
        rows = with_field(&rows, 0, "faction", "blufor").expect("faction");
        rows = with_field(&rows, 0, "range", "long").expect("range");
        rows = add_net(&rows).expect("second");
        rows = with_field(&rows, 1, "label", "Alpha").expect("alpha");
        rows = with_field(&rows, 1, "faction", "blufor").expect("fac2");
        validate(&plan_from_nets(&rows).expect("plan")).expect("valid");
    }

    #[test]
    fn the_reader_chain_names_every_hop() {
        let hops: Vec<&str> = RADIO_READERS.iter().map(|(h, _)| *h).collect();
        assert_eq!(hops, ["compile", "flatten", "mod", "editor"]);
        for (hop, reader) in RADIO_READERS {
            assert!(reader.len() > 30, "{hop}'s reader is not named: {reader}");
        }
    }

    #[test]
    fn the_pickers_offer_exactly_the_schema_vocabulary() {
        assert_eq!(RANGES, ["short", "long"]);
        for r in RANGES {
            assert_ne!(range_label(r), "Unknown range");
        }
    }
