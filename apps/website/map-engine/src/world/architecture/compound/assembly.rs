//! Role: assembly.
//! Position: `world/architecture/compound` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::spatial::bvh::sidecar::BvhSidecar;
use crate::spatial::bvh::surface::SurfaceKind;
use crate::spatial::bvh::traversal::Bvh;
use crate::world::architecture::compound::instances::Instance;
use crate::world::architecture::compound::instances::InstanceRecord;
use crate::world::architecture::compound::instances::instances_from_records;
use crate::world::architecture::compound::transform::Rigid;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

/// Contract version of `<slug>.instances.json`.
pub const INSTANCES_SCHEMA_VERSION: &str = "1.0.0";

/// Cover an instance offers a prone / crouched soldier, from the prefab category heuristic the Workbench extractor used (`TBD_BuildingArchitectExtractor.c`): cupboards and wardrobes are full cover, tables and beds low cover, chairs none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverTier {
    /// Full.
    Full,

    /// Low.
    Low,

    /// None.
    None,
}

/// Where the instance's transform came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlacementSource {
    /// Xob socket.
    XobSocket,

    /// Recon.
    Recon,

    /// Explicit `coords` / `angles` / `scale` in the prefab text.
    PrefabCoords,

    /// A hand-placed scene entry (`<slug>.scene.json`).
    Scene,
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

/// The shell plus every placed instance — the two-level acceleration structure the LOS walk, the wash and the drawing run over (`building_compound_los.rs`).
#[derive(Clone, Debug)]
pub struct CompoundBuilding {
    /// Shell.
    pub shell: Arc<BvhSidecar>,

    /// Instances.
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
}

impl CompoundBuilding {
    /// Shell + records, every record's `blas` path resolved through `blas_by_path` (keys are the record's `blas` strings verbatim). Atomic: a missing BLAS assembles nothing.
    pub fn assemble(
        shell: Arc<BvhSidecar>,
        records: &[InstanceRecord],
        blas_by_path: &HashMap<String, Arc<BvhSidecar>>,
    ) -> Result<Self, CompoundError> {
        let mut c = Self::new(shell);
        c.append(records, blas_by_path)?;
        Ok(c)
    }
}

impl CompoundBuilding {
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
        self.instances
            .extend(instances_from_records(records, blas_by_path)?);
        Ok(())
    }
}

impl CompoundBuilding {
    /// Index of the instance with this id.
    #[must_use]
    pub fn instance_index(&self, id: &str) -> Option<usize> {
        self.instances.iter().position(|i| i.record.id == id)
    }
}

impl CompoundBuilding {
    /// Building ← BLAS placement of instance `i` for its current state.
    #[must_use]
    pub fn placement(&self, i: usize) -> Rigid {
        self.instances[i].placement()
    }
}

impl CompoundBuilding {
    /// Every instance baked into building space under its CURRENT state, after the shell — one mesh for the section cuts and height fields (`owner[tri]` = 0 for the shell, `i + 1` for instance `i`). Rebuilds a BVH over the union.
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

/// [`CompoundBuilding::flatten`]'s output: one sidecar-shaped mesh plus the owner of every triangle.
#[derive(Debug)]
pub struct FlatMesh {
    /// Mesh.
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
