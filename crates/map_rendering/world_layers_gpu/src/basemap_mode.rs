//! **Role:** `BasemapMode`, how a textured lane's texture is laid out, as the render statistics
//! report spells it.
//! **Position:** recorded on every textured lane's record (`textured_lane::TexLane`) by the layer
//! that commits it, and read by the map renderer's statistics report as `basemap_mode`.
//! **Signals & state:** none; a plain value.
//! **Invariants:** the report spellings and the mode codes are fixed: a texture layer's begin call
//! names the mode by code, and every unknown code is the unified basemap.

/// How a textured lane's texture is laid out, as the statistics report spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasemapMode {
    /// The unified basemap (mode code 0, and every unknown code).
    Unified,

    /// A satellite pyramid level (mode code 1).
    Pyramid,

    /// A single bitmap (mode code 2); the forest density and viewshed rasters report it.
    Single,

    /// The hillshade (mode code 3).
    Hillshade,
}

impl BasemapMode {
    /// The mode as the statistics report's `basemap_mode` spells it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unified => "unified",
            Self::Pyramid => "pyramid",
            Self::Single => "single-bitmap",
            Self::Hillshade => "hillshade",
        }
    }

    /// The mode a texture layer's begin call names by code.
    #[must_use]
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Pyramid,
            2 => Self::Single,
            3 => Self::Hillshade,
            _ => Self::Unified,
        }
    }
}

#[cfg(test)]
#[path = "tests/basemap_mode_tests.rs"]
mod tests;
