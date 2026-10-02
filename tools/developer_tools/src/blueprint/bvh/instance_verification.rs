//! `cargo xtask map instances-verify --instances <slug>.instances.json --recon
//! <slug>_children.json` — the socket-transform check: every architectural instance
//! the offline pipeline placed from an XOB socket (`source: xobSocket`) is matched to the
//! Workbench recon dump of the live entity hierarchy and must agree within [`POS_TOL_M`] /
//! [`YAW_TOL_DEG`].
//!
//! Matching: the recon (as shipped in the addon at the time of the first dump) records class,
//! components, bounds size, world yaw and `relPos` (world-axis offset from the building origin)
//! but an empty `resource` for prefab-nested children, so children are bucketed into a
//! [`Group`] from class + components (window sets are `Building`, door sets are depth-1
//! `GenericEntity`, leaves carry `DoorComponent`, panes carry
//! `SCR_DestructionMultiPhaseComponent`, entries are `StaticModelEntity`) and paired with the
//! instances of the same group by greedy nearest position in the building's local frame.
//! Instances that descend from a furniture instance are skipped: the world places the furniture
//! composition as a sibling entity, not under the building, so the recon cannot see them.
//!
//! Frames: the building's `rootAngles` turn `relPos` / world yaw into the local frame the
//! instances use. The rotation handedness is not assumed: both yaw signs are tried and the one
//! with the smaller total position error is reported (the handedness pin).

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use website_map_engine::world::architecture::compound::assembly::PlacementSource;
use website_map_engine::world::architecture::compound::instances::InstanceKind;
use website_map_engine::world::architecture::compound::instances::InstanceRecord;
use website_map_engine::world::architecture::compound::instances::InstancesFile;
use website_map_engine::world::architecture::compound::transform::Rigid;

pub const POS_TOL_M: f64 = 0.02;
pub const YAW_TOL_DEG: f64 = 1.0;
/// Candidate pairs farther apart than this are never matched (keeps a missing entity from
/// stealing a neighbour's partner).
const MATCH_CAP_M: f64 = 1.5;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconFile {
    pub slug: String,
    pub root_angles: [f64; 3],
    /// The building's absolute origin (the world-row pin locates its chunk row).
    #[serde(default)]
    pub root_world_pos: Option<[f64; 3]>,
    pub children: Vec<ReconChild>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReconChild {
    pub depth: u32,
    #[serde(default)]
    pub name: String,
    pub class: String,
    #[serde(default)]
    pub resource: String,
    pub rel_pos: [f64; 3],
    pub yaw_deg: f64,
    #[serde(default)]
    pub size: [f64; 3],
    #[serde(default)]
    pub components: Vec<String>,
    // ── Socket enrichment (present once the recon plugin is compiled with ExtrasJson) ──
    /// The `Hierarchy` component's `PivotID` — the socket the child hangs on.
    #[serde(default)]
    pub pivot_id: String,
    /// Origin in the PARENT entity's frame (`parent.CoordToLocal`).
    #[serde(default)]
    pub local_pos: Option<[f64; 3]>,
    /// World `[pitch, yaw, roll]`.
    #[serde(default)]
    pub angles_deg: Option<[f64; 3]>,
    /// Absolute world origin of the child (the world-row pin).
    #[serde(default)]
    pub world_pos: Option<[f64; 3]>,
    /// `DoorComponent` params on a leaf.
    #[serde(default)]
    pub door: Option<ReconDoor>,
}

/// The recon's `door` object (a `DoorComponent`'s hinge params).
#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReconDoor {
    #[serde(default)]
    pub angle_range: f64,
    #[serde(default)]
    pub closed_angle: f64,
    #[serde(default)]
    pub initial_angle: f64,
}

/// Architectural bucket shared by recon children and instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Group {
    WindowFrame,
    DoorFrame,
    DoorLeaf,
    Glass,
    Prop,
    Other,
}

impl ReconChild {
    pub fn group(&self) -> Group {
        let has = |c: &str| self.components.iter().any(|k| k == c);
        if has("DoorComponent") {
            Group::DoorLeaf
        } else if self.class == "Building" {
            Group::WindowFrame
        } else if self.class == "StaticModelEntity" {
            Group::Prop
        } else if self.depth == 1 {
            Group::DoorFrame
        } else if has("SCR_DestructionMultiPhaseComponent") || self.size[2].abs() < 1e-3 {
            Group::Glass
        } else {
            Group::Other
        }
    }
}

/// One instance ↔ recon child pairing.
#[derive(Debug, Clone)]
pub struct Match {
    pub instance: String,
    pub child_index: usize,
    pub group: Group,
    pub pos_err_m: f64,
    pub yaw_err_deg: f64,
}

#[derive(Debug, Default)]
pub struct Report {
    /// Yaw sign used for the world → local rotation (`+1` = `Rigid::from_enfusion` as is).
    pub yaw_sign: f64,
    pub matches: Vec<Match>,
    pub unmatched: Vec<String>,
    /// Recon children no instance claimed (index, group).
    pub extra: Vec<(usize, Group)>,
    /// Instances skipped because they descend from a furniture instance.
    pub skipped_furniture: usize,
    /// Enriched-recon checks: how many were possible, and the failures (`instance: detail`).
    pub door_checks: usize,
    pub door_mismatches: Vec<String>,
    pub pivot_checks: usize,
    pub pivot_mismatches: Vec<String>,
    pub local_checks: usize,
    pub local_mismatches: Vec<String>,
}

impl Report {
    pub fn failures(&self) -> Vec<&Match> {
        self.matches
            .iter()
            .filter(|m| m.pos_err_m > POS_TOL_M || m.yaw_err_deg > YAW_TOL_DEG)
            .collect()
    }
    pub fn worst_pos_m(&self) -> f64 {
        self.matches.iter().map(|m| m.pos_err_m).fold(0.0, f64::max)
    }
    pub fn worst_yaw_deg(&self) -> f64 {
        self.matches
            .iter()
            .map(|m| m.yaw_err_deg)
            .fold(0.0, f64::max)
    }
    pub fn ok(&self) -> bool {
        self.failures().is_empty()
            && self.unmatched.is_empty()
            && self.door_mismatches.is_empty()
            && self.pivot_mismatches.is_empty()
            && self.local_mismatches.is_empty()
    }
}

#[cfg(test)]
#[path = "../tests/verify/tests.rs"]
mod tests;

#[path = "instance_verification/instance_group.rs"]
mod instance_group;
pub use instance_group::run_instances_verify;

#[cfg(test)]
pub(crate) use instance_group::load;

#[cfg(test)]
pub(crate) use instance_group::verify;

#[cfg(test)]
pub(crate) use instance_group::wrap_deg;
