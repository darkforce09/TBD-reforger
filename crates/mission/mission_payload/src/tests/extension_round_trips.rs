//! Round trips of every authored extension block through the payload compiler.
//!
//! **Role:** proves the compiler promotes each authored block out of the environment bag onto the
//! payload root and omits its key when nothing is authored; where the block rides the extension
//! carrier, the carrier reads the compiled block back, and a malformed block is refused there.
//! **Position:** a payload test over the extension block registry of the mission model; the
//! blocks' parse and validate tests stay with the mission model, which never depends on the
//! compiler.
//! **Signals & state:** none; pure functions over JSON fixtures.
//! **Invariants:** one table row per authored block; the fixtures match the blocks' own test
//! fixtures byte for byte.

use crate::compile_payload;
use mission_model::authored_blocks::ExtensionBlocks;
use serde_json::{Value, json};

/// One authored block: its payload key, a schema-legal fixture, and whether the extension carrier
/// (rather than the document) owns it.
struct BlockCase {
    key: &'static str,
    block: Value,
    rides_the_carrier: bool,
}

fn cases() -> Vec<BlockCase> {
    let group = "{000CD338713F2B5A}Prefabs/AI/Groups/Group_Base.et";
    vec![
        BlockCase {
            key: "audio",
            block: json!({
                "emitters": [
                    {"id": "ae-gen", "x": 100.0, "z": 200.0, "sound": "SOUND_HINT",
                     "radiusM": 25.0, "loop": true},
                    {"id": "ae-shot", "x": 300.0, "z": 400.0, "y": 12.5, "sound": "SOUND_FEVER",
                     "radiusM": 8.0, "loop": false, "triggerId": "tr-door"}
                ],
                "musicCues": [{"id": "mc-start", "event": "mission_start", "track": "SOUND_HINT"}]
            }),
            rides_the_carrier: true,
        },
        BlockCase {
            key: "radioPlan",
            block: json!({"nets": [{"id": "net:blufor_cmd", "label": "Command", "freqMHz": 30.0,
                                    "faction": "blufor", "range": "long"}]}),
            rides_the_carrier: false,
        },
        BlockCase {
            key: "spawnModules",
            block: json!([
                {"id": "sm-wave", "kind": "wave", "factionKey": "opfor", "groupTemplate": group,
                 "x": 1200.0, "z": 3400.0, "count": 2, "intervalSeconds": 45.0, "maxAlive": 4},
                {"id": "sm-gar", "kind": "garrison", "factionKey": "blufor",
                 "groupTemplate": group, "zoneId": "z_spawn_blufor", "count": 1}
            ]),
            rides_the_carrier: true,
        },
        BlockCase {
            key: "tacticalGraphics",
            block: json!([
                {"id": "tg-phase", "kind": "phase_line",
                 "points": [[1000.0, 2000.0], [1400.0, 2100.0]], "label": "PL BLUE",
                 "sideKey": "blufor", "style": {"color": "#3388ff", "alpha": 0.8}},
                {"id": "tg-bound", "kind": "boundary",
                 "points": [[900.0, 1800.0], [1200.0, 1900.0], [1500.0, 2400.0]]},
                {"id": "tg-axis", "kind": "axis_of_advance",
                 "points": [[800.0, 1200.0], [1600.0, 2600.0]], "label": "AXIS SABRE"},
                {"id": "tg-arrow", "kind": "curved_arrow",
                 "points": [[700.0, 1100.0], [1100.0, 1500.0], [1700.0, 1400.0]],
                 "style": {"brush": "solid", "color": "#ff2222", "widthM": 24.0}}
            ]),
            rides_the_carrier: true,
        },
        BlockCase {
            key: "tasks",
            block: json!([
                {"id": "t-pri", "title": "Seize the hill", "tier": "primary", "state": "assigned",
                 "triggerId": "trg-hill", "markerId": "attack"},
                {"id": "t-sec", "title": "Find the cache", "tier": "secondary",
                 "state": "assigned", "triggerId": "trg-cache"},
                {"id": "t-opt", "title": "Radio check", "tier": "optional", "state": "assigned",
                 "description": "No trigger — stays assigned."}
            ]),
            rides_the_carrier: true,
        },
        BlockCase {
            key: "weatherTimeline",
            block: json!({"keyframes": [
                {"atMinutes": 0, "weatherPreset": "clear"},
                {"atMinutes": 15, "weatherPreset": "overcast", "windDirDeg": 90.0},
                {"atMinutes": 40, "weatherPreset": "heavy_rain", "fog": 0.4}
            ]}),
            rides_the_carrier: true,
        },
    ]
}

fn compile_with_environment(environment: Value) -> Value {
    compile_payload(
        &json!({"meta": {"terrain": "everon", "environment": environment}}).to_string(),
        "{}",
        false,
    )
}

#[test]
fn every_authored_block_reaches_the_payload_root_and_the_carrier_reads_it_back() {
    for case in cases() {
        let p = compile_with_environment(json!({"weather": "clear", case.key: case.block}));
        assert_eq!(
            p[case.key], case.block,
            "{} must reach the payload root: {p:#}",
            case.key
        );

        let (carried, refusals) = ExtensionBlocks::from_payload(&p);
        assert!(refusals.is_empty(), "{}: {refusals:?}", case.key);
        let expected = case.rides_the_carrier.then_some(&case.block);
        assert_eq!(
            carried.get(case.key),
            expected,
            "{} on the carrier",
            case.key
        );
    }
}

#[test]
fn an_unauthored_payload_omits_every_block_key() {
    let p = compile_with_environment(json!({"weather": "clear"}));
    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    for case in cases() {
        assert!(
            p.get(case.key).is_none(),
            "no {} authored, no key: {p:#}",
            case.key
        );
        assert!(
            carried.get(case.key).is_none(),
            "{} on the carrier",
            case.key
        );
    }
}

#[test]
fn a_malformed_block_is_refused_at_the_carrier_with_a_readable_clause() {
    let p = compile_with_environment(json!({
        "weather": "clear",
        "tacticalGraphics": [{"id": "tg-short", "kind": "phase_line", "points": [[1.0, 2.0]]}]
    }));
    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(carried.get("tacticalGraphics").is_none());
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].0, "tacticalGraphics");
    assert!(refusals[0].1.contains("at least 2"), "{:?}", refusals[0]);
}
