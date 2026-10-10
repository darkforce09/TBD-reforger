//! **Role:** `cargo xtask map bvh-batch` — the offline BLAS / instance pipeline; its
//! inspection twin `map xob-inspect` lives in [`super::super::mesh_decoding::archive_inspection`].
//!
//! `bvh-batch --prefab <Prefabs/…/X.et>` walks the prefab closure straight out of the game
//! paks: the building's own collision mesh becomes the shell sidecar
//! (`buildings/<slug>.bvh`, version 2 with per-triangle [`SurfaceKind`]s from the COLL
//! game materials), every child entity with a collision mesh (door sets → frame + leaf,
//! window sets → frame + panes, the furniture composition, props) gets one deduplicated
//! BLAS under `blas/<stem>.bvh` and one [`InstanceRecord`] with its transform in the
//! building's local frame — socket bones from the parent model's XOB node table
//! (`source: xobSocket`), else the prefab's `coords` / `angles` / `scale`
//! (`prefabCoords`). Output: `buildings/<slug>.instances.json`, validated against
//! `contracts/definitions/building-instances.schema.json`.
//!
//! `--scene <spec.json>` walks extra hand-placed roots (trees around the house) into
//! `buildings/<slug>.scene.json` — the same document shape, `source: scene`.
//!
//! Nothing extracted from the paks is written except the derived sidecars and JSON.
//!
//! **Position:** called by `bvh-batch` and by the prefab library; reads models through [`crate::mesh_decoding`] and prefabs through [`crate::occlusion_sidecars::prefab_templates`].
//! **Signals & state:** the walker's asset cache, one decode per model per run.
//! **Invariants:** a file is written only when its bytes change; instances are in the building's local frame.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::error::{Result, ResultExt, bail};
use building_interiors::compound::assembly::CoverTier;
use building_interiors::compound::assembly::INSTANCES_SCHEMA_VERSION;
use building_interiors::compound::assembly::PlacementSource;
use building_interiors::compound::doors::DoorRecord;
use building_interiors::compound::instances::InstanceKind;
use building_interiors::compound::instances::InstanceRecord;
use building_interiors::compound::instances::InstancesFile;
use building_interiors::compound::instances::LocalTransform;
use geometry_primitives::rigid_transform::Rigid;
use serde::Deserialize;
use spatial_indexes::bounding_volume_hierarchy::sidecar::BvhSidecar;
use spatial_indexes::bounding_volume_hierarchy::sidecar::emit_bytes;
use spatial_indexes::bounding_volume_hierarchy::sidecar::lift_verts;
use spatial_indexes::bounding_volume_hierarchy::sidecar::quantize_verts;
use spatial_indexes::bounding_volume_hierarchy::surface_kind::SurfaceKind;
use spatial_indexes::bounding_volume_hierarchy::triangle_tree::Bvh;

use crate::architectural_analysis::surface_classification::{
    kind_for_game_material, kind_for_layer, parse_kind_override,
};
use crate::mesh_decoding::mesh_format::{self, XobMesh};
use crate::mesh_decoding::node_records::{XobNodes, parse_head_nodes};
use crate::occlusion_sidecars::prefab_templates::{PrefabResolver, ResolvedPrefab};
use enfusion_pak::{AssetSource, DirSource, LayeredSource, PakSet};

/// Recursion bound for the child walk (door set → leaf is depth 2; compositions 2–3).
const MAX_DEPTH: usize = 8;

/// One decoded XOB: its collision mesh (if any), node table (if any) and the sidecar built
/// from it.
pub(crate) struct Asset {
    pub path: String,
    pub stem: String,
    pub coll: Option<XobMesh>,
    pub nodes: Option<XobNodes>,
    pub kinds: Vec<SurfaceKind>,
    /// Per COLL record: layer-preset name (or `?`).
    pub layers: Vec<String>,
    /// Per COLL triangle: emitted into the sidecar under the [`LayerPolicy`] (false = a record
    /// on a preset projectiles do not collide with).
    pub kept: Vec<bool>,
    /// Per COLL record: `(preset, tris kept, tris dropped)`.
    pub layer_census: Vec<(String, usize, usize)>,
    pub sidecar_bytes: Option<Vec<u8>>,
    pub node_count: usize,
}

/// Which COLL records a BLAS is built from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum LayerPolicy {
    /// Every record, the physics shells included.
    All,
    /// Only records whose layer preset stops a projectile
    /// ([`preset_stops_projectile`]): what the engine's `Projectile` trace sees.
    #[default]
    Projectile,
}

impl Asset {
    #[must_use]
    pub(crate) fn has_collision(&self) -> bool {
        self.sidecar_bytes.is_some()
    }

    /// Triangles the sidecar carries (the [`LayerPolicy`] survivors).
    #[must_use]
    pub(crate) fn kept_triangles(&self) -> usize {
        self.kept.iter().filter(|k| **k).count()
    }

    #[must_use]
    pub(crate) fn kind_counts(&self) -> (usize, usize, usize) {
        let mut n = (0, 0, 0);
        for (k, keep) in self.kinds.iter().zip(&self.kept) {
            if !keep {
                continue;
            }
            match k {
                SurfaceKind::Opaque => n.0 += 1,
                SurfaceKind::Glass => n.1 += 1,
                SurfaceKind::Foliage => n.2 += 1,
            }
        }
        n
    }

    /// Root-frame AABB of the collision mesh.
    #[must_use]
    pub(crate) fn bounds(&self) -> Option<([f64; 3], [f64; 3])> {
        self.coll
            .as_ref()
            .map(|m| mesh_format::bounding_box(&m.verts))
    }
}

/// Memoized XOB decoding; stems are kept unique across paths.
pub(crate) struct AssetCache<'a> {
    source: &'a dyn AssetSource,
    overrides_for: Option<(String, Vec<(u16, SurfaceKind)>)>,
    policy: LayerPolicy,
    by_path: HashMap<String, Rc<Asset>>,
    stems: HashMap<String, String>,
}

impl<'a> AssetCache<'a> {
    pub(crate) fn new(source: &'a dyn AssetSource) -> Self {
        Self {
            source,
            overrides_for: None,
            policy: LayerPolicy::default(),
            by_path: HashMap::new(),
            stems: HashMap::new(),
        }
    }

    /// The record policy for every decode from now on (set before the first `load`).
    pub(crate) fn set_policy(&mut self, policy: LayerPolicy) {
        self.policy = policy;
    }

    #[must_use]
    pub(crate) fn policy(&self) -> LayerPolicy {
        self.policy
    }

    /// `--kind` overrides apply to one XOB (the shell).
    pub(crate) fn set_overrides(&mut self, xob_path: &str, overrides: Vec<(u16, SurfaceKind)>) {
        self.overrides_for = Some((enfusion_pak::normalize_path(xob_path), overrides));
    }

    pub(crate) fn load(&mut self, xob_path: &str) -> Result<Rc<Asset>> {
        let key = enfusion_pak::normalize_path(xob_path);
        if let Some(a) = self.by_path.get(&key) {
            return Ok(a.clone());
        }
        let data = self
            .source
            .read(xob_path)
            .with_context(|| format!("read model {xob_path}"))?;
        let overrides: &[(u16, SurfaceKind)] = match &self.overrides_for {
            Some((p, o)) if *p == key => o,
            _ => &[],
        };
        let mut asset = decode_asset(xob_path, &data, overrides, self.policy);
        // Two different models with the same file stem would collide under blas/.
        if let Some(other) = self.stems.get(&asset.stem)
            && other != &key
        {
            let mut h: u32 = 2166136261;
            for b in key.bytes() {
                h = (h ^ u32::from(b)).wrapping_mul(16777619);
            }
            asset.stem = format!("{}_{h:08x}", asset.stem);
        }
        self.stems.insert(asset.stem.clone(), key.clone());
        let rc = Rc::new(asset);
        self.by_path.insert(key, rc.clone());
        Ok(rc)
    }

    pub(crate) fn loaded(&self) -> impl Iterator<Item = &Rc<Asset>> {
        self.by_path.values()
    }
}

/// The child-entity walk state.
pub(crate) struct ChildEntityWalker<'a> {
    pub resolver: PrefabResolver<'a>,
    pub assets: AssetCache<'a>,
    pub instances: Vec<InstanceRecord>,
    pub notes: Vec<String>,
}

impl<'a> ChildEntityWalker<'a> {
    pub(crate) fn new(source: &'a dyn AssetSource) -> Self {
        Self {
            resolver: PrefabResolver::new(source),
            assets: AssetCache::new(source),
            instances: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Walk one entity: emit its instance (when it has a collision mesh and is not the
    /// shell) and recurse into its children. Returns the entity's own asset (the socket
    /// table the children attach to).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn walk(
        &mut self,
        prefab_path: &str,
        id: &str,
        parent_id: Option<&str>,
        transform: Rigid,
        source: PlacementSource,
        is_shell: bool,
        depth: usize,
    ) -> Result<Option<Rc<Asset>>> {
        if depth > MAX_DEPTH {
            self.notes.push(format!(
                "{id}: nesting deeper than {MAX_DEPTH}, children skipped"
            ));
            return Ok(None);
        }
        let prefab = self.resolver.resolve(prefab_path)?;
        let asset = match &prefab.mesh {
            Some(m) => match self.assets.load(m) {
                Ok(a) => Some(a),
                Err(e) => {
                    self.notes
                        .push(format!("{id}: model {m} unreadable ({e:#})"));
                    None
                }
            },
            None => None,
        };
        if !is_shell {
            match &asset {
                Some(a) if a.has_collision() => {
                    let kind = classify_prefab(&prefab, a);
                    let cover = match kind {
                        InstanceKind::Furniture | InstanceKind::Prop => {
                            cover_for_prefab(&prefab.path).1
                        }
                        InstanceKind::DoorLeaf
                        | InstanceKind::DoorFrame
                        | InstanceKind::WindowFrame => CoverTier::Full,
                        InstanceKind::Tree => CoverTier::Full,
                        _ => CoverTier::None,
                    };
                    let door = prefab.door.as_ref().map(|d| DoorRecord {
                        angle_range_deg: d.angle_range_deg,
                        closed_angle_deg: d.closed_angle_deg,
                        initial_angle_deg: d.initial_angle_deg,
                        angle_range_explicit: d.angle_range_explicit,
                        opened_distance: prefab.sliding.as_ref().map(|s| s.opened_distance),
                    });
                    let door = door.or_else(|| {
                        prefab.sliding.as_ref().map(|s| DoorRecord {
                            angle_range_deg: 0.0,
                            closed_angle_deg: 0.0,
                            initial_angle_deg: s.initial_distance,
                            angle_range_explicit: false,
                            opened_distance: Some(s.opened_distance),
                        })
                    });
                    self.instances.push(InstanceRecord {
                        id: id.into(),
                        kind,
                        prefab: prefab.path.clone(),
                        blas: format!("blas/{}.bvh", a.stem),
                        xob: Some(a.path.clone()),
                        local: LocalTransform::from_rigid(&transform),
                        door,
                        cover,
                        source,
                        parent: parent_id.map(ToString::to_string),
                    });
                }
                Some(a) => self.notes.push(format!(
                    "{id}: {} has no collision chunk — no BLAS, no instance",
                    a.path
                )),
                None => {
                    if prefab.mesh.is_none() && prefab.children.is_empty() {
                        self.notes
                            .push(format!("{id}: {} has no mesh", prefab.path));
                    }
                }
            }
        }
        let own_id = if is_shell { None } else { Some(id.to_string()) };
        for (i, child) in prefab.children.iter().enumerate() {
            let label = child
                .pivot_id
                .clone()
                .or_else(|| child.id.clone())
                .unwrap_or_else(|| format!("child{i}"));
            let child_id = if is_shell {
                label.clone()
            } else {
                format!("{id}/{label}")
            };
            let offset = Rigid::from_enfusion(child.coords, child.angles_deg, child.scale);
            let (local, src) = match &child.pivot_id {
                Some(pivot) => {
                    let socket = asset
                        .as_ref()
                        .and_then(|a| a.nodes.as_ref())
                        .and_then(|n| n.socket(pivot));
                    match socket {
                        Some(s) => (s.local.compose(&offset), PlacementSource::XobSocket),
                        None => {
                            self.notes.push(format!(
                                "{child_id}: socket {pivot} not found in {} — prefab coords used",
                                prefab.mesh.as_deref().unwrap_or("(no mesh)")
                            ));
                            (offset, PlacementSource::PrefabCoords)
                        }
                    }
                }
                None => (offset, PlacementSource::PrefabCoords),
            };
            let child_transform = transform.compose(&local);
            let child_source = if source == PlacementSource::Scene {
                PlacementSource::Scene
            } else {
                src
            };
            if child.prefab.is_empty() {
                continue;
            }
            match self.walk(
                &child.prefab,
                &child_id,
                own_id.as_deref().or(parent_id),
                child_transform,
                child_source,
                false,
                depth + 1,
            ) {
                Ok(_) => {}
                Err(e) => self
                    .notes
                    .push(format!("{child_id}: {} unresolved ({e:#})", child.prefab)),
            }
        }
        Ok(asset)
    }
}

/// Hand-placed scene spec (`--scene`): extra roots around the building.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SceneSpecification {
    pub entries: Vec<SceneEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SceneEntry {
    pub(crate) id: String,
    pub(crate) prefab: String,
    pub(crate) pos: [f64; 3],
    #[serde(default)]
    pub(crate) angles_deg: [f64; 3],
    #[serde(default = "default_scale")]
    pub(crate) scale: f64,
}

#[cfg(test)]
#[path = "tests/batch_processing_tests.rs"]
mod tests;

#[path = "batch_processing/asset_sources_and_classification.rs"]
mod asset_sources_and_classification;
pub(crate) use asset_sources_and_classification::classify_prefab;
pub(crate) use asset_sources_and_classification::cover_for_prefab;
pub(crate) use asset_sources_and_classification::decode_asset;
pub(crate) use asset_sources_and_classification::default_pak_dir;
use asset_sources_and_classification::default_scale;
pub(crate) use asset_sources_and_classification::open_sources;
pub(crate) use asset_sources_and_classification::slug_of;
use asset_sources_and_classification::validate_instances;
pub(crate) use asset_sources_and_classification::write_if_changed;

#[path = "batch_processing/batch_command_line.rs"]
mod batch_command_line;
pub use batch_command_line::run_occlusion_sidecar_batch;
