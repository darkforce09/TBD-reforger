//! Role: entities.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::MapPrelim;
use super::MissionDocCore;
use super::Out;
use super::position_any;
use yrs::Map;

impl MissionDocCore {
    /// Add entity using the supplied domain data.
    #[allow(clippy::too_many_arguments)]
    pub fn add_entity(
        &self,
        id: &str,
        alias: &str,
        resource_name: &str,
        x: f64,
        y: f64,
        z: f64,
        rotation: f64,
    ) {
        let mut txn = self.begin();
        let e = self
            .entities
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        e.insert(&mut txn, "alias", alias);
        e.insert(&mut txn, "resourceName", resource_name);
        e.insert(&mut txn, "position", position_any(x, y, z, rotation));
    }
}

impl MissionDocCore {
    /// Set entity faction using the supplied domain data.
    pub fn set_entity_faction(&self, entity_id: &str, faction: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(e)) = self.entities.get(&txn, entity_id) {
            e.insert(&mut txn, "faction", faction);
        }
    }
}

impl MissionDocCore {
    /// Remove entity using the supplied domain data.
    pub fn remove_entity(&self, entity_id: &str) {
        let mut txn = self.begin();
        self.entities.remove(&mut txn, entity_id);
    }
}
