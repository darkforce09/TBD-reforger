//! T-090.11 — the compound building: the shell's collision mesh plus every child entity a
//! prefab places in it (door leaves and frames, window frames, glass panes, furniture,
//! scene trees), each a BLAS under a rigid transform in the building's local frame.
//!
//! This file carries the **wire types** of the `<slug>.instances.json` sidecar (emitted by
//! `cargo xtask map bvh-batch`, T-090.11.2; the JSON contract is camelCase like the
//! blueprint, schema `packages/tbd-schema/schema/building-instances.schema.json`) and, below
//! them, the assembled model: [`CompoundBuilding`] (shell + [`Instance`]s under rigid
//! placements, door leaves with a [`DoorState`]) and its [`FlatMesh`] bake. The raycast walk,
//! LOS attribution and wash over instances live in `building_compound_los.rs` (T-090.11.4).
//!
//! Frame conventions: Enfusion local space (x, y up, z), metres; a `LocalTransform` maps the
//! instance's BLAS space into the building's space (`p_building = local(p_blas)`). Rotation
//! travels as a unit quaternion `[x, y, z, w]` — lossless for socket transforms read
//! straight out of the XOB node table — never as Euler angles.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::bvh::{Bvh, BvhSidecar, SurfaceKind};
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
    /// Sidecar path relative to the prefabs root — the parent of the `buildings/` directory the
    /// instances file lives in (`blas/<asset>.bvh`); loaders try the file's own directory first.
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

// ───────────────────────────── behaviour (T-090.11.4) ─────────────────────────────

/// A door leaf's state: closed, or open by a fraction of its full sweep.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DoorState {
    Closed,
    /// Fraction of the full sweep (`angle_range_deg` for a hinge, `opened_distance` for a
    /// slider), clamped to `0..=1` wherever it is applied.
    Open {
        fraction: f64,
    },
}

impl DoorState {
    /// Fully open.
    pub const OPEN: DoorState = DoorState::Open { fraction: 1.0 };

    /// The applied fraction (`0` closed … `1` fully open).
    #[must_use]
    pub fn fraction(self) -> f64 {
        match self {
            DoorState::Closed => 0.0,
            DoorState::Open { fraction } => {
                if fraction.is_finite() {
                    fraction.clamp(0.0, 1.0)
                } else {
                    0.0
                }
            }
        }
    }

    #[must_use]
    pub fn is_open(self) -> bool {
        self.fraction() > 0.0
    }

    /// Closed ↔ fully open (the viewer's click).
    #[must_use]
    pub fn toggled(self) -> DoorState {
        if self.is_open() {
            DoorState::Closed
        } else {
            DoorState::OPEN
        }
    }
}

/// One placed BLAS with its live state.
#[derive(Clone, Debug)]
pub struct Instance {
    pub record: InstanceRecord,
    pub blas: Arc<BvhSidecar>,
    /// Building ← BLAS placement at rest (the leaf's CLOSED pose for a door).
    pub local: Rigid,
    /// The BLAS's own AABB (root node bounds), in BLAS space.
    pub bounds: ([f64; 3], [f64; 3]),
    pub state: DoorState,
}

impl Instance {
    /// Is this a door leaf with hinge / slide parameters?
    #[must_use]
    pub fn is_door(&self) -> bool {
        self.record.kind == InstanceKind::DoorLeaf && self.record.door.is_some()
    }

    /// The leaf's motion for its current state, in leaf space: a hinge turns about the leaf
    /// origin's local Y (`DoorComponent` — the collider hangs off the hinge along local +X,
    /// so `rot_y(θ)` swings the free edge; the yaw sense is the pinned Enfusion yaw), a slider
    /// translates along local X by `fraction · opened_distance`. Identity for everything else.
    #[must_use]
    pub fn hinge(&self) -> Rigid {
        let Some(door) = self
            .record
            .door
            .filter(|_| self.record.kind == InstanceKind::DoorLeaf)
        else {
            return Rigid::identity();
        };
        let f = self.state.fraction();
        match door.opened_distance {
            Some(dist) => Rigid::translation([f * dist, 0.0, 0.0]),
            None => Rigid::rot_y(door.closed_angle_deg + f * door.angle_range_deg),
        }
    }

    /// Building ← BLAS placement for the current state (`local ∘ hinge`).
    #[must_use]
    pub fn placement(&self) -> Rigid {
        if self.is_door() {
            self.local.compose(&self.hinge())
        } else {
            self.local
        }
    }

    /// World (building-space) AABB of the BLAS under the current placement.
    #[must_use]
    pub fn world_aabb(&self) -> ([f64; 3], [f64; 3]) {
        self.placement().aabb_of(self.bounds.0, self.bounds.1)
    }
}

/// Why a compound could not be assembled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompoundError {
    /// Records reference BLAS paths the loader did not provide (deduplicated, in first-use order).
    MissingBlas(Vec<String>),
}

impl core::fmt::Display for CompoundError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CompoundError::MissingBlas(paths) => write!(f, "missing BLAS: {}", paths.join(", ")),
        }
    }
}

impl std::error::Error for CompoundError {}

/// The shell plus every placed instance — the two-level acceleration structure the LOS walk,
/// the wash and the drawing run over (`building_compound_los.rs`).
#[derive(Clone, Debug)]
pub struct CompoundBuilding {
    pub shell: Arc<BvhSidecar>,
    pub instances: Vec<Instance>,
}

impl CompoundBuilding {
    /// A shell with no instances yet.
    #[must_use]
    pub fn new(shell: Arc<BvhSidecar>) -> Self {
        Self {
            shell,
            instances: Vec::new(),
        }
    }

    /// Shell + records, every record's `blas` path resolved through `blas_by_path` (keys are
    /// the record's `blas` strings verbatim). Atomic: a missing BLAS assembles nothing.
    pub fn assemble(
        shell: Arc<BvhSidecar>,
        records: &[InstanceRecord],
        blas_by_path: &HashMap<String, Arc<BvhSidecar>>,
    ) -> Result<Self, CompoundError> {
        let mut c = Self::new(shell);
        c.append(records, blas_by_path)?;
        Ok(c)
    }

    /// Add more records (a `<slug>.scene.json`'s trees, for instance). Atomic like [`Self::assemble`].
    pub fn append(
        &mut self,
        records: &[InstanceRecord],
        blas_by_path: &HashMap<String, Arc<BvhSidecar>>,
    ) -> Result<(), CompoundError> {
        let mut missing: Vec<String> = Vec::new();
        for r in records {
            if !blas_by_path.contains_key(&r.blas) && !missing.contains(&r.blas) {
                missing.push(r.blas.clone());
            }
        }
        if !missing.is_empty() {
            return Err(CompoundError::MissingBlas(missing));
        }
        let mut staged = Vec::with_capacity(records.len());
        for r in records {
            let blas = Arc::clone(&blas_by_path[&r.blas]);
            // `Bvh::build` refuses an empty mesh, so every parsed BLAS has root bounds.
            let bounds = blas.bvh.root_bounds().unwrap_or(([0.0; 3], [0.0; 3]));
            let state = match r.door {
                Some(d) if r.kind == InstanceKind::DoorLeaf => {
                    let sweep = if d.opened_distance.is_some() {
                        1.0
                    } else {
                        d.angle_range_deg
                    };
                    let f = if sweep.abs() > 1e-9 {
                        (d.initial_angle_deg - d.closed_angle_deg) / sweep
                    } else {
                        0.0
                    };
                    if f > 1e-9 {
                        DoorState::Open { fraction: f }
                    } else {
                        DoorState::Closed
                    }
                }
                _ => DoorState::Closed,
            };
            staged.push(Instance {
                record: r.clone(),
                local: r.local.rigid(),
                blas,
                bounds,
                state,
            });
        }
        self.instances.extend(staged);
        Ok(())
    }

    /// Index of the instance with this id.
    #[must_use]
    pub fn instance_index(&self, id: &str) -> Option<usize> {
        self.instances.iter().position(|i| i.record.id == id)
    }

    /// Every door leaf, in instance order.
    pub fn doors(&self) -> impl Iterator<Item = &Instance> {
        self.instances.iter().filter(|i| i.is_door())
    }

    /// Set a leaf's state (the fraction is clamped on use). `false` when `id` is not a door.
    pub fn set_door(&mut self, id: &str, state: DoorState) -> bool {
        match self.instance_index(id) {
            Some(i) if self.instances[i].is_door() => {
                self.instances[i].state = state;
                true
            }
            _ => false,
        }
    }

    /// A leaf's state (`None` when `id` is not a door).
    #[must_use]
    pub fn door_state(&self, id: &str) -> Option<DoorState> {
        self.instance_index(id)
            .filter(|&i| self.instances[i].is_door())
            .map(|i| self.instances[i].state)
    }

    /// Building ← BLAS placement of instance `i` for its current state.
    #[must_use]
    pub fn placement(&self, i: usize) -> Rigid {
        self.instances[i].placement()
    }

    /// Every instance baked into building space under its CURRENT state, after the shell —
    /// one mesh for the section cuts and height fields (`owner[tri]` = 0 for the shell,
    /// `i + 1` for instance `i`). Rebuilds a BVH over the union.
    #[must_use]
    pub fn flatten(&self) -> FlatMesh {
        let mut verts: Vec<[f64; 3]> = self.shell.verts.clone();
        let mut tris: Vec<[u32; 3]> = self.shell.tris.clone();
        let mut kinds: Vec<SurfaceKind> = self.shell.kinds.clone();
        if kinds.len() < tris.len() {
            kinds.resize(tris.len(), SurfaceKind::Opaque);
        }
        let mut owner: Vec<u32> = vec![0; tris.len()];
        for (i, inst) in self.instances.iter().enumerate() {
            let place = inst.placement();
            let base = verts.len() as u32;
            verts.extend(inst.blas.verts.iter().map(|&v| place.point(v)));
            tris.extend(
                inst.blas
                    .tris
                    .iter()
                    .map(|t| [t[0] + base, t[1] + base, t[2] + base]),
            );
            kinds.extend((0..inst.blas.tris.len()).map(|t| inst.blas.kind(t as u32)));
            owner.extend(std::iter::repeat_n(i as u32 + 1, inst.blas.tris.len()));
        }
        let bvh = Bvh::build(&verts, &tris);
        FlatMesh {
            mesh: BvhSidecar {
                verts,
                tris,
                bvh,
                kinds,
            },
            owner,
        }
    }
}

/// [`CompoundBuilding::flatten`]'s output: one sidecar-shaped mesh plus the owner of every
/// triangle.
#[derive(Debug)]
pub struct FlatMesh {
    pub mesh: BvhSidecar,
    /// Per triangle: `0` = shell, `i + 1` = instance `i`.
    pub owner: Vec<u32>,
}

impl FlatMesh {
    /// The instance index owning triangle `tri` (`None` for the shell or out of range).
    #[must_use]
    pub fn owner_of(&self, tri: u32) -> Option<usize> {
        match self.owner.get(tri as usize) {
            Some(0) | None => None,
            Some(&o) => Some(o as usize - 1),
        }
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
