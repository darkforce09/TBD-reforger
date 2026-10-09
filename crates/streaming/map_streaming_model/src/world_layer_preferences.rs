//! The twelve world-layer switches a user toggles on the map.
//!
//! **Role:** [`WorldLayerPrefs`]: which world layers (roads, buildings, forest, trees, props,
//! contours, sea, fences, airfield, spot heights, town labels, road names) the map shows, and
//! their defaults.
//! **Position:** stored by the Mission Creator's preference store, read by the map host's
//! [`crate::host_preferences::HostPreferences::world_layers`] reader at every refresh, applied by
//! the world and label loaders.
//! **Signals & state:** none; a plain value.
//! **Invariants:** the stored JSON keys are the field names except `townLabels` and `roadNames`;
//! every layer is on by default except props.

use serde::{Deserialize, Serialize};

/// The world-layer switches; `true` shows the layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldLayerPrefs {
    /// The road network.
    pub roads: bool,

    /// Building footprints.
    pub buildings: bool,

    /// The forest mass fill and outline.
    pub forest: bool,

    /// Tree glyphs.
    pub trees: bool,

    /// Prop glyphs.
    pub props: bool,

    /// Contour lines.
    pub contours: bool,

    /// The sea band.
    pub sea: bool,

    /// Fences, piers and bridge rails.
    pub fences: bool,

    /// The airfield apron.
    pub airfield: bool,

    /// Spot height labels.
    pub heights: bool,

    /// Town name labels, stored as `townLabels`.
    #[serde(rename = "townLabels")]
    pub town_labels: bool,

    /// Road name labels, stored as `roadNames`.
    #[serde(rename = "roadNames")]
    pub road_names: bool,
}

impl Default for WorldLayerPrefs {
    fn default() -> Self {
        Self {
            roads: true,
            buildings: true,
            forest: true,
            trees: true,
            props: false,
            contours: true,
            sea: true,
            fences: true,
            airfield: true,
            heights: true,
            town_labels: true,
            road_names: true,
        }
    }
}
