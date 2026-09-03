//! T-090.11 — the compound building: the shell's collision mesh plus every child entity a
//! prefab places in it (door leaves and frames, window frames, glass panes, furniture,
//! scene trees), each a BLAS under a rigid transform in the building's local frame.
//!
//! This file carries the **wire types** of the `<slug>.instances.json` sidecar (emitted by
//! `cargo xtask map bvh-batch`, T-090.11.2; the JSON contract is camelCase like the
//! blueprint, schema `packages/tbd-schema/schema/building-instances.schema.json`). The
//! raycast walk, door state and wash over instances land in T-090.11.4 on top of them.
//!
//! Frame conventions: Enfusion local space (x, y up, z), metres; a `LocalTransform` maps the
//! instance's BLAS space into the building's space (`p_building = local(p_blas)`). Rotation
//! travels as a unit quaternion `[x, y, z, w]` — lossless for socket transforms read
//! straight out of the XOB node table — never as Euler angles.

use serde::{Deserialize, Serialize};

use crate::geometry::rigid::Rigid;

/// Contract version of `<slug>.instances.json`.
pub const INSTANCES_SCHEMA_VERSION: &str = "1.0.0";

/// What an instance is, for LOS attribution and lane routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstanceKind {
    /// The building's own collision mesh (never emitted as an instance record; the shell is
    /// `shellBvh`).
    Shell,
    /// A rotating / sliding door leaf — carries a [`DoorRecord`].
    DoorLeaf,
    /// The static frame a leaf hangs in.
    DoorFrame,
    /// A window frame (mullions, sill) without its panes.
    WindowFrame,
    /// A glass pane (its own destructible entity).
    Glass,
    /// Furniture and interior props.
    Furniture,
    /// A whole tree: one BLAS whose kinds table carries the trunk (opaque) and the foliage
    /// colliders (foliage) per triangle.
    Tree,
    /// A canopy proxy built from the visual LOD when the tree's collision mesh has no
    /// foliage collider (all-Foliage BLAS).
    TreeCanopy,
    /// Anything else placed in the building (radiators, lights, decorations).
    Prop,
}

/// Cover an instance offers a prone / crouched soldier, from the prefab category heuristic
/// the Workbench extractor used (`TBD_BuildingArchitectExtractor.c`): cupboards and
/// wardrobes are full cover, tables and beds low cover, chairs none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverTier {
    Full,
    Low,
    None,
}

/// Where the instance's transform came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlacementSource {
    /// A socket bone decoded from the parent model's XOB node table (T-090.11.2).
    XobSocket,
    /// The Workbench recon dump of the live entity hierarchy (T-090.11.3 fallback).
    Recon,
    /// Explicit `coords` / `angles` / `scale` in the prefab text.
    PrefabCoords,
    /// A hand-placed scene entry (`<slug>.scene.json`).
    Scene,
}

fn one() -> f64 {
    1.0
}

/// A rigid placement: position, unit quaternion, uniform scale.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalTransform {
    pub pos: [f64; 3],
    /// `[x, y, z, w]`.
    pub quat: [f64; 4],
    #[serde(default = "one")]
    pub scale: f64,
}

impl LocalTransform {
    #[must_use]
    pub fn identity() -> Self {
        Self {
            pos: [0.0; 3],
            quat: [0.0, 0.0, 0.0, 1.0],
            scale: 1.0,
        }
    }

    #[must_use]
    pub fn from_rigid(r: &Rigid) -> Self {
        Self {
            pos: r.t,
            quat: r.to_quat(),
            scale: r.scale,
        }
    }

    #[must_use]
    pub fn rigid(&self) -> Rigid {
        let mut r = Rigid::from_quat_pos(self.quat, self.pos);
        r.scale = if self.scale.is_finite() && self.scale > 0.0 {
            self.scale
        } else {
            1.0
        };
        r
    }
}

/// Door mechanics from the prefab's `DoorComponent` / `SlidingDoorComponent`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorRecord {
    /// Signed sweep about the leaf's local Y; the sign is the swing side.
    pub angle_range_deg: f64,
    pub closed_angle_deg: f64,
    pub initial_angle_deg: f64,
    /// `AngleRange` was set somewhere in the prefab chain (else the 90° default).
    #[serde(default)]
    pub angle_range_explicit: bool,
    /// Sliding door: travel along local X when fully open (m). `None` for a rotating door.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened_distance: Option<f64>,
}

/// One placed BLAS.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceRecord {
    /// Stable within the building: the prefab child's `ID` chain joined by `/`.
    pub id: String,
    pub kind: InstanceKind,
    /// The child's prefab path (GUID stripped).
    pub prefab: String,
    /// Sidecar path relative to the instances file's directory (`blas/<asset>.bvh`).
    pub blas: String,
    /// The XOB the BLAS was built from (diagnostics; never fetched by the viewer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xob: Option<String>,
    pub local: LocalTransform,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub door: Option<DoorRecord>,
    pub cover: CoverTier,
    pub source: PlacementSource,
    /// The instance this one is attached under (`None` = the shell).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

/// The `<slug>.instances.json` document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstancesFile {
    pub schema_version: String,
    pub prefab_id: String,
    /// The building prefab (`Prefabs/…/X.et`).
    pub resource_name: String,
    /// Shell sidecar path relative to this file's directory.
    pub shell_bvh: String,
    pub instances: Vec<InstanceRecord>,
    /// Emitter notes: fallbacks taken, unresolved children, missing meshes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl InstancesFile {
    /// Every distinct BLAS path referenced, in first-use order.
    #[must_use]
    pub fn blas_paths(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for i in &self.instances {
            if !out.contains(&i.blas.as_str()) {
                out.push(&i.blas);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instances_json_round_trips_camel_case() {
        let door = InstanceRecord {
            id: "door_int_left_01/leaf".into(),
            kind: InstanceKind::DoorLeaf,
            prefab: "Prefabs/Doors/Leaf.et".into(),
            blas: "blas/Leaf.bvh".into(),
            xob: Some("Assets/Doors/Leaf.xob".into()),
            local: LocalTransform::from_rigid(&Rigid::from_enfusion(
                [1.0, 0.0, -2.0],
                [0.0, 90.0, 0.0],
                1.0,
            )),
            door: Some(DoorRecord {
                angle_range_deg: -120.0,
                closed_angle_deg: 0.0,
                initial_angle_deg: 0.0,
                angle_range_explicit: true,
                opened_distance: None,
            }),
            cover: CoverTier::Full,
            source: PlacementSource::XobSocket,
            parent: Some("door_int_left_01".into()),
        };
        let file = InstancesFile {
            schema_version: INSTANCES_SCHEMA_VERSION.into(),
            prefab_id: "House".into(),
            resource_name: "Prefabs/Houses/House.et".into(),
            shell_bvh: "House.bvh".into(),
            instances: vec![
                door.clone(),
                InstanceRecord {
                    id: "furn/T1".into(),
                    kind: InstanceKind::Furniture,
                    prefab: "Prefabs/Props/Table.et".into(),
                    blas: "blas/Table.bvh".into(),
                    xob: None,
                    local: LocalTransform::identity(),
                    door: None,
                    cover: CoverTier::Low,
                    source: PlacementSource::PrefabCoords,
                    parent: None,
                },
            ],
            notes: vec![],
        };
        let json = serde_json::to_string_pretty(&file).unwrap();
        assert!(json.contains("\"schemaVersion\""), "{json}");
        assert!(json.contains("\"kind\": \"doorLeaf\""));
        assert!(json.contains("\"cover\": \"full\""));
        assert!(json.contains("\"source\": \"xobSocket\""));
        assert!(json.contains("\"angleRangeDeg\": -120.0"));
        assert!(!json.contains("openedDistance"), "None is skipped");
        assert!(!json.contains("\"notes\""), "empty notes are skipped");
        let back: InstancesFile = serde_json::from_str(&json).unwrap();
        assert_eq!(back, file);
        assert_eq!(back.blas_paths(), ["blas/Leaf.bvh", "blas/Table.bvh"]);
        // The quaternion round-trips the rotation: local +x of a 90° yaw points to -z.
        let r = back.instances[0].local.rigid();
        let d = r.dir([1.0, 0.0, 0.0]);
        assert!(d[0].abs() < 1e-9 && (d[2] + 1.0).abs() < 1e-9);
        // `scale` defaults to 1 when absent.
        let minimal: LocalTransform =
            serde_json::from_str(r#"{"pos":[0,0,0],"quat":[0,0,0,1]}"#).unwrap();
        assert_eq!(minimal.scale, 1.0);
    }
}
