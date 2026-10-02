//! Role: zones.
//! Position: `doc/operations/entity` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::DrawTarget;
use super::MissionDocCore;

/// One authored zone, read back for the dock list and the Attributes panel.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneRow {
    /// Id.
    pub id: String,

    /// Schema `zone.type`.
    pub kind: String,

    /// Label.
    pub label: Option<String>,

    /// Faction.
    pub faction: Option<String>,

    /// Rules.
    pub rules: serde_json::Value,

    /// `Some((x, z, r))` for a circle.
    pub circle: Option<(f64, f64, f64)>,

    /// The ring for a polygon.
    pub polygon: Vec<(f64, f64)>,
}

impl ZoneRow {
    /// A one-line geometry summary for the dock row.
    #[must_use]
    pub fn shape_summary(&self) -> String {
        if let Some((x, z, r)) = self.circle {
            format!("circle r {r:.1} m @ {x:.0}, {z:.0}")
        } else if self.polygon.is_empty() {
            "no shape".to_string()
        } else {
            format!("polygon, {} vertices", self.polygon.len())
        }
    }
}

/// Canonical trigger activations value.
pub const TRIGGER_ACTIVATIONS: &[&str] = &["presence", "radio", "timer"];

/// One authored trigger, read back for the palette list and the Attributes panel. Mirrors [`ZoneRow`], plus the trigger-only `name` / `owner_id` / `activation`.
#[derive(Clone, Debug, PartialEq)]
pub struct TriggerRow {
    /// Id.
    pub id: String,

    /// Name.
    pub name: Option<String>,

    /// CONN-TRG-OWNER-001 — the linked placed entity, or `None` (unowned). May be DANGLING: the entity it names can have been deleted; readers resolve it to nothing, they do not clear it.
    pub owner_id: Option<String>,

    /// One of [`TRIGGER_ACTIVATIONS`] (stored, not evaluated).
    pub activation: String,

    /// Rules.
    pub rules: serde_json::Value,

    /// `Some((x, z, r))` for a circle.
    pub circle: Option<(f64, f64, f64)>,

    /// The ring for a polygon.
    pub polygon: Vec<(f64, f64)>,
}

impl TriggerRow {
    /// A one-line geometry summary for the palette row (the [`ZoneRow::shape_summary`] twin).
    #[must_use]
    pub fn shape_summary(&self) -> String {
        if let Some((x, z, r)) = self.circle {
            format!("circle r {r:.1} m @ {x:.0}, {z:.0}")
        } else if self.polygon.is_empty() {
            "no shape".to_string()
        } else {
            format!("polygon, {} vertices", self.polygon.len())
        }
    }

    /// The trigger's geometric CENTRE in world metres — a circle's centre, or a polygon's vertex mean. `None` for a shapeless row. This is the trigger end of the owner-link line.
    #[must_use]
    pub fn centre(&self) -> Option<(f64, f64)> {
        if let Some((x, z, _)) = self.circle {
            return Some((x, z));
        }
        if self.polygon.is_empty() {
            return None;
        }
        let n = self.polygon.len() as f64;
        let (sx, sz) = self
            .polygon
            .iter()
            .fold((0.0, 0.0), |(ax, az), (x, z)| (ax + x, az + z));
        Some((sx / n, sz / n))
    }
}

/// One entry the Owner picker offers: a placed entity's id and a human label. CONN-TRG-OWNER-001 — "listing placed entities (slots/vehicles by label)".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerOption {
    /// Id.
    pub id: String,

    /// Label.
    pub label: String,
}

/// Apply zone_rows to explicit document state.
pub fn zone_rows(core: &MissionDocCore) -> Option<Vec<ZoneRow>> {
    let map: serde_json::Value = serde_json::from_str(&core.zones_json()).ok()?;
    let obj = map.as_object()?;
    let mut rows: Vec<ZoneRow> = obj
        .iter()
        .map(|(id, z)| {
            let shape = z.get("shape");
            let circle = shape.and_then(|s| s.get("circle")).and_then(|c| {
                Some((
                    c.get("x")?.as_f64()?,
                    c.get("z")?.as_f64()?,
                    c.get("r")?.as_f64()?,
                ))
            });
            let polygon = shape
                .and_then(|s| s.get("polygon"))
                .and_then(serde_json::Value::as_array)
                .map(|ring| {
                    ring.iter()
                        .filter_map(|p| {
                            let a = p.as_array()?;
                            Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
                        })
                        .collect()
                })
                .unwrap_or_default();
            ZoneRow {
                id: id.clone(),
                kind: z
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default()
                    .to_string(),
                label: z
                    .get("label")
                    .and_then(|l| l.as_str())
                    .map(ToString::to_string),
                faction: z
                    .get("faction")
                    .and_then(|f| f.as_str())
                    .map(ToString::to_string),
                rules: z.get("rules").cloned().unwrap_or(serde_json::Value::Null),
                circle,
                polygon,
            }
        })
        .collect();

    rows.sort_by(|a, b| a.id.cmp(&b.id));
    Some(rows)
}

/// Apply trigger_rows to explicit document state.
pub fn trigger_rows(core: &MissionDocCore) -> Option<Vec<TriggerRow>> {
    let map: serde_json::Value = serde_json::from_str(&core.triggers_json()).ok()?;
    let obj = map.as_object()?;
    let mut rows: Vec<TriggerRow> = obj
        .iter()
        .map(|(id, t)| {
            let shape = t.get("shape");
            let circle = shape.and_then(|s| s.get("circle")).and_then(|c| {
                Some((
                    c.get("x")?.as_f64()?,
                    c.get("z")?.as_f64()?,
                    c.get("r")?.as_f64()?,
                ))
            });
            let polygon = shape
                .and_then(|s| s.get("polygon"))
                .and_then(serde_json::Value::as_array)
                .map(|ring| {
                    ring.iter()
                        .filter_map(|p| {
                            let a = p.as_array()?;
                            Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
                        })
                        .collect()
                })
                .unwrap_or_default();
            TriggerRow {
                id: id.clone(),
                name: t
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(ToString::to_string),
                owner_id: t
                    .get("ownerId")
                    .and_then(|o| o.as_str())
                    .map(ToString::to_string),
                activation: t
                    .get("activation")
                    .and_then(|a| a.as_str())
                    .unwrap_or_default()
                    .to_string(),
                rules: t.get("rules").cloned().unwrap_or(serde_json::Value::Null),
                circle,
                polygon,
            }
        })
        .collect();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    Some(rows)
}

/// Apply set_zone_rule to explicit document state.
pub fn set_zone_rule(
    core: &MissionDocCore,
    id: &str,
    key: &str,
    value: Option<serde_json::Value>,
) -> Option<String> {
    let map: serde_json::Value = serde_json::from_str(&core.zones_json()).ok()?;
    let mut rules = map
        .get(id)?
        .get("rules")
        .and_then(|r| r.as_object().cloned())
        .unwrap_or_default();
    match value {
        Some(v) => {
            rules.insert(key.to_string(), v);
        }
        None => {
            rules.remove(key);
        }
    }
    Some(serde_json::Value::Object(rules).to_string())
}

/// Apply set_trigger_rule to explicit document state.
pub fn set_trigger_rule(
    core: &MissionDocCore,
    id: &str,
    key: &str,
    value: Option<serde_json::Value>,
) -> Option<String> {
    let map: serde_json::Value = serde_json::from_str(&core.triggers_json()).ok()?;
    let mut rules = map
        .get(id)?
        .get("rules")
        .and_then(|r| r.as_object().cloned())
        .unwrap_or_default();
    match value {
        Some(v) => {
            rules.insert(key.to_string(), v);
        }
        None => {
            rules.remove(key);
        }
    }
    Some(serde_json::Value::Object(rules).to_string())
}

/// Mint an unused id in `collection`'s OWN namespace (`z{n}` for zones, `t{n}` for triggers), proven unique against that collection's live map rather than assumed — undo frees ids and an IDB restore can bring back a document that already used one. Each collection is a separate namespace (the slot SoA does not contain either), so `mint_id`'s slot proof does not apply here.
pub fn mint_row_id(core: &MissionDocCore, collection: DrawTarget) -> String {
    let (json, prefix) = match collection {
        DrawTarget::Zone => (core.zones_json(), "z"),
        DrawTarget::Trigger => (core.triggers_json(), "t"),
    };
    let existing: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(&json)
            .ok()
            .and_then(|v| {
                v.as_object()
                    .map(|m| m.keys().map(ToString::to_string).collect())
            })
            .unwrap_or_default();
    (1u32..)
        .map(|n| format!("{prefix}{n}"))
        .find(|id| !existing.contains(id))
        .unwrap_or_else(|| format!("{prefix}1"))
}

/// Apply write_row_returning_id to explicit document state.
pub fn write_row_returning_id(
    core: &MissionDocCore,
    collection: DrawTarget,
    f: impl FnOnce(&MissionDocCore, &str),
) -> Option<String> {
    let id = mint_row_id(core, collection);
    f(core, &id);
    Some(id)
}
