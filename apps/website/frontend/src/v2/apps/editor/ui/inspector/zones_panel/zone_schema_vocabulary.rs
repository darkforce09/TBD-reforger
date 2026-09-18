//! Zones panel zone schema vocabulary.

use super::*;

/// Embedded mission schema used for zone types and rule fields.
pub(crate) const MISSION_SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts_v2/definitions/mission.schema.json"
));

/// One editable zone rule derived from the mission schema.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneRuleField {
    pub key: String,
    pub kind: ZoneRuleKind,
    pub doc: String,
}

/// Control shape inferred from a zone rule schema property.
#[derive(Clone, Debug, PartialEq)]
pub enum ZoneRuleKind {
    Bool {
        default: bool,
    },
    Choice {
        options: Vec<String>,
        default: Option<String>,
    },
    Text {
        default: Option<String>,
        pattern: Option<String>,
    },
    Number {
        default: Option<f64>,
        minimum: Option<f64>,
        exclusive_minimum: Option<f64>,
        maximum: Option<f64>,
        integer: bool,
    },
}

fn resolve_ref<'a>(
    schema: &'a serde_json::Value,
    node: &'a serde_json::Value,
) -> &'a serde_json::Value {
    let Some(r) = node.get("$ref").and_then(serde_json::Value::as_str) else {
        return node;
    };
    r.strip_prefix("#/$defs/")
        .and_then(|name| schema.get("$defs").and_then(|d| d.get(name)))
        .unwrap_or(node)
}

/// Reads the zone rule control vocabulary from the schema.
#[must_use]
pub fn zone_rule_fields() -> Vec<ZoneRuleField> {
    let Ok(schema) = serde_json::from_str::<serde_json::Value>(MISSION_SCHEMA) else {
        return Vec::new();
    };
    let Some(props) = schema
        .get("$defs")
        .and_then(|d| d.get("zoneRules"))
        .and_then(|z| z.get("properties"))
        .and_then(serde_json::Value::as_object)
    else {
        return Vec::new();
    };
    props
        .iter()
        .map(|(key, raw)| {
            let node = resolve_ref(&schema, raw);
            let doc = raw
                .get("description")
                .or_else(|| node.get("description"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let default = raw.get("default").or_else(|| node.get("default"));
            let ty = node.get("type").and_then(serde_json::Value::as_str);
            let enum_opts = node.get("enum").and_then(serde_json::Value::as_array);
            let kind = match (ty, enum_opts) {
                (Some("boolean"), _) => ZoneRuleKind::Bool {
                    default: default
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false),
                },
                (_, Some(opts)) => ZoneRuleKind::Choice {
                    options: opts
                        .iter()
                        .filter_map(|o| o.as_str().map(ToString::to_string))
                        .collect(),
                    default: default
                        .and_then(serde_json::Value::as_str)
                        .map(ToString::to_string),
                },
                (Some("number" | "integer"), _) => ZoneRuleKind::Number {
                    default: default.and_then(serde_json::Value::as_f64),
                    minimum: node.get("minimum").and_then(serde_json::Value::as_f64),
                    exclusive_minimum: node
                        .get("exclusiveMinimum")
                        .and_then(serde_json::Value::as_f64),
                    maximum: node.get("maximum").and_then(serde_json::Value::as_f64),
                    integer: ty == Some("integer"),
                },
                _ => ZoneRuleKind::Text {
                    default: default
                        .and_then(serde_json::Value::as_str)
                        .map(ToString::to_string),
                    pattern: node
                        .get("pattern")
                        .and_then(serde_json::Value::as_str)
                        .map(ToString::to_string),
                },
            };
            ZoneRuleField {
                key: key.clone(),
                kind,
                doc,
            }
        })
        .collect()
}

/// Reads authorable zone types from the schema.
#[must_use]
pub fn zone_types() -> Vec<String> {
    let Ok(schema) = serde_json::from_str::<serde_json::Value>(MISSION_SCHEMA) else {
        return Vec::new();
    };
    schema
        .get("$defs")
        .and_then(|d| d.get("zone"))
        .and_then(|z| z.get("properties"))
        .and_then(|p| p.get("type"))
        .and_then(|t| t.get("enum"))
        .and_then(serde_json::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Formats a schema token as a human-readable label.
#[must_use]
pub fn humanize_token(token: &str) -> String {
    let mut out = String::with_capacity(token.len());
    for (i, part) in token.split('_').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let mut cs = part.chars();
        if let Some(f) = cs.next() {
            if i == 0 {
                out.extend(f.to_uppercase());
            } else {
                out.push(f);
            }
            out.push_str(cs.as_str());
        }
    }
    out
}

/// Formats a rule key as a human-readable label.
#[must_use]
pub fn humanize_key(key: &str) -> String {
    let mut out = String::with_capacity(key.len() + 4);
    for (i, c) in key.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push(' ');
            }
            out.extend(c.to_lowercase());
        } else if i == 0 {
            out.extend(c.to_uppercase());
        } else {
            out.push(c);
        }
    }
    out
}
