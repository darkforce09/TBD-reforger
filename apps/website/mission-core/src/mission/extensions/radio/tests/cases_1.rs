//! Role: Domain regression cases.
//! Position: `mission/extensions/radio/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn a_valid_plan_parses() {
    let got = parse(&one_net()).expect("parses");
    assert_eq!(got.nets.len(), 1);
    assert_eq!(got.nets[0].id, "net:blufor_cmd");
    assert_eq!(got.nets[0].freq_mhz, 30.0);
    assert_eq!(got.nets[0].faction.as_deref(), Some("blufor"));
    assert_eq!(got.nets[0].range.as_deref(), Some("long"));
}

#[test]
fn faction_and_range_may_be_omitted() {
    let got = parse(&json!({
        "nets": [{"id": "net:common", "label": "Common", "freqMHz": 40}]
    }))
    .expect("parses");
    assert!(got.nets[0].faction.is_none());
    assert!(got.nets[0].range.is_none());
}

#[test]
fn a_duplicate_frequency_is_refused() {
    let err = parse(&json!({
        "nets": [
            {"id": "net:a", "label": "A", "freqMHz": 41.0},
            {"id": "net:b", "label": "B", "freqMHz": 41.0}
        ]
    }))
    .expect_err("duplicate frequency");
    assert!(err.contains("same frequency"), "{err}");
    assert!(err.contains("41"), "{err}");
    assert!(err.contains("net:a"), "{err}");
    refuse_duplicate_frequency(
        &[AuthoredNet {
            id: "net:a".into(),
            label: "A".into(),
            freq_mhz: 41.0,
            faction: None,
            range: None,
        }],
        41.0,
        1,
    )
    .expect_err("the helper itself must refuse");
}

#[test]
fn an_out_of_range_frequency_is_refused() {
    let low = parse(&json!({
        "nets": [{"id": "net:a", "label": "A", "freqMHz": 29.9}]
    }))
    .expect_err("below floor");
    assert!(low.contains("29.9"), "{low}");
    assert!(low.contains("30"), "{low}");

    let high = parse(&json!({
        "nets": [{"id": "net:a", "label": "A", "freqMHz": 512.1}]
    }))
    .expect_err("above ceiling");
    assert!(high.contains("512"), "{high}");
}

#[test]
fn a_duplicate_id_is_refused() {
    let err = parse(&json!({
        "nets": [
            {"id": "net:a", "label": "A", "freqMHz": 30.0},
            {"id": "net:a", "label": "B", "freqMHz": 30.5}
        ]
    }))
    .expect_err("duplicate id");
    assert!(err.contains("unique"), "{err}");
    assert!(err.contains("net:a"), "{err}");
}

#[test]
fn more_than_max_nets_is_refused() {
    let nets: Vec<Value> = (0..=MAX_NETS)
        .map(|i| {
            json!({
                "id": format!("net:n{i}"),
                "label": format!("N{i}"),
                "freqMHz": FREQ_MIN_MHZ + 0.5 * i as f64
            })
        })
        .collect();
    let err = parse(&json!({"nets": nets})).expect_err("over cap");
    assert!(err.contains(&MAX_NETS.to_string()), "{err}");
}

#[test]
fn an_empty_nets_array_is_refused() {
    let err = parse(&json!({"nets": []})).expect_err("empty");
    assert!(err.contains("empty"), "{err}");
}

#[test]
fn radio_plan_is_registered_and_document_modelled() {
    assert!(is_authored_block("radioPlan"));
    assert!(
        DOCUMENT_OWNED_BLOCKS.contains(&"radioPlan"),
        "radioPlan has a typed field on ModMissionDocument; carrying it too would emit the key twice"
    );
    let (blocks, refusals) = AuthoredBlocks::parse(&json!({"radioPlan": one_net()}));
    assert!(refusals.is_empty(), "{refusals:?}");
    let plan = blocks.radio_plan.expect("parsed");
    assert_eq!(plan.nets[0].id, "net:blufor_cmd");
}

#[test]
fn a_mission_copies_radio_plan_to_the_payload_root() {
    let plan = one_net();
    let p = compile_env_with_plan(&plan);
    assert_eq!(
        p["radioPlan"], plan,
        "AUTHORED_BLOCKS must promote radioPlan out of the env bag: {p:#}"
    );
    let mut dst = serde_json::Map::new();
    let copied = copy_authored_blocks(&json!({"radioPlan": plan}), &mut dst);
    assert_eq!(copied, ["radioPlan"]);
}

#[test]
fn an_unauthored_payload_still_omits_the_radio_plan_key() {
    let p = compile_payload(
        &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}}).to_string(),
        "{}",
        false,
    );
    assert!(
        p.get("radioPlan").is_none(),
        "parity: no radioPlan authored ⇒ no radioPlan key on the payload: {p:#}"
    );
}
