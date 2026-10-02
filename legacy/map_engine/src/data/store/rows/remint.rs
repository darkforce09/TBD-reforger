//! Role: remint.
//! Position: `doc/store` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::HashMap;
use super::HashSet;

/// Domain representation of remint map.
pub(super) struct RemintMap {
    /// Map.
    pub(super) map: HashMap<String, String>,

    /// Merged.
    pub(super) merged: HashSet<String>,

    /// Taken.
    pub(super) taken: HashSet<String>,

    /// Seq.
    pub(super) seq: u64,
}

impl RemintMap {
    /// New using the supplied domain data.
    #[cfg(test)]
    pub(super) fn new() -> Self {
        Self::with_reserved(HashSet::new())
    }
}

impl RemintMap {
    /// With reserved using the supplied domain data.
    pub(super) fn with_reserved(reserved: HashSet<String>) -> Self {
        Self {
            map: HashMap::new(),
            merged: HashSet::new(),
            taken: reserved,
            seq: 0,
        }
    }
}

impl RemintMap {
    /// Map to existing using the supplied domain data.
    pub(super) fn map_to_existing(&mut self, old: &str, resident: &str) {
        self.map.insert(old.to_string(), resident.to_string());
        self.merged.insert(old.to_string());
    }
}

impl RemintMap {
    /// Ensure fresh using the supplied domain data.
    pub(super) fn ensure_fresh(&mut self, old: &str) {
        if self.map.contains_key(old) {
            return;
        }
        let fresh = loop {
            self.seq += 1;
            let candidate = format!("mrg-{}-{}", self.seq, old);
            if !self.taken.contains(&candidate) {
                break candidate;
            }
        };
        self.taken.insert(fresh.clone());
        self.map.insert(old.to_string(), fresh);
    }
}

impl RemintMap {
    /// Get using the supplied domain data.
    pub(super) fn get(&self, old: &str) -> Option<String> {
        self.map.get(old).cloned()
    }
}

/// Domain representation of named side index.
pub(super) type NamedSideIndex = HashMap<(String, String), String>;
