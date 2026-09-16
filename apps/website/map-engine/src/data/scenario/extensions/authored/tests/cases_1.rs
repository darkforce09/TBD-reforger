//! Role: Domain regression cases.
//! Position: `mission/extensions/authored/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn win_conditions_is_the_registered_block_and_the_document_models_it() {
    assert!(is_authored_block("winConditions"));
    assert!(DOCUMENT_OWNED_BLOCKS.contains(&"winConditions"));
    assert!(
        is_authored_block("tasks"),
        "T-936.2 registers tasks; a missing row is a silent drop at flatten"
    );
    assert!(
        is_authored_block("radioPlan"),
        "T-936.3 registers radioPlan; a missing row is a silent drop at flatten"
    );
    assert!(DOCUMENT_OWNED_BLOCKS.contains(&"radioPlan"));
    assert!(
        is_authored_block("weatherTimeline"),
        "T-936.4 registers weatherTimeline; a missing row is a silent drop at flatten"
    );
    assert!(!DOCUMENT_OWNED_BLOCKS.contains(&"weatherTimeline"));
    assert!(
        is_authored_block("audio"),
        "T-936.5 registers audio; a missing row is a silent drop at flatten"
    );
    assert!(!DOCUMENT_OWNED_BLOCKS.contains(&"audio"));
    assert!(
        is_authored_block("spawnModules"),
        "T-936.6 registers spawnModules; a missing row is a silent drop at flatten"
    );
    assert!(!DOCUMENT_OWNED_BLOCKS.contains(&"spawnModules"));
    assert!(
        is_authored_block("tacticalGraphics"),
        "T-936.7 registers tacticalGraphics; a missing row is a silent drop at flatten"
    );
    assert!(!DOCUMENT_OWNED_BLOCKS.contains(&"tacticalGraphics"));
    assert!(!is_authored_block("payloadExtras"));

    assert!(!is_authored_block("notAnAuthoredBlock"));
    assert_eq!(AUTHORED_BLOCKS.len(), 7);
}

#[test]
fn every_document_owned_block_is_registered() {
    for key in DOCUMENT_OWNED_BLOCKS {
        assert!(is_authored_block(key), "`{key}` is not in AUTHORED_BLOCKS");
    }
}

#[test]
fn copy_carries_a_listed_key_verbatim_and_leaves_everything_else() {
    let env = json!({
        "weather": "clear",
        "timeLimitSeconds": 5400,
        "winConditions": {"mode": "vip", "endOn": ["faction_eliminated"], "vipSlotId": "s1"},
        "notAnAuthoredBlock": [],
    });
    let mut dst = Map::new();
    let copied = copy_authored_blocks(&env, &mut dst);

    assert_eq!(copied, ["winConditions"]);
    assert_eq!(dst.len(), 1, "only the listed key travels: {dst:?}");
    assert_eq!(dst["winConditions"], env["winConditions"], "verbatim");
    assert!(
        !dst.contains_key("notAnAuthoredBlock"),
        "an unlisted key stays parked in payloadExtras"
    );
    assert!(
        !dst.contains_key("weather"),
        "the bag's own keys stay in the bag"
    );
}

#[test]
fn an_absent_or_null_block_copies_nothing() {
    let mut dst = Map::new();
    assert!(copy_authored_blocks(&json!({}), &mut dst).is_empty());
    assert!(dst.is_empty());

    let mut dst = Map::new();
    assert!(copy_authored_blocks(&json!({"winConditions": null}), &mut dst).is_empty());
    assert!(
        dst.is_empty(),
        "null is a cleared key, not an authored one: {dst:?}"
    );

    let mut dst = Map::new();
    assert!(copy_authored_blocks(&json!(null), &mut dst).is_empty());
    assert!(dst.is_empty());
}

#[test]
fn a_malformed_block_is_saved_but_refused_at_compile() {
    let bad = json!({"mode": "vip_hunt", "endOn": ["time_limit"]});

    let mut dst = Map::new();
    assert_eq!(
        copy_authored_blocks(&json!({"winConditions": bad}), &mut dst),
        ["winConditions"],
        "save must not lose the author's work in progress"
    );

    let (blocks, refusals) = AuthoredBlocks::parse(&json!({"winConditions": bad}));
    assert!(
        blocks.win_conditions.is_none(),
        "compile falls back to the derivation"
    );
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].0, "winConditions");
    assert!(refusals[0].1.contains("vip_hunt"), "{}", refusals[0].1);
}

#[test]
fn parse_reads_a_good_block_and_ignores_an_absent_one() {
    let (blocks, refusals) = AuthoredBlocks::parse(&json!({
        "winConditions": {
            "mode": "extraction", "endOn": ["time_limit"], "extractionZoneId": "z1"
        }
    }));
    assert!(refusals.is_empty(), "{refusals:?}");
    let wc = blocks.win_conditions.expect("parsed");
    assert_eq!(wc.mode, "extraction");
    assert_eq!(wc.params.extraction_zone_id.as_deref(), Some("z1"));

    let (blocks, refusals) = AuthoredBlocks::parse(&json!({"schemaVersion": 1}));
    assert_eq!(blocks, AuthoredBlocks::default());
    assert!(refusals.is_empty());
}

#[test]
fn every_document_owned_block_has_a_parse_arm() {
    for key in DOCUMENT_OWNED_BLOCKS {
        let sample = match *key {
            "winConditions" => json!({"mode": "attrition", "endOn": ["time_limit"]}),
            "radioPlan" => {
                json!({"nets": [{"id": "net:blufor_cmd", "label": "Command", "freqMHz": 30.0}]})
            }
            other => panic!(
                "DOCUMENT_OWNED_BLOCKS row `{other}` has no sample here — add one, and an arm \
                     in AuthoredBlocks::parse, or the block validates and is then dropped"
            ),
        };
        let mut payload = Map::new();
        payload.insert((*key).to_string(), sample);
        let (blocks, refusals) = AuthoredBlocks::parse(&Value::Object(payload));
        assert!(refusals.is_empty(), "{refusals:?}");
        let landed = match *key {
            "winConditions" => blocks.win_conditions.is_some(),
            "radioPlan" => blocks.radio_plan.is_some(),
            _ => false,
        };
        assert!(
            landed,
            "`{key}` validated but AuthoredBlocks::parse dropped it"
        );
    }
}

#[test]
fn an_empty_carrier_adds_nothing_to_the_document() {
    let empty = ExtensionBlocks::default();
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);

    let h = Host {
        win_conditions: json!({"mode": "attrition", "endOn": ["time_limit"]}),
        extensions: empty,
    };
    assert_eq!(
        serde_json::to_string(&h).expect("serialises"),
        r#"{"winConditions":{"mode":"attrition","endOn":["time_limit"]}}"#,
        "an empty carrier must add NOTHING — this is the parity claim, in bytes"
    );
}

#[test]
fn the_carrier_withholds_a_document_modelled_block() {
    let (carried, refusals) = ExtensionBlocks::from_payload(&json!({
        "winConditions": {"mode": "vip", "endOn": ["time_limit"], "vipSlotId": "s1"}
    }));
    assert!(refusals.is_empty(), "{refusals:?}");
    assert!(
        carried.is_empty(),
        "winConditions has a typed field; carrying it too would emit the key twice"
    );

    let wire = host(carried);
    assert_eq!(
        wire.as_object().expect("object").len(),
        1,
        "exactly one winConditions key on the wire: {wire}"
    );
}

#[test]
fn a_carried_block_reaches_the_document_root() {
    let mut carried = ExtensionBlocks::default();
    carried.set("tasks", json!([{"id": "t1", "tier": "primary"}]));
    assert!(!carried.is_empty());
    assert_eq!(carried.len(), 1);
    assert_eq!(
        carried.get("tasks"),
        Some(&json!([{"id": "t1", "tier": "primary"}]))
    );

    let wire = host(carried);
    assert_eq!(
        wire["tasks"],
        json!([{"id": "t1", "tier": "primary"}]),
        "a carried block must land at the ROOT, beside winConditions: {wire}"
    );
    assert_eq!(wire["winConditions"]["mode"], "attrition");
    assert_eq!(
        wire.as_object().expect("object").len(),
        2,
        "the modelled block plus the one carried block: {wire}"
    );
}

#[test]
fn the_carrier_is_ordered_and_cannot_emit_a_key_twice() {
    let mut carried = ExtensionBlocks::default();
    carried.set("tasks", json!([1]));
    carried.set("audio", json!({"emitters": []}));
    carried.set("tasks", json!([2]));
    assert_eq!(carried.len(), 2, "a re-set replaces");

    let text = serde_json::to_string(&Host {
        win_conditions: json!({"mode": "attrition", "endOn": ["time_limit"]}),
        extensions: carried,
    })
    .expect("serialises");
    assert_eq!(
        text,
        r#"{"winConditions":{"mode":"attrition","endOn":["time_limit"]},"tasks":[2],"audio":{"emitters":[]}}"#
    );
}
