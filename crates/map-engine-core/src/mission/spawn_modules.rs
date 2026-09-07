//! T-936.6 — the authored `spawnModules[]` block: waves and garrisons.
//!
//! ══ Why this module exists ══════════════════════════════════════════════════════════════════
//! Every AI unit is placed statically. There is no wave or garrison vocabulary in
//! `mission.schema.json`, no editor panel, and no script under `Scripts/Game/TBD/Gamemode` that
//! spawns groups at runtime. This block is
//! `[{id, kind: wave|garrison, factionKey, groupTemplate, x+z XOR zoneId, count, intervalSeconds?,
//! maxAlive?, triggerId?}]`. The editor writes it into `meta.environment.spawnModules`;
//! [`crate::mission::extensions::AUTHORED_BLOCKS`] copies it onto the compiled payload root;
//! `TBD_DynamicSpawner.c` runs it. A mission that authors no modules still compiles to today's
//! bytes — the carrier emits nothing when the key is absent.
//!
//! ══ Placement ═══════════════════════════════════════════════════════════════════════════════
//! A module names EITHER a world position (`x` and `z` together) OR a `zoneId`, never both and
//! never neither. [`placement_is_exclusive`] is the perturbation target: widening it so both
//! sides can be true turns [`tests::both_position_and_zone_are_refused`] red.
//!
//! ══ Factions and caps ═══════════════════════════════════════════════════════════════════════
//! `factionKey` is the closed set [`FACTION_KEYS`] — the same four keys
//! `TBD_SpawnManager.EngineFactionKey` maps. `count` and `maxAlive` are strictly positive and
//! hard-capped at [`MAX_ALIVE`] so a wave cannot request an unbounded server load.

use serde_json::{Map, Value};

/// `kind` vocabulary `$defs/spawnModule.kind` declares.
pub const KINDS: &[&str] = &["wave", "garrison"];

/// Faction keys the spawner can resolve. Order matches SpawnManager's EngineFactionKey cases.
pub const FACTION_KEYS: &[&str] = &["blufor", "opfor", "indfor", "civ"];

/// Hard cap on `count` and `maxAlive`. A wave above this is refused here so the mod never
/// receives a number it would honour by spawning without bound.
pub const MAX_ALIVE: i64 = 32;

const MODULE_KEYS: &[&str] = &[
    "id",
    "kind",
    "factionKey",
    "groupTemplate",
    "x",
    "z",
    "zoneId",
    "count",
    "intervalSeconds",
    "maxAlive",
    "triggerId",
];

/// One authored spawn module.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredSpawnModule {
    pub id: String,
    pub kind: String,
    pub faction_key: String,
    pub group_template: String,
    pub x: Option<f64>,
    pub z: Option<f64>,
    pub zone_id: Option<String>,
    pub count: i64,
    pub interval_seconds: Option<f64>,
    pub max_alive: Option<i64>,
    pub trigger_id: Option<String>,
}

/// Exclusive placement: position XOR zone, and at least one side.
///
/// Widening this so both sides can be true is the T-936.6 perturbation:
/// [`tests::both_position_and_zone_are_refused`] goes red, restore + `touch` goes green.
#[must_use]
pub fn placement_is_exclusive(has_position: bool, has_zone: bool) -> bool {
    has_position != has_zone
}

/// Parse an authored `spawnModules[]` array.
///
/// # Errors
/// Wrong type, empty array, a row the schema would refuse, a duplicate id, an unknown faction
/// or kind, a count/maxAlive outside `(0, MAX_ALIVE]`, a non-positive interval, or placement
/// that is not exclusive.
pub fn parse(value: &Value) -> Result<Vec<AuthoredSpawnModule>, String> {
    let Some(arr) = value.as_array() else {
        return Err(format!(
            "`spawnModules` must be an array, not {}",
            type_name(value)
        ));
    };
    if arr.is_empty() {
        return Err(
            "`spawnModules` is empty — an empty list is omitted rather than authored".into(),
        );
    }

    let mut out = Vec::with_capacity(arr.len());
    let mut seen: Vec<String> = Vec::with_capacity(arr.len());
    for (index, item) in arr.iter().enumerate() {
        let row = parse_module(item, index)?;
        if seen.iter().any(|s| s == &row.id) {
            return Err(format!(
                "`spawnModules[{index}]`.id is {} — each spawn module id must be unique",
                quote(&row.id)
            ));
        }
        seen.push(row.id.clone());
        out.push(row);
    }
    Ok(out)
}

/// [`parse`] with the value discarded — the [`crate::mission::extensions::AUTHORED_BLOCKS`] row's
/// validator.
///
/// # Errors
/// Every error [`parse`] returns.
pub fn validate(value: &Value) -> Result<(), String> {
    parse(value).map(|_| ())
}

fn parse_module(value: &Value, index: usize) -> Result<AuthoredSpawnModule, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`spawnModules[{index}]` must be an object, not {}",
            type_name(value)
        ));
    };
    refuse_unknown(obj, MODULE_KEYS, &format!("spawnModules[{index}]"))?;

    let id = required_nonempty(obj, index, "id")?;
    let kind = required_nonempty(obj, index, "kind")?;
    if !KINDS.contains(&kind.as_str()) {
        return Err(format!(
            "`spawnModules[{index}]`.kind is {} — must be one of {}",
            quote(&kind),
            KINDS.join(", ")
        ));
    }
    let faction_key = required_nonempty(obj, index, "factionKey")?;
    if !FACTION_KEYS.contains(&faction_key.as_str()) {
        return Err(format!(
            "`spawnModules[{index}]`.factionKey is {} — must be a known faction ({})",
            quote(&faction_key),
            FACTION_KEYS.join(", ")
        ));
    }
    let group_template = required_nonempty(obj, index, "groupTemplate")?;

    let x = optional_finite(obj, index, "x")?;
    let z = optional_finite(obj, index, "z")?;
    if x.is_some() != z.is_some() {
        return Err(format!(
            "`spawnModules[{index}]` has incomplete position — x and z must be authored together"
        ));
    }
    let zone_id = optional_nonempty(obj, index, "zoneId")?;
    let has_position = x.is_some() && z.is_some();
    let has_zone = zone_id.is_some();
    if !placement_is_exclusive(has_position, has_zone) {
        return Err(format!(
            "`spawnModules[{index}]` must name either x+z or zoneId, never both and never neither"
        ));
    }

    let count = required_count(obj, index, "count")?;
    let max_alive = optional_count(obj, index, "maxAlive")?;
    let interval_seconds = optional_positive_seconds(obj, index, "intervalSeconds")?;
    let trigger_id = optional_nonempty(obj, index, "triggerId")?;

    Ok(AuthoredSpawnModule {
        id,
        kind,
        faction_key,
        group_template,
        x,
        z,
        zone_id,
        count,
        interval_seconds,
        max_alive,
        trigger_id,
    })
}

fn refuse_unknown(obj: &Map<String, Value>, allowed: &[&str], path: &str) -> Result<(), String> {
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!(
                "`{path}` carries {key:?}, which the schema does not declare \
                 (additionalProperties is false)"
            ));
        }
    }
    Ok(())
}

fn required_nonempty(obj: &Map<String, Value>, index: usize, key: &str) -> Result<String, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!(
            "`spawnModules[{index}]`.{key} is required and is missing"
        ));
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`spawnModules[{index}]`.{key} must be a string, not {}",
            type_name(raw)
        ));
    };
    if s.is_empty() {
        return Err(format!("`spawnModules[{index}]`.{key} is blank"));
    }
    Ok(s.to_string())
}

fn optional_nonempty(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<Option<String>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`spawnModules[{index}]`.{key} must be a string, not {}",
            type_name(raw)
        ));
    };
    if s.is_empty() {
        return Err(format!("`spawnModules[{index}]`.{key} is blank"));
    }
    Ok(Some(s.to_string()))
}

fn optional_finite(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<Option<f64>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    let Some(n) = raw.as_f64() else {
        return Err(format!(
            "`spawnModules[{index}]`.{key} must be a number, not {}",
            type_name(raw)
        ));
    };
    if !n.is_finite() {
        return Err(format!(
            "`spawnModules[{index}]`.{key} is {n} — must be a finite number"
        ));
    }
    Ok(Some(n))
}

fn required_count(obj: &Map<String, Value>, index: usize, key: &str) -> Result<i64, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!(
            "`spawnModules[{index}]`.{key} is required and is missing"
        ));
    };
    parse_count(raw, index, key)
}

fn optional_count(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<Option<i64>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    parse_count(raw, index, key).map(Some)
}

fn parse_count(raw: &Value, index: usize, key: &str) -> Result<i64, String> {
    let Some(n) = as_i64(raw) else {
        return Err(format!(
            "`spawnModules[{index}]`.{key} must be an integer, not {}",
            type_name(raw)
        ));
    };
    if n <= 0 || n > MAX_ALIVE {
        return Err(format!(
            "`spawnModules[{index}]`.{key} is {n} — must be in 1..={MAX_ALIVE}"
        ));
    }
    Ok(n)
}

fn optional_positive_seconds(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<Option<f64>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    let Some(n) = raw.as_f64() else {
        return Err(format!(
            "`spawnModules[{index}]`.{key} must be a number, not {}",
            type_name(raw)
        ));
    };
    if !n.is_finite() || n <= 0.0 {
        return Err(format!(
            "`spawnModules[{index}]`.{key} is {n} — intervalSeconds must be above zero"
        ));
    }
    Ok(Some(n))
}

fn as_i64(raw: &Value) -> Option<i64> {
    if let Some(n) = raw.as_i64() {
        return Some(n);
    }
    if let Some(n) = raw.as_u64() {
        return i64::try_from(n).ok();
    }
    let n = raw.as_f64()?;
    if n.is_finite() && n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
        return Some(n as i64);
    }
    None
}

fn quote(s: &str) -> String {
    format!("{s:?}")
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::compile::compile_payload;
    use crate::mission::extensions::{ExtensionBlocks, copy_authored_blocks, is_authored_block};
    use serde_json::json;

    fn wave() -> Value {
        json!({
            "id": "sm-wave",
            "kind": "wave",
            "factionKey": "opfor",
            "groupTemplate": "{000CD338713F2B5A}Prefabs/AI/Groups/Group_Base.et",
            "x": 1200.0,
            "z": 3400.0,
            "count": 2,
            "intervalSeconds": 45.0,
            "maxAlive": 4
        })
    }

    fn garrison() -> Value {
        json!({
            "id": "sm-gar",
            "kind": "garrison",
            "factionKey": "blufor",
            "groupTemplate": "{000CD338713F2B5A}Prefabs/AI/Groups/Group_Base.et",
            "zoneId": "z_spawn_blufor",
            "count": 1
        })
    }

    fn wave_and_garrison() -> Value {
        json!([wave(), garrison()])
    }

    fn compile_env_with_modules(block: &Value) -> Value {
        compile_payload(
            &json!({
                "meta": {
                    "terrain": "everon",
                    "environment": { "weather": "clear", "spawnModules": block }
                }
            })
            .to_string(),
            "{}",
            false,
        )
    }

    #[test]
    fn a_wave_and_garrison_block_parses() {
        let got = parse(&wave_and_garrison()).expect("parses");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].kind, "wave");
        assert_eq!(got[0].faction_key, "opfor");
        assert_eq!(got[0].x, Some(1200.0));
        assert_eq!(got[0].interval_seconds, Some(45.0));
        assert_eq!(got[0].max_alive, Some(4));
        assert_eq!(got[1].kind, "garrison");
        assert_eq!(got[1].zone_id.as_deref(), Some("z_spawn_blufor"));
        assert!(got[1].x.is_none());
    }

    #[test]
    fn both_position_and_zone_are_refused() {
        assert!(
            !placement_is_exclusive(true, true),
            "the predicate itself must refuse both"
        );
        assert!(placement_is_exclusive(true, false));
        assert!(placement_is_exclusive(false, true));
        assert!(!placement_is_exclusive(false, false));

        let err = parse(&json!([{
            "id": "sm-both",
            "kind": "garrison",
            "factionKey": "blufor",
            "groupTemplate": "Group_Base",
            "x": 1.0,
            "z": 2.0,
            "zoneId": "z1",
            "count": 1
        }]))
        .expect_err("both");
        assert!(err.contains("zoneId"), "{err}");
        assert!(err.contains("never both"), "{err}");
    }

    #[test]
    fn neither_position_nor_zone_is_refused() {
        let err = parse(&json!([{
            "id": "sm-none",
            "kind": "wave",
            "factionKey": "opfor",
            "groupTemplate": "Group_Base",
            "count": 1
        }]))
        .expect_err("neither");
        assert!(
            err.contains("never neither") || err.contains("zoneId"),
            "{err}"
        );
    }

    #[test]
    fn incomplete_position_is_refused() {
        let err = parse(&json!([{
            "id": "sm-x",
            "kind": "wave",
            "factionKey": "opfor",
            "groupTemplate": "Group_Base",
            "x": 1.0,
            "count": 1
        }]))
        .expect_err("x only");
        assert!(err.contains("together"), "{err}");
    }

    #[test]
    fn an_unknown_faction_is_refused() {
        let err = parse(&json!([{
            "id": "sm-navy",
            "kind": "wave",
            "factionKey": "navy",
            "groupTemplate": "Group_Base",
            "x": 1.0,
            "z": 2.0,
            "count": 1
        }]))
        .expect_err("faction");
        assert!(err.contains("navy"), "{err}");
        assert!(err.contains("blufor"), "{err}");
    }

    #[test]
    fn an_unknown_kind_is_refused() {
        let err = parse(&json!([{
            "id": "sm-patrol",
            "kind": "patrol",
            "factionKey": "blufor",
            "groupTemplate": "Group_Base",
            "x": 1.0,
            "z": 2.0,
            "count": 1
        }]))
        .expect_err("kind");
        assert!(err.contains("patrol"), "{err}");
        assert!(err.contains("wave"), "{err}");
    }

    #[test]
    fn zero_and_over_cap_counts_are_refused() {
        let err = parse(&json!([{
            "id": "sm-zero",
            "kind": "wave",
            "factionKey": "opfor",
            "groupTemplate": "Group_Base",
            "x": 1.0,
            "z": 2.0,
            "count": 0
        }]))
        .expect_err("zero");
        assert!(err.contains("1..="), "{err}");

        let err = parse(&json!([{
            "id": "sm-cap",
            "kind": "wave",
            "factionKey": "opfor",
            "groupTemplate": "Group_Base",
            "x": 1.0,
            "z": 2.0,
            "count": 1,
            "maxAlive": 99
        }]))
        .expect_err("cap");
        assert!(err.contains("32"), "{err}");
    }

    #[test]
    fn a_non_positive_interval_is_refused() {
        let err = parse(&json!([{
            "id": "sm-int",
            "kind": "wave",
            "factionKey": "opfor",
            "groupTemplate": "Group_Base",
            "x": 1.0,
            "z": 2.0,
            "count": 1,
            "intervalSeconds": 0
        }]))
        .expect_err("interval");
        assert!(err.contains("above zero"), "{err}");
    }

    #[test]
    fn an_unknown_key_is_refused() {
        let err = parse(&json!([{
            "id": "sm-extra",
            "kind": "garrison",
            "factionKey": "blufor",
            "groupTemplate": "Group_Base",
            "zoneId": "z1",
            "count": 1,
            "behaviour": "defend"
        }]))
        .expect_err("unknown");
        assert!(err.contains("behaviour"), "{err}");
    }

    #[test]
    fn empty_and_non_array_are_refused() {
        let err = parse(&json!([])).expect_err("empty");
        assert!(err.contains("empty"), "{err}");
        let err = parse(&json!({"id": "sm-1"})).expect_err("object");
        assert!(err.contains("array"), "{err}");
    }

    #[test]
    fn a_duplicate_id_is_refused() {
        let err = parse(&json!([
            {
                "id": "same",
                "kind": "wave",
                "factionKey": "opfor",
                "groupTemplate": "A",
                "x": 1.0,
                "z": 2.0,
                "count": 1
            },
            {
                "id": "same",
                "kind": "garrison",
                "factionKey": "blufor",
                "groupTemplate": "B",
                "zoneId": "z1",
                "count": 1
            }
        ]))
        .expect_err("dup");
        assert!(err.contains("unique"), "{err}");
    }

    #[test]
    fn spawn_modules_is_registered_on_the_carrier() {
        assert!(
            is_authored_block("spawnModules"),
            "T-936.6's row must be in AUTHORED_BLOCKS or the carrier never emits it"
        );
        assert!(
            !crate::mission::extensions::DOCUMENT_OWNED_BLOCKS.contains(&"spawnModules"),
            "spawnModules is optional — it rides ExtensionBlocks"
        );
        assert_eq!(KINDS, ["wave", "garrison"]);
        assert_eq!(FACTION_KEYS, ["blufor", "opfor", "indfor", "civ"]);
        assert_eq!(MAX_ALIVE, 32);
    }

    #[test]
    fn a_wave_and_garrison_mission_copies_to_the_payload_root() {
        let block = wave_and_garrison();
        let p = compile_env_with_modules(&block);
        assert_eq!(
            p["spawnModules"], block,
            "AUTHORED_BLOCKS must promote spawnModules out of the env bag: {p:#}"
        );
        assert_eq!(p["spawnModules"].as_array().expect("array").len(), 2);
        assert_eq!(p["spawnModules"][0]["kind"], "wave");
        assert_eq!(p["spawnModules"][1]["kind"], "garrison");

        let (carried, refusals) = ExtensionBlocks::from_payload(&p);
        assert!(refusals.is_empty(), "{refusals:?}");
        assert_eq!(carried.get("spawnModules"), Some(&block));
    }

    #[test]
    fn an_unauthored_payload_still_omits_the_spawn_modules_key() {
        let p = compile_payload(
            &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}})
                .to_string(),
            "{}",
            false,
        );
        assert!(
            p.get("spawnModules").is_none(),
            "parity: no spawnModules authored ⇒ no spawnModules key: {p:#}"
        );
        let (carried, refusals) = ExtensionBlocks::from_payload(&p);
        assert!(refusals.is_empty(), "{refusals:?}");
        assert!(carried.get("spawnModules").is_none());
    }

    /// The unlisted-key witness now uses T-936.7's `tacticalGraphics`.
    #[test]
    fn an_unlisted_environment_key_is_not_promoted() {
        let env = json!({"weather": "clear", "tacticalGraphics": []});
        let mut dst = serde_json::Map::new();
        let copied = copy_authored_blocks(&env, &mut dst);
        assert!(!copied.contains(&"tacticalGraphics"), "{copied:?}");
        assert!(
            !dst.contains_key("tacticalGraphics"),
            "an unlisted key stays parked: {dst:?}"
        );
    }
}
