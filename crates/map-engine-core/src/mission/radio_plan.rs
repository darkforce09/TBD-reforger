//! T-936.3 — the authored `radioPlan` block: nets, frequencies, and the collision gates.
//!
//! ══ Why this module exists ══════════════════════════════════════════════════════════════════
//! `flatten.rs` has synthesized every net since T-203 (`NET_FREQ_BASE_MHZ + STEP × index`). The
//! editor had no radio UI and the payload had no `radioPlan`, so every mission received the same
//! mechanical allocation. `$defs/radioPlan` / `$defs/net` already exist (freqMHz minimum 30,
//! maximum 512; `maxItems` 32) and `Radio/TBD_RadioPlan.c` already parses them. This module is
//! the authoring gate: a list the editor wrote reaches the compiled document unchanged; a
//! mission that wrote none still gets today's derived plan byte-for-byte.
//!
//! ══ The AUTHORED_BLOCKS row ═════════════════════════════════════════════════════════════════
//! [`validate`] is what `extensions.rs` registers. The block is DOCUMENT-MODELLED — `ModMissionDocument`
//! already has a typed `radio_plan` field — so it is listed in [`crate::mission::extensions::DOCUMENT_OWNED_BLOCKS`]
//! and withheld from the carrier. Flatten chooses the authored plan when [`parse`] accepts it,
//! otherwise it runs `derive_radio_plan` exactly as before.
//!
//! ══ Perturbation ════════════════════════════════════════════════════════════════════════════
//! [`refuse_duplicate_frequency`] is the target: dropping the call in [`parse`] (or making it
//! always `Ok`) must turn [`tests::a_duplicate_frequency_is_refused`] red.

use serde_json::{Map, Value};

/// `$defs/net/freqMHz` `minimum: 30` — also T-203's allocation floor.
pub const FREQ_MIN_MHZ: f64 = 30.0;

/// `$defs/net/freqMHz` `maximum: 512`.
pub const FREQ_MAX_MHZ: f64 = 512.0;

/// `$defs/radioPlan.nets` `maxItems` and `TBD_RadioPlan.MAX_NETS`.
pub const MAX_NETS: usize = 32;

/// `$defs/net.label` `maxLength` and `TBD_RadioPlan.MAX_LABEL_CHARS`.
pub const MAX_LABEL_CHARS: usize = 48;

/// `$defs/net.range` vocabulary.
pub const RANGES: &[&str] = &["short", "long"];

/// One authored net, parsed and checked.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredNet {
    pub id: String,
    pub label: String,
    pub freq_mhz: f64,
    pub faction: Option<String>,
    pub range: Option<String>,
}

/// One authored `radioPlan` object (`{ nets: [...] }`).
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredRadioPlan {
    pub nets: Vec<AuthoredNet>,
}

/// Compare two frequencies at 1 kHz so `30` and `30.0` collide and `30.0` vs `30.5` do not.
#[must_use]
pub fn freq_key(mhz: f64) -> i64 {
    (mhz * 1000.0).round() as i64
}

/// Refuse a net whose frequency is already used by an earlier net.
///
/// Two sides on one frequency would hear each other — that is the one way a frequency choice
/// can be actively wrong. **Perturbation target.**
///
/// # Errors
/// Returns a sentence naming both nets when `candidate` matches an earlier frequency.
pub fn refuse_duplicate_frequency(
    nets: &[AuthoredNet],
    candidate: f64,
    index: usize,
) -> Result<(), String> {
    let key = freq_key(candidate);
    if let Some((other, n)) = nets
        .iter()
        .enumerate()
        .find(|(_, n)| freq_key(n.freq_mhz) == key)
    {
        return Err(format!(
            "`radioPlan.nets[{index}]`.freqMHz is {candidate} — the same frequency as \
             nets[{other}] ({}) so the two channels would hear each other",
            n.id
        ));
    }
    Ok(())
}

/// Parse and validate one authored `radioPlan` value.
///
/// # Errors
/// Returns the refusal clause when the value is not `{nets: [...]}`, a net is malformed, a
/// frequency is outside 30..=512, two nets share a frequency or id, or the list exceeds
/// [`MAX_NETS`].
pub fn parse(value: &Value) -> Result<AuthoredRadioPlan, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`radioPlan` must be an object, not {}",
            type_name(value)
        ));
    };

    for key in obj.keys() {
        if key != "nets" {
            return Err(format!(
                "`radioPlan` carries {key:?}, which `$defs/radioPlan` does not declare \
                 (additionalProperties is false)"
            ));
        }
    }

    let Some(arr) = obj.get("nets").and_then(Value::as_array) else {
        return Err("`radioPlan.nets` is required and must be an array of one or more nets".into());
    };
    if arr.is_empty() {
        return Err(
            "`radioPlan.nets` is empty — the schema requires at least one net, and an empty \
             plan is omitted rather than authored"
                .into(),
        );
    }
    if arr.len() > MAX_NETS {
        return Err(format!(
            "`radioPlan.nets` has {} entries — `TBD_RadioPlan.MAX_NETS` is {MAX_NETS} and the \
             mod would silently drop the rest",
            arr.len()
        ));
    }

    let mut out = Vec::with_capacity(arr.len());
    let mut seen_ids: Vec<String> = Vec::with_capacity(arr.len());

    for (index, item) in arr.iter().enumerate() {
        let net = parse_net(item, index)?;
        refuse_duplicate_frequency(&out, net.freq_mhz, index)?;
        if seen_ids.iter().any(|id| id == &net.id) {
            return Err(format!(
                "`radioPlan.nets[{index}]`.id is {} — each net id must be unique",
                quote(&net.id)
            ));
        }
        seen_ids.push(net.id.clone());
        out.push(net);
    }

    Ok(AuthoredRadioPlan { nets: out })
}

/// [`parse`] with the value discarded — the [`crate::mission::extensions::AUTHORED_BLOCKS`] row's
/// validator.
///
/// # Errors
/// Every error [`parse`] returns.
pub fn validate(value: &Value) -> Result<(), String> {
    parse(value).map(|_| ())
}

fn parse_net(value: &Value, index: usize) -> Result<AuthoredNet, String> {
    let Some(obj) = value.as_object() else {
        return Err(format!(
            "`radioPlan.nets[{index}]` must be an object, not {}",
            type_name(value)
        ));
    };

    for key in obj.keys() {
        if !KNOWN_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "`radioPlan.nets[{index}]` carries {key:?}, which `$defs/net` does not declare \
                 (additionalProperties is false)"
            ));
        }
    }

    let id = required_string(obj, index, "id")?;
    if !is_net_id(&id) {
        return Err(format!(
            "`radioPlan.nets[{index}]`.id is {} — a net id is `net:` then lowercase letters, \
             digits and underscores",
            quote(&id)
        ));
    }

    let label = required_string(obj, index, "label")?;
    let label_chars = label.chars().count();
    if !(1..=MAX_LABEL_CHARS).contains(&label_chars) {
        return Err(format!(
            "`radioPlan.nets[{index}]`.label is {label_chars} characters — `$defs/net.label` \
             allows 1..={MAX_LABEL_CHARS} (the mod truncates past that without a word)"
        ));
    }

    let freq_mhz = required_freq(obj, index)?;
    let faction = optional_string(obj, index, "faction")?;
    if let Some(ref f) = faction
        && !is_faction_key(f)
    {
        return Err(format!(
            "`radioPlan.nets[{index}]`.faction is {} — a faction key starts with a letter and \
             is lowercase letters, digits and underscores",
            quote(f)
        ));
    }

    let range = optional_string(obj, index, "range")?;
    if let Some(ref r) = range
        && !RANGES.contains(&r.as_str())
    {
        return Err(format!(
            "`radioPlan.nets[{index}]`.range is {} — the editor authors one of {}",
            quote(r),
            RANGES.join(", ")
        ));
    }

    Ok(AuthoredNet {
        id,
        label,
        freq_mhz,
        faction,
        range,
    })
}

const KNOWN_KEYS: &[&str] = &["id", "label", "freqMHz", "faction", "range"];

/// `^net:[a-z0-9_]+$`
fn is_net_id(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("net:") else {
        return false;
    };
    !rest.is_empty()
        && rest
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// `^[a-z][a-z0-9_]*$`
fn is_faction_key(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {
            chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        }
        _ => false,
    }
}

fn required_string(obj: &Map<String, Value>, index: usize, key: &str) -> Result<String, String> {
    let Some(raw) = obj.get(key) else {
        return Err(format!(
            "`radioPlan.nets[{index}].{key}` is required and is missing"
        ));
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`radioPlan.nets[{index}].{key}` must be a string, not {}",
            type_name(raw)
        ));
    };
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "`radioPlan.nets[{index}].{key}` is blank — a net needs a {key} the radio UI can name"
        ));
    }
    Ok(trimmed.to_string())
}

fn optional_string(
    obj: &Map<String, Value>,
    index: usize,
    key: &str,
) -> Result<Option<String>, String> {
    let Some(raw) = obj.get(key) else {
        return Ok(None);
    };
    let Some(s) = raw.as_str() else {
        return Err(format!(
            "`radioPlan.nets[{index}].{key}` must be a string, not {}",
            type_name(raw)
        ));
    };
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "`radioPlan.nets[{index}].{key}` is blank — omit the key rather than authoring an \
             empty {key}"
        ));
    }
    Ok(Some(trimmed.to_string()))
}

fn required_freq(obj: &Map<String, Value>, index: usize) -> Result<f64, String> {
    let Some(raw) = obj.get("freqMHz") else {
        return Err(format!(
            "`radioPlan.nets[{index}].freqMHz` is required and is missing"
        ));
    };
    let Some(freq) = raw.as_f64() else {
        return Err(format!(
            "`radioPlan.nets[{index}].freqMHz` must be a number, not {}",
            type_name(raw)
        ));
    };
    if !freq.is_finite() || !(FREQ_MIN_MHZ..=FREQ_MAX_MHZ).contains(&freq) {
        return Err(format!(
            "`radioPlan.nets[{index}].freqMHz` is {freq} — `$defs/net.freqMHz` allows \
             {FREQ_MIN_MHZ}..={FREQ_MAX_MHZ}"
        ));
    }
    Ok(freq)
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
    use crate::mission::extensions::{
        AuthoredBlocks, DOCUMENT_OWNED_BLOCKS, copy_authored_blocks, is_authored_block,
    };
    use serde_json::json;

    fn one_net() -> Value {
        json!({
            "nets": [{
                "id": "net:blufor_cmd",
                "label": "Command",
                "freqMHz": 30.0,
                "faction": "blufor",
                "range": "long"
            }]
        })
    }

    fn compile_env_with_plan(plan: &Value) -> Value {
        compile_payload(
            &json!({
                "meta": {
                    "terrain": "everon",
                    "environment": { "weather": "clear", "radioPlan": plan }
                }
            })
            .to_string(),
            "{}",
            false,
        )
    }

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

    /// **The perturbation target.** Deleting the [`refuse_duplicate_frequency`] call in [`parse`]
    /// (or making that function always return `Ok`) flips this test from red-when-dropped back to
    /// green-when-restored.
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
            &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}})
                .to_string(),
            "{}",
            false,
        );
        assert!(
            p.get("radioPlan").is_none(),
            "parity: no radioPlan authored ⇒ no radioPlan key on the payload: {p:#}"
        );
    }
}
