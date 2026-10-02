//! Role: preferences.
//! Position: `streaming/bridge` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use serde::{Deserialize, Serialize};

/// World layer prefs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldLayerPrefs {
    /// Roads.
    pub roads: bool,

    /// Buildings.
    pub buildings: bool,

    /// Forest.
    pub forest: bool,

    /// Trees.
    pub trees: bool,

    /// Props.
    pub props: bool,

    /// Contours.
    pub contours: bool,

    /// Sea.
    pub sea: bool,

    /// Fences.
    pub fences: bool,

    /// Airfield.
    pub airfield: bool,

    /// Heights.
    pub heights: bool,

    /// Town labels.
    #[serde(rename = "townLabels")]
    pub town_labels: bool,

    /// Road names.
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
