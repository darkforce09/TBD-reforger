//! Settings catalog for the mission settings interface.

use super::*;

/// The mission schema used for declared settings and defaults.
pub(super) const MISSION_SCHEMA_JSON: &str =
    crate::v2::apps::editor::ui::inspector::zones_panel::MISSION_SCHEMA;

#[derive(Clone, Debug, PartialEq)]
/// The schema status and default for an authored setting.
pub enum SettingDefault {
    Schema {
        pointer: String,
        value: serde_json::Value,
    },
    Declared {
        pointer: String,
    },
    NotInSchema,
}

impl SettingDefault {
    #[must_use]
    fn from_schema_node(
        pointer: &str,
        node: &serde_json::Value,
        resolved: &serde_json::Value,
    ) -> Self {
        match node.get("default").or_else(|| resolved.get("default")) {
            Some(value) => Self::Schema {
                pointer: pointer.to_string(),
                value: value.clone(),
            },
            None => Self::Declared {
                pointer: pointer.to_string(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// The mission or entity that owns an authored setting.
pub enum SettingOwner {
    Mission,
    Entity {
        kind: &'static str,
        id: String,
        label: String,
    },
}

impl SettingOwner {
    #[must_use]
    /// Provides subject id for mission settings.
    pub fn subject_id(&self) -> Option<&str> {
        match self {
            Self::Mission => None,
            Self::Entity { id, .. } => Some(id.as_str()),
        }
    }

    #[must_use]
    /// Provides label for mission settings.
    pub fn label(&self) -> String {
        match self {
            Self::Mission => "Mission (this document)".to_string(),
            Self::Entity { kind, label, .. } => format!("{kind} — {label}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// An authored setting and its schema comparison.
pub struct SettingRow {
    pub key: String,
    pub owner: SettingOwner,
    pub value: serde_json::Value,
    pub default: SettingDefault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Whether an authored setting differs from its default.
pub enum DiffState {
    Differs,
    Matches,
    Unknown,
}

impl SettingRow {
    #[must_use]
    /// Provides diff state for mission settings.
    pub fn diff_state(&self) -> DiffState {
        match &self.default {
            SettingDefault::Schema { value, .. } => {
                if values_agree(&self.value, value) {
                    DiffState::Matches
                } else {
                    DiffState::Differs
                }
            }
            SettingDefault::Declared { .. } | SettingDefault::NotInSchema => DiffState::Unknown,
        }
    }

    #[must_use]
    /// Provides survives diff filter for mission settings.
    pub fn survives_diff_filter(&self) -> bool {
        self.diff_state() != DiffState::Matches
    }
}

#[must_use]
/// Compares authored and default JSON values.
pub(super) fn values_agree(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => (x - y).abs() <= f64::EPSILON * x.abs().max(y.abs()).max(1.0),
        _ => a == b,
    }
}

/// Schema pointers for mission-owned settings.
pub(super) const MISSION_SETTING_POINTERS: &[(&str, &str)] = &[
    ("terrain", "#/$defs/meta/properties/terrain"),
    ("time", "#/$defs/environment/properties/dateTime"),
    ("weather", "#/$defs/environment/properties/weatherPreset"),
    ("briefingSeconds", "#/$defs/flow/properties/briefingSeconds"),
    (
        "safeStartSeconds",
        "#/$defs/flow/properties/safeStartSeconds",
    ),
    (
        "timeLimitSeconds",
        "#/$defs/flow/properties/timeLimitSeconds",
    ),
    ("jip", "#/$defs/flow/properties/jip"),
];

#[must_use]
/// Builds a schema pointer for a zone rule.
pub(super) fn zone_rule_pointer(key: &str) -> String {
    format!("#/$defs/zoneRules/properties/{key}")
}

#[must_use]
/// Finds the schema pointer for a mission setting.
pub(super) fn mission_setting_pointer(key: &str) -> Option<&'static str> {
    MISSION_SETTING_POINTERS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, p)| *p)
}

#[must_use]
/// Reads a declared default from the mission schema.
pub(super) fn schema_default(schema: Option<&serde_json::Value>, pointer: &str) -> SettingDefault {
    let Some(schema) = schema else {
        return SettingDefault::NotInSchema;
    };
    let Some(node) = schema.pointer(pointer.trim_start_matches('#')) else {
        return SettingDefault::NotInSchema;
    };
    let resolved = node
        .get("$ref")
        .and_then(serde_json::Value::as_str)
        .filter(|r| r.starts_with("#/$defs/"))
        .and_then(|r| schema.pointer(r.trim_start_matches('#')))
        .unwrap_or(node);
    SettingDefault::from_schema_node(pointer, node, resolved)
}

#[must_use]
/// Parses the mission schema used by settings aggregation.
pub(super) fn mission_schema() -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(MISSION_SCHEMA_JSON).ok()
}

#[must_use]
/// Collects authored mission and entity settings.
pub fn aggregate_settings(root: &serde_json::Value) -> Vec<SettingRow> {
    let schema = mission_schema();
    let schema = schema.as_ref();
    let mut rows: Vec<SettingRow> = Vec::new();

    let mission_row = |key: &str, value: &serde_json::Value| SettingRow {
        key: key.to_string(),
        owner: SettingOwner::Mission,
        value: value.clone(),
        default: match mission_setting_pointer(key) {
            Some(p) => schema_default(schema, p),
            None => SettingDefault::NotInSchema,
        },
    };

    let meta = root.get("meta");
    if let Some(v) = meta.and_then(|m| m.get("terrain")) {
        rows.push(mission_row("terrain", v));
    }
    if let Some(env) = meta
        .and_then(|m| m.get("environment"))
        .and_then(serde_json::Value::as_object)
    {
        for (key, _) in MISSION_SETTING_POINTERS {
            if let Some(v) = env.get(*key) {
                rows.push(mission_row(key, v));
            }
        }
        let mut unlisted: Vec<&String> = env
            .keys()
            .filter(|k| mission_setting_pointer(k).is_none())
            .collect();
        unlisted.sort();
        for key in unlisted {
            rows.push(mission_row(key, &env[key]));
        }
    }

    if let Some(zones) = root.get("zonesById").and_then(serde_json::Value::as_object) {
        let mut ids: Vec<&String> = zones.keys().collect();
        ids.sort();
        for id in ids {
            let zone = &zones[id];
            let Some(rules) = zone.get("rules").and_then(serde_json::Value::as_object) else {
                continue;
            };
            let label = zone
                .get("label")
                .and_then(serde_json::Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .or_else(|| zone.get("type").and_then(serde_json::Value::as_str))
                .unwrap_or(id.as_str())
                .to_string();
            let mut keys: Vec<&String> = rules.keys().collect();
            keys.sort();
            for key in keys {
                rows.push(SettingRow {
                    key: key.clone(),
                    owner: SettingOwner::Entity {
                        kind: "Zone",
                        id: id.clone(),
                        label: label.clone(),
                    },
                    value: rules[key].clone(),
                    default: schema_default(schema, &zone_rule_pointer(key)),
                });
            }
        }
    }

    rows
}

#[must_use]
/// Formats an authored setting value for display.
pub fn fmt_setting_value(v: &serde_json::Value) -> String {
    match v.as_str() {
        Some("") => "(empty)".to_string(),
        Some(s) => s.to_string(),
        None => v.to_string(),
    }
}

#[must_use]
/// Formats a setting default for display.
pub fn fmt_setting_default(d: &SettingDefault) -> String {
    match d {
        SettingDefault::Schema { value, .. } => fmt_setting_value(value),
        SettingDefault::Declared { .. } => NO_DEFAULT_DECLARED.to_string(),
        SettingDefault::NotInSchema => NOT_A_SCHEMA_KEY.to_string(),
    }
}
