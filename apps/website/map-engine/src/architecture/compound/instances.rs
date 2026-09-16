//! Role: instances.
//! Position: `architecture/compound` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::architecture::compound::assembly::CompoundError;
use crate::architecture::compound::assembly::CoverTier;
use crate::architecture::compound::assembly::PlacementSource;
use crate::architecture::compound::doors::DoorRecord;
use crate::architecture::compound::doors::DoorState;
use crate::architecture::compound::transform::Rigid;
use crate::spatial::bvh::sidecar::BvhSidecar;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

/// What an instance is, for LOS attribution and lane routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstanceKind {
    /// The building's own collision mesh (never emitted as an instance record; the shell is `shellBvh`).
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

    /// A whole tree: one BLAS whose kinds table carries the trunk (opaque) and the foliage colliders (foliage) per triangle.
    Tree,

    /// A canopy proxy built from the visual LOD when the tree's collision mesh has no foliage collider (all-Foliage BLAS).
    TreeCanopy,

    /// Anything else placed in the building (radiators, lights, decorations).
    Prop,
}

/// One.
pub(crate) fn one() -> f64 {
    1.0
}

/// A rigid placement: position, unit quaternion, uniform scale.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalTransform {
    /// Pos.
    pub pos: [f64; 3],

    /// `[x, y, z, w]`.
    pub quat: [f64; 4],

    /// Scale.
    #[serde(default = "one")]
    pub scale: f64,
}

impl LocalTransform {
    /// Identity.
    #[must_use]
    pub fn identity() -> Self {
        Self {
            pos: [0.0; 3],
            quat: [0.0, 0.0, 0.0, 1.0],
            scale: 1.0,
        }
    }
}

impl LocalTransform {
    /// From rigid.
    #[must_use]
    pub fn from_rigid(r: &Rigid) -> Self {
        Self {
            pos: r.t,
            quat: r.to_quat(),
            scale: r.scale,
        }
    }
}

impl LocalTransform {
    /// Rigid.
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

/// One placed BLAS.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceRecord {
    /// Stable within the building: the prefab child's `ID` chain joined by `/`.
    pub id: String,

    /// Kind.
    pub kind: InstanceKind,

    /// The child's prefab path (GUID stripped).
    pub prefab: String,

    /// Sidecar path relative to the prefabs root — the parent of the `buildings/` directory the instances file lives in (`blas/<asset>.bvh`); loaders try the file's own directory first.
    pub blas: String,

    /// The XOB the BLAS was built from (diagnostics; never fetched by the viewer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xob: Option<String>,

    /// Local.
    pub local: LocalTransform,

    /// Door.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub door: Option<DoorRecord>,

    /// Cover.
    pub cover: CoverTier,

    /// Source.
    pub source: PlacementSource,

    /// The instance this one is attached under (`None` = the shell).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

/// The `<slug>.instances.json` document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstancesFile {
    /// Schema version.
    pub schema_version: String,

    /// Prefab id.
    pub prefab_id: String,

    /// The building prefab (`Prefabs/…/X.et`).
    pub resource_name: String,

    /// Shell sidecar path relative to this file's directory.
    pub shell_bvh: String,

    /// Instances.
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

/// One placed BLAS with its live state.
#[derive(Clone, Debug)]
pub struct Instance {
    /// Record.
    pub record: InstanceRecord,

    /// Blas.
    pub blas: Arc<BvhSidecar>,

    /// Building ← BLAS placement at rest (the leaf's CLOSED pose for a door).
    pub local: Rigid,

    /// The BLAS's own AABB (root node bounds), in BLAS space.
    pub bounds: ([f64; 3], [f64; 3]),

    /// State.
    pub state: DoorState,
}

impl Instance {
    /// Building ← BLAS placement for the current state (`local ∘ hinge`).
    #[must_use]
    pub fn placement(&self) -> Rigid {
        if self.is_door() {
            self.local.compose(&self.hinge())
        } else {
            self.local
        }
    }
}

impl Instance {
    /// World (building-space) AABB of the BLAS under the current placement.
    #[must_use]
    pub fn world_aabb(&self) -> ([f64; 3], [f64; 3]) {
        self.placement().aabb_of(self.bounds.0, self.bounds.1)
    }
}

/// Instances from records.
pub fn instances_from_records(
    records: &[InstanceRecord],
    blas_by_path: &HashMap<String, Arc<BvhSidecar>>,
) -> Result<Vec<Instance>, CompoundError> {
    let mut missing: Vec<String> = Vec::new();
    for r in records {
        if !blas_by_path.contains_key(&r.blas) && !missing.contains(&r.blas) {
            missing.push(r.blas.clone());
        }
    }
    if !missing.is_empty() {
        return Err(CompoundError::MissingBlas(missing));
    }
    {
        let mut staged = Vec::with_capacity(records.len());
        for r in records {
            let blas = Arc::clone(&blas_by_path[&r.blas]);

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
        Ok(staged)
    }
}
