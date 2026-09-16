//! Role: Domain regression cases.
//! Position: `mission/extensions/tactical_graphics/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn one_graphic_of_each_kind_parses() {
    let got = parse(&one_of_each()).expect("parses");
    assert_eq!(got.len(), 4);
    assert_eq!(got[0].kind, "phase_line");
    assert_eq!(got[0].points, vec![[1000.0, 2000.0], [1400.0, 2100.0]]);
    assert_eq!(got[0].label.as_deref(), Some("PL BLUE"));
    assert_eq!(got[0].side_key.as_deref(), Some("blufor"));
    assert_eq!(
        got[0].style.as_ref().and_then(|s| s.color.as_deref()),
        Some("#3388ff")
    );
    assert_eq!(got[0].style.as_ref().and_then(|s| s.alpha), Some(0.8));
    assert_eq!(got[1].kind, "boundary");
    assert!(got[1].label.is_none());
    assert!(got[1].style.is_none());
    assert_eq!(got[2].kind, "axis_of_advance");
    assert_eq!(got[3].kind, "curved_arrow");
    assert_eq!(got[3].points.len(), 3);
    assert_eq!(
        got[3].style.as_ref().and_then(|s| s.brush.as_deref()),
        Some("solid")
    );
    assert_eq!(got[3].style.as_ref().and_then(|s| s.width_m), Some(24.0));
}

#[test]
fn a_one_point_phase_line_is_refused() {
    let err = parse(&json!([{
        "id": "tg-short",
        "kind": "phase_line",
        "points": [[1.0, 2.0]]
    }]))
    .expect_err("a one-point phase line is not a line");
    assert!(err.contains("at least 2"), "{err}");

    assert_eq!(min_points("phase_line"), Some(2));
    assert_eq!(min_points("boundary"), Some(2));
    assert_eq!(min_points("axis_of_advance"), Some(2));
    assert_eq!(min_points("curved_arrow"), Some(3));
    assert_eq!(min_points("marker"), None);
}

#[test]
fn a_two_point_curved_arrow_is_refused_but_a_two_point_axis_is_not() {
    let err = parse(&json!([{
        "id": "tg-flat",
        "kind": "curved_arrow",
        "points": [[1.0, 2.0], [3.0, 4.0]]
    }]))
    .expect_err("two points cannot curve");
    assert!(err.contains("at least 3"), "{err}");

    parse(&json!([{
        "id": "tg-axis",
        "kind": "axis_of_advance",
        "points": [[1.0, 2.0], [3.0, 4.0]]
    }]))
    .expect("the straight kind accepts two");
}

#[test]
fn a_non_finite_coordinate_is_refused() {
    let err = parse(&json!([{
        "id": "tg-nan",
        "kind": "phase_line",
        "points": [[1.0, 2.0], ["3.0", 4.0]]
    }]))
    .expect_err("a string is not a coordinate");
    assert!(err.contains("must be a number"), "{err}");

    let mut graphic = json!({
        "id": "tg-inf",
        "kind": "phase_line",
        "points": [[1.0, 2.0], [0.0, 0.0]]
    });
    graphic["points"][1][0] = Value::from(f64::MAX);

    parse(&json!([graphic])).expect("a large finite coordinate is legal");
}

#[test]
fn a_vertex_that_is_not_a_pair_is_refused() {
    let err = parse(&json!([{
        "id": "tg-triple",
        "kind": "phase_line",
        "points": [[1.0, 2.0], [3.0, 4.0, 5.0]]
    }]))
    .expect_err("three values is not [x, z]");
    assert!(err.contains("exactly [x, z]"), "{err}");
}

#[test]
fn an_unknown_kind_and_an_unknown_key_are_refused() {
    let err = parse(&json!([{
        "id": "tg-x",
        "kind": "fire_support_line",
        "points": [[1.0, 2.0], [3.0, 4.0]]
    }]))
    .expect_err("unknown kind");
    assert!(err.contains("must be one of"), "{err}");

    let err = parse(&json!([{
        "id": "tg-x",
        "kind": "phase_line",
        "points": [[1.0, 2.0], [3.0, 4.0]],
        "colour": "#ffffff"
    }]))
    .expect_err("unknown key");
    assert!(err.contains("additionalProperties"), "{err}");
}

#[test]
fn a_style_outside_the_marker_vocabulary_is_refused() {
    let bad = |style: Value| {
        parse(&json!([{
            "id": "tg-s",
            "kind": "phase_line",
            "points": [[1.0, 2.0], [3.0, 4.0]],
            "style": style
        }]))
        .expect_err("bad style")
    };
    assert!(bad(json!({"color": "3388ff"})).contains("#rrggbb"));
    assert!(bad(json!({"color": "#3388f"})).contains("#rrggbb"));
    assert!(bad(json!({"alpha": 1.5})).contains("between 0 and 1"));
    assert!(bad(json!({"alpha": -0.1})).contains("between 0 and 1"));
    assert!(bad(json!({"brush": "stipple"})).contains("must be one of"));
    assert!(bad(json!({"widthM": 0.0})).contains("above zero"));
    assert!(bad(json!({"dash": true})).contains("additionalProperties"));

    for brush in BRUSHES {
        parse(&json!([{
            "id": "tg-s",
            "kind": "phase_line",
            "points": [[1.0, 2.0], [3.0, 4.0]],
            "style": {"brush": brush}
        }]))
        .unwrap_or_else(|e| panic!("{brush} must be legal: {e}"));
    }
}

#[test]
fn a_malformed_side_key_is_refused_and_an_authored_one_is_not() {
    let err = parse(&json!([{
        "id": "tg-s",
        "kind": "phase_line",
        "points": [[1.0, 2.0], [3.0, 4.0]],
        "sideKey": "BLUFOR"
    }]))
    .expect_err("uppercase is not a faction key");
    assert!(err.contains("lowercase"), "{err}");

    let got = parse(&json!([{
        "id": "tg-s",
        "kind": "phase_line",
        "points": [[1.0, 2.0], [3.0, 4.0]],
        "sideKey": "task_force_7"
    }]))
    .expect("an authored faction key passes");
    assert_eq!(got[0].side_key.as_deref(), Some("task_force_7"));
}

#[test]
fn duplicate_ids_and_empty_and_oversized_blocks_are_refused() {
    let err = parse(&json!([phase_line(), phase_line()])).expect_err("dup");
    assert!(err.contains("unique"), "{err}");

    let err = parse(&json!([])).expect_err("empty");
    assert!(err.contains("empty"), "{err}");

    let err = parse(&json!({})).expect_err("not an array");
    assert!(err.contains("must be an array"), "{err}");

    let too_many: Vec<Value> = (0..=MAX_POINTS).map(|i| json!([i as f64, 0.0])).collect();
    let err = parse(&json!([{
        "id": "tg-long",
        "kind": "phase_line",
        "points": too_many
    }]))
    .expect_err("over the cap");
    assert!(err.contains("at most 128"), "{err}");
}

#[test]
fn tactical_graphics_is_registered_on_the_carrier() {
    assert!(
        is_authored_block("tacticalGraphics"),
        "T-936.7's row must be in AUTHORED_BLOCKS or the carrier never emits it"
    );
    assert!(
        !DOCUMENT_OWNED_BLOCKS.contains(&"tacticalGraphics"),
        "tacticalGraphics is optional — it rides ExtensionBlocks"
    );
    assert_eq!(
        KINDS,
        ["phase_line", "boundary", "axis_of_advance", "curved_arrow"]
    );
    assert_eq!(MAX_POINTS, 128);
}

#[test]
fn tactical_graphics_registered_here_must_also_be_readable_by_flatten() {
    assert_eq!(
        AUTHORED_BLOCKS.len(),
        7,
        "the seven T-936 blocks; a row added without a flatten field is a silent drop"
    );
    let p = compile_env_with_graphics(&one_of_each());
    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert_eq!(carried.get("tacticalGraphics"), Some(&one_of_each()));
}

#[test]
fn a_mission_with_one_of_each_kind_copies_to_the_payload_root() {
    let block = one_of_each();
    let p = compile_env_with_graphics(&block);
    assert_eq!(
        p["tacticalGraphics"], block,
        "AUTHORED_BLOCKS must promote tacticalGraphics out of the env bag: {p:#}"
    );
    assert_eq!(p["tacticalGraphics"].as_array().expect("array").len(), 4);
    assert_eq!(
        p["environment"]["tacticalGraphics"], block,
        "the bag reaches the wire unchanged — that is the reload path"
    );
}

#[test]
fn an_unauthored_payload_still_omits_the_tactical_graphics_key() {
    let p = compile_payload(
        &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}}).to_string(),
        "{}",
        false,
    );
    assert!(
        p.get("tacticalGraphics").is_none(),
        "parity: no graphics authored ⇒ no tacticalGraphics key: {p:#}"
    );
    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert!(carried.get("tacticalGraphics").is_none());
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
fn a_malformed_block_is_refused_at_the_carrier_with_a_readable_clause() {
    let p = compile_env_with_graphics(&json!([{
        "id": "tg-short",
        "kind": "phase_line",
        "points": [[1.0, 2.0]]
    }]));
    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(carried.get("tacticalGraphics").is_none());
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].0, "tacticalGraphics");
    assert!(refusals[0].1.contains("at least 2"), "{:?}", refusals[0]);
}
