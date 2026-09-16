//! Role: zones.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Map;
use super::MapPrelim;
use super::MissionDocCore;
use super::Out;
use super::circle_shape_any;
use super::json_str_to_any;
use super::polygon_shape_any;
use yrs::Transact;
use yrs::types::ToJson;

impl MissionDocCore {
    /// Add circle zone using the supplied domain data.
    pub fn add_circle_zone(&self, id: &str, kind: &str, x: f64, z: f64, r: f64) {
        let mut txn = self.begin();
        let zone = self
            .zones
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        zone.insert(&mut txn, "type", kind);
        zone.insert(&mut txn, "shape", circle_shape_any(x, z, r));
    }
}

impl MissionDocCore {
    /// Label-free by construction: the draw tool authors the ring first and the author names the zone afterwards, which is two gestures and rightly two undo steps. A create that must land NAMED in ONE step is [`Self::add_polygon_zone_labelled`], which this delegates to so the two cannot drift.
    pub fn add_polygon_zone(&self, id: &str, kind: &str, points_flat: &[f64]) {
        self.add_polygon_zone_labelled(id, kind, points_flat, None);
    }
}

impl MissionDocCore {
    /// ═══ WHY THIS EXISTS AND IS NOT TWO CALLS ═══.
    pub fn add_polygon_zone_labelled(
        &self,
        id: &str,
        kind: &str,
        points_flat: &[f64],
        label: Option<&str>,
    ) {
        let mut txn = self.begin();
        let zone = self
            .zones
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        zone.insert(&mut txn, "type", kind);
        zone.insert(&mut txn, "shape", polygon_shape_any(points_flat));
        if let Some(l) = label {
            zone.insert(&mut txn, "label", l);
        }
    }
}

impl MissionDocCore {
    /// Set zone circle using the supplied domain data.
    pub fn set_zone_circle(&self, zone_id: &str, x: f64, z: f64, r: f64) {
        let mut txn = self.begin();
        if let Some(Out::YMap(zone)) = self.zones.get(&txn, zone_id) {
            zone.insert(&mut txn, "shape", circle_shape_any(x, z, r));
        }
    }
}

impl MissionDocCore {
    /// Set zone polygon using the supplied domain data.
    pub fn set_zone_polygon(&self, zone_id: &str, points_flat: &[f64]) {
        let mut txn = self.begin();
        if let Some(Out::YMap(zone)) = self.zones.get(&txn, zone_id) {
            zone.insert(&mut txn, "shape", polygon_shape_any(points_flat));
        }
    }
}

impl MissionDocCore {
    /// Set zone type using the supplied domain data.
    pub fn set_zone_type(&self, zone_id: &str, kind: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(zone)) = self.zones.get(&txn, zone_id) {
            zone.insert(&mut txn, "type", kind);
        }
    }
}

impl MissionDocCore {
    /// Set zone label using the supplied domain data.
    pub fn set_zone_label(&self, zone_id: &str, label: Option<&str>) {
        let mut txn = self.begin();
        if let Some(Out::YMap(zone)) = self.zones.get(&txn, zone_id) {
            if let Some(l) = label {
                zone.insert(&mut txn, "label", l);
            } else {
                zone.remove(&mut txn, "label");
            }
        }
    }
}

impl MissionDocCore {
    /// Set zone faction using the supplied domain data.
    pub fn set_zone_faction(&self, zone_id: &str, faction: Option<&str>) {
        let mut txn = self.begin();
        if let Some(Out::YMap(zone)) = self.zones.get(&txn, zone_id) {
            if let Some(f) = faction {
                zone.insert(&mut txn, "faction", f);
            } else {
                zone.remove(&mut txn, "faction");
            }
        }
    }
}

impl MissionDocCore {
    /// `None`, malformed JSON, a non-object, and `{}` all REMOVE the key. That last one matters: `rules` is optional as a whole and every key defaults, so "the author cleared every rule" and "the author never opened the panel" are the same document — writing `"rules": {}` would invent a third state the schema and the mod cannot tell apart.
    pub fn set_zone_rules(&self, zone_id: &str, rules_json: Option<&str>) {
        let mut txn = self.begin();
        let Some(Out::YMap(zone)) = self.zones.get(&txn, zone_id) else {
            return;
        };
        let parsed = rules_json.map(json_str_to_any);
        match parsed {
            Some(Any::Map(m)) if !m.is_empty() => {
                zone.insert(&mut txn, "rules", Any::Map(m));
            }
            _ => {
                zone.remove(&mut txn, "rules");
            }
        }
    }
}

impl MissionDocCore {
    /// Remove zone using the supplied domain data.
    pub fn remove_zone(&self, zone_id: &str) {
        let mut txn = self.begin();
        self.zones.remove(&mut txn, zone_id);
    }
}

impl MissionDocCore {
    /// Zones json using the supplied domain data.
    #[must_use]
    pub fn zones_json(&self) -> String {
        let txn = self.doc.transact();
        let mut buf = String::new();
        self.zones.to_json(&txn).to_json(&mut buf);
        buf
    }
}

impl MissionDocCore {
    /// Zone count using the supplied domain data.
    #[must_use]
    pub fn zone_count(&self) -> usize {
        self.zones.len(&self.doc.transact()) as usize
    }
}
