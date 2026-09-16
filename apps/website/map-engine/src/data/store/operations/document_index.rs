//! Role: document index.
//! Position: `doc/operations` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::attrs::{raw_slot_rows, row_str};
use super::entity::*;
use super::projections::{faction_rows, layer_rows, squad_rows};
use crate::data::store::MissionDocCore;
use std::collections::HashMap;

/// Domain representation of doc kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DocKind {
    /// A `slots` row (an ORBAT player/AI slot).
    Slot,

    /// A `vehiclesById` row.
    Vehicle,

    /// Domain representation of object.
    Object,

    /// Domain representation of marker.
    Marker,

    /// Domain representation of zone.
    Zone,

    /// A `triggers` row.
    Trigger,

    /// Domain representation of comment.
    Comment,

    /// An `editorLayersById` folder.
    Layer,
}

impl DocKind {
    /// The badge noun, singular.
    #[must_use]
    pub fn noun(self) -> &'static str {
        match self {
            DocKind::Slot => "slot",
            DocKind::Vehicle => "vehicle",
            DocKind::Object => "object",
            DocKind::Marker => "marker",
            DocKind::Zone => "zone",
            DocKind::Trigger => "trigger",
            DocKind::Comment => "comment",
            DocKind::Layer => "layer",
        }
    }

    /// The row glyph (Material Symbols name), matching the icon each kind's own panel already uses.
    #[must_use]
    pub fn icon(self) -> &'static str {
        match self {
            DocKind::Slot => "person",
            DocKind::Vehicle => "directions_car",
            DocKind::Object => "category",
            DocKind::Marker => "place",
            DocKind::Zone => "crop_square",
            DocKind::Trigger => "bolt",
            DocKind::Comment => "sticky_note_2",
            DocKind::Layer => "folder",
        }
    }
}

/// Domain representation of doc entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocEntity {
    /// The doc id — what the click-to-select router is handed.
    pub id: String,

    /// Kind.
    pub kind: DocKind,

    /// The row's display name (already fallen back: never empty).
    pub label: String,

    /// The Enfusion `resourceName` / object alias this row spawns as, empty when it has none. This is the datum `class:` matches, and only this one.
    pub class_name: String,

    /// The side this row belongs to (`BLUFOR` / `OPFOR` / `INDFOR`, or a library faction's key), empty when the kind carries none. The datum `mod:` matches, and the selection filter's faction axis.
    pub faction: String,

    /// Text.
    pub text: Vec<(&'static str, String)>,
}

/// Side label using the supplied domain data.
pub fn side_label(raw: &str) -> String {
    raw.strip_prefix("faction-").unwrap_or(raw).to_uppercase()
}

/// Or fallback using the supplied domain data.
pub fn or_fallback(label: &str, fallback: &str) -> String {
    let t = label.trim();
    if t.is_empty() {
        fallback.to_string()
    } else {
        t.to_string()
    }
}

/// Push text using the supplied domain data.
pub fn push_text(text: &mut Vec<(&'static str, String)>, field: &'static str, value: &str) {
    let v = value.trim();
    if !v.is_empty() {
        text.push((field, v.to_string()));
    }
}

/// Document entities using the supplied domain data.
#[must_use]
pub fn document_entities(core: &MissionDocCore) -> Vec<DocEntity> {
    let mut out: Vec<DocEntity> = Vec::new();

    let side_of_faction: HashMap<String, String> = faction_rows(core)
        .into_iter()
        .map(|f| {
            let key = if f.key.is_empty() { f.name } else { f.key };
            (f.id, side_label(&key))
        })
        .collect();
    let side_of_squad: HashMap<String, String> = squad_rows(core)
        .into_iter()
        .map(|s| {
            let side = side_of_faction
                .get(&s.faction_id)
                .cloned()
                .unwrap_or_default();
            (s.id, side)
        })
        .collect();

    let raw = raw_slot_rows(core);
    for s in slot_details(core) {
        let asset_id = row_str(&raw, &s.id, "assetId");
        let description = row_str(&raw, &s.id, "description");
        let mut text = Vec::new();
        push_text(&mut text, "role", &s.role);
        push_text(&mut text, "callsign", &s.callsign);
        push_text(&mut text, "tag", &s.tag);
        push_text(&mut text, "rank", &s.rank);
        push_text(&mut text, "description", &description);
        push_text(&mut text, "loadout", &s.summary);
        push_text(&mut text, "class", super::assets::classname_tail(&asset_id));
        push_text(&mut text, "id", &s.id);
        out.push(DocEntity {
            label: or_fallback(&s.role, &format!("Slot {}", s.id)),
            faction: side_of_squad.get(&s.squad_id).cloned().unwrap_or_default(),
            class_name: asset_id,
            kind: DocKind::Slot,
            id: s.id,
            text,
        });
    }

    if let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json())
        && let Some(map) = root.get("entitiesById").and_then(|v| v.as_object())
    {
        for (id, v) in map {
            let s = |k: &str| {
                v.get(k)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            };
            let alias = s("alias");
            let resource_name = s("resourceName");
            let mut text = Vec::new();
            push_text(&mut text, "alias", &alias);
            push_text(
                &mut text,
                "class",
                super::assets::classname_tail(&resource_name),
            );
            push_text(&mut text, "id", id);
            out.push(DocEntity {
                id: id.clone(),
                kind: DocKind::Object,
                label: or_fallback(&alias, &format!("Object {id}")),
                faction: side_label(&s("faction")),
                class_name: resource_name,
                text,
            });
        }
    }

    for cm in comment_details(core) {
        let mut text = Vec::new();
        push_text(&mut text, "title", &cm.title);
        push_text(&mut text, "note", &cm.tooltip);
        push_text(&mut text, "id", &cm.id);
        out.push(DocEntity {
            label: or_fallback(&cm.title, &format!("Comment {}", cm.id)),
            kind: DocKind::Comment,
            class_name: String::new(),
            faction: String::new(),
            id: cm.id,
            text,
        });
    }

    for l in layer_rows(core) {
        let mut text = Vec::new();
        push_text(&mut text, "name", &l.name);
        push_text(&mut text, "id", &l.id);
        out.push(DocEntity {
            label: or_fallback(&l.name, &format!("Layer {}", l.id)),
            kind: DocKind::Layer,
            class_name: String::new(),
            faction: String::new(),
            id: l.id,
            text,
        });
    }

    for v in vehicle_rows(core) {
        let tail = super::assets::classname_tail(&v.resource_name).to_string();
        let mut text = Vec::new();
        push_text(&mut text, "class", &tail);
        push_text(&mut text, "id", &v.id);
        out.push(DocEntity {
            label: or_fallback(&tail, &format!("Vehicle {}", v.id)),
            kind: DocKind::Vehicle,
            faction: side_label(&v.faction_id),
            class_name: v.resource_name,
            id: v.id,
            text,
        });
    }
    for z in zone_rows(core).unwrap_or_default() {
        let label = z.label.clone().unwrap_or_default();
        let mut text = Vec::new();
        push_text(&mut text, "label", &label);
        push_text(&mut text, "type", &z.kind);
        push_text(&mut text, "id", &z.id);
        out.push(DocEntity {
            label: or_fallback(&label, &format!("Zone {}", z.id)),
            kind: DocKind::Zone,
            class_name: String::new(),
            faction: side_label(z.faction.as_deref().unwrap_or_default()),
            id: z.id,
            text,
        });
    }
    for t in trigger_rows(core).unwrap_or_default() {
        let name = t.name.clone().unwrap_or_default();
        let mut text = Vec::new();
        push_text(&mut text, "name", &name);
        push_text(&mut text, "activation", &t.activation);
        push_text(&mut text, "id", &t.id);
        out.push(DocEntity {
            label: or_fallback(&name, &format!("Trigger {}", t.id)),
            kind: DocKind::Trigger,
            class_name: String::new(),
            faction: String::new(),
            id: t.id,
            text,
        });
    }
    for m in marker_rows_of(core) {
        let mut text = Vec::new();
        push_text(&mut text, "caption", &m.label);
        push_text(&mut text, "icon", &m.icon);
        push_text(&mut text, "id", &m.id);
        out.push(DocEntity {
            label: or_fallback(&m.label, &or_fallback(&m.icon, &format!("Marker {}", m.id))),
            kind: DocKind::Marker,
            class_name: String::new(),
            faction: side_label(&m.faction_id),
            id: m.id,
            text,
        });
    }
    out
}
