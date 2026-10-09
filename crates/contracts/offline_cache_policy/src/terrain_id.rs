//! The identifier of a terrain as its served files spell it.
//!
//! **Role:** the typed `terrainId` of a served terrain manifest and map tile index, which is also
//! the folder its files are served under (`/map-assets/<terrain>/`).
//! **Position:** carried by [`crate::offline_pack::MapTileIndex`] and
//! [`crate::offline_pack::OfflinePack`] and taken by the served-path helpers of
//! [`crate::offline_pack`]; the page names the terrain it packs with one.
//! **Signals & state:** none; plain data.
//! **Invariants:** serialises as the bare string (`serde(transparent)`), so every JSON shape
//! that carries it is byte-identical to a plain `String` field.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A terrain's identifier, such as `everon`: the `terrainId` of its manifest and tile index.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TerrainId(String);

impl TerrainId {
    /// The identifier spelled `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The identifier as its served files spell it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TerrainId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl PartialEq<str> for TerrainId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for TerrainId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}
