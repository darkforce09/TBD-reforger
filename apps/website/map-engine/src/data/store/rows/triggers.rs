//! Role: triggers.
//! Position: `doc/store` in the map engine's headless mission data domain.
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
    /// Add circle trigger using the supplied domain data.
    pub fn add_circle_trigger(&self, id: &str, activation: &str, x: f64, z: f64, r: f64) {
        let mut txn = self.begin();
        let t = self
            .triggers
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        t.insert(&mut txn, "activation", activation);
        t.insert(&mut txn, "shape", circle_shape_any(x, z, r));
    }
}

impl MissionDocCore {
    /// Add polygon trigger using the supplied domain data.
    pub fn add_polygon_trigger(&self, id: &str, activation: &str, points_flat: &[f64]) {
        let mut txn = self.begin();
        let t = self
            .triggers
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        t.insert(&mut txn, "activation", activation);
        t.insert(&mut txn, "shape", polygon_shape_any(points_flat));
    }
}

impl MissionDocCore {
    /// Set trigger circle using the supplied domain data.
    pub fn set_trigger_circle(&self, trigger_id: &str, x: f64, z: f64, r: f64) {
        let mut txn = self.begin();
        if let Some(Out::YMap(t)) = self.triggers.get(&txn, trigger_id) {
            t.insert(&mut txn, "shape", circle_shape_any(x, z, r));
        }
    }
}

impl MissionDocCore {
    /// Set trigger polygon using the supplied domain data.
    pub fn set_trigger_polygon(&self, trigger_id: &str, points_flat: &[f64]) {
        let mut txn = self.begin();
        if let Some(Out::YMap(t)) = self.triggers.get(&txn, trigger_id) {
            t.insert(&mut txn, "shape", polygon_shape_any(points_flat));
        }
    }
}

impl MissionDocCore {
    /// Set trigger name using the supplied domain data.
    pub fn set_trigger_name(&self, trigger_id: &str, name: Option<&str>) {
        let mut txn = self.begin();
        if let Some(Out::YMap(t)) = self.triggers.get(&txn, trigger_id) {
            if let Some(n) = name {
                t.insert(&mut txn, "name", n);
            } else {
                t.remove(&mut txn, "name");
            }
        }
    }
}

impl MissionDocCore {
    /// Set trigger owner using the supplied domain data.
    pub fn set_trigger_owner(&self, trigger_id: &str, owner_id: Option<&str>) {
        let mut txn = self.begin();
        if let Some(Out::YMap(t)) = self.triggers.get(&txn, trigger_id) {
            if let Some(o) = owner_id {
                t.insert(&mut txn, "ownerId", o);
            } else {
                t.remove(&mut txn, "ownerId");
            }
        }
    }
}

impl MissionDocCore {
    /// Set trigger activation using the supplied domain data.
    pub fn set_trigger_activation(&self, trigger_id: &str, activation: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(t)) = self.triggers.get(&txn, trigger_id) {
            t.insert(&mut txn, "activation", activation);
        }
    }
}

impl MissionDocCore {
    /// Set trigger rules using the supplied domain data.
    pub fn set_trigger_rules(&self, trigger_id: &str, rules_json: Option<&str>) {
        let mut txn = self.begin();
        let Some(Out::YMap(t)) = self.triggers.get(&txn, trigger_id) else {
            return;
        };
        match rules_json.map(json_str_to_any) {
            Some(Any::Map(m)) if !m.is_empty() => {
                t.insert(&mut txn, "rules", Any::Map(m));
            }
            _ => {
                t.remove(&mut txn, "rules");
            }
        }
    }
}

impl MissionDocCore {
    /// Remove trigger using the supplied domain data.
    pub fn remove_trigger(&self, trigger_id: &str) {
        let mut txn = self.begin();
        self.triggers.remove(&mut txn, trigger_id);
    }
}

impl MissionDocCore {
    /// Triggers json using the supplied domain data.
    #[must_use]
    pub fn triggers_json(&self) -> String {
        let txn = self.doc.transact();
        let mut buf = String::new();
        self.triggers.to_json(&txn).to_json(&mut buf);
        buf
    }
}

impl MissionDocCore {
    /// Trigger count using the supplied domain data.
    #[must_use]
    pub fn trigger_count(&self) -> usize {
        self.triggers.len(&self.doc.transact()) as usize
    }
}
