//! The corpus facts no ticket file states, read from [`repository::CORPUS_PINS`].
//!
//! Two rules need a list the ticket files cannot supply: the ids that must never be minted, and
//! which ticket implements an editor gap row when the ticket itself does not claim it. Both are
//! statements about this checkout's corpus rather than about the ticket format, so they live in
//! the corpus directory beside the schema and the scope vocabulary, and the code that enforces
//! them reads them from there.
//!
//! [`load`] is fail-closed: a missing, unreadable or malformed file is an error, never an empty
//! table, so losing the file reds `ticket check` instead of relaxing the rules it feeds.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::repository::CORPUS_PINS;

/// Everything [`repository::CORPUS_PINS`](crate::repository::CORPUS_PINS) declares.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusPins {
    /// The programme ticket whose dotted children are the game-mod slices.
    pub game_mod_programme_ticket: String,
    /// The programme ticket that owns the map-and-terrain specification set.
    pub map_terrain_programme_ticket: String,
    /// Ids no ticket row may carry. `ticket check` reds when the registry holds one.
    pub never_minted: Vec<String>,
    /// Editor gap row id → the ticket that implements it, consulted only when no ticket claims
    /// the row through its own `implements` list.
    pub gap_implementations: BTreeMap<String, String>,
}

impl CorpusPins {
    /// Whether `id` is one of the ids that must never be minted.
    #[must_use]
    pub fn is_never_minted(&self, id: &str) -> bool {
        self.never_minted.iter().any(|pinned| pinned == id)
    }

    /// The ticket pinned to an editor gap row id, if one is.
    #[must_use]
    pub fn gap_implementation(&self, gap_id: &str) -> Option<&str> {
        self.gap_implementations.get(gap_id).map(String::as_str)
    }
}

/// Read the pins for the checkout at `root`.
///
/// # Errors
/// When the file is absent, unreadable, or does not parse into [`CorpusPins`].
pub fn load(root: &Path) -> Result<CorpusPins> {
    let path = root.join(CORPUS_PINS);
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("missing corpus pins: {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("malformed corpus pins: {}", path.display()))
}

#[cfg(test)]
#[path = "tests/corpus_pins_tests.rs"]
mod tests;
