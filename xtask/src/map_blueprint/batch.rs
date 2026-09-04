//! `cargo xtask map bvh-batch` — the offline BLAS / instance pipeline (T-090.11.2); its
//! inspection twin `map xob-inspect` lives in `inspect.rs`.
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
//! `packages/tbd-schema/schema/building-instances.schema.json`.
//!
//! `--scene <spec.json>` walks extra hand-placed roots (trees around the house) into
//! `buildings/<slug>.scene.json` — the same document shape, `source: scene`.
//!
//! Nothing extracted from the paks is written except the derived sidecars and JSON.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result, bail};
use map_engine_core::building_compound::{
    CoverTier, DoorRecord, INSTANCES_SCHEMA_VERSION, InstanceKind, InstanceRecord, InstancesFile,
    LocalTransform, PlacementSource,
};
use map_engine_core::bvh::{Bvh, BvhSidecar, SurfaceKind, emit_bytes, lift_verts, quantize_verts};
use map_engine_core::geometry::rigid::Rigid;
use serde::Deserialize;

use super::pak::{AssetSource, DirSource, LayeredSource, PakSet};
use super::prefab::{PrefabResolver, ResolvedPrefab};
use super::surface_kind::{kind_for_gamemat, kind_for_layer, parse_kind_override};
use super::xob::{self, XobMesh};
use super::xob_nodes::{XobNodes, parse_head_nodes};

/// Recursion bound for the child walk (door set → leaf is depth 2; compositions 2–3).
const MAX_DEPTH: usize = 8;

/// The operator's hand-extracted tree, layered under the paks when present.
fn default_extract_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("ReforgerExtract/unpacked"))
        .filter(|p| p.is_dir())
}

/// Paks first (the shipped truth), loose extract second.
pub fn open_sources(paks: Option<&Path>, extract: Option<&Path>) -> Result<LayeredSource> {
    let mut layers: Vec<Box<dyn AssetSource>> = Vec::new();
    let pak_dir = paks
        .map(Path::to_path_buf)
        .or_else(PakSet::default_dir)
        .filter(|d| d.is_dir());
    if let Some(dir) = pak_dir {
        let set = PakSet::from_dir(&dir)?;
        eprintln!(
            "  paks: {} files across {} paks under {}",
            set.file_count(),
            set.pak_count(),
            dir.display()
        );
        layers.push(Box::new(set));
    }
    if let Some(dir) = extract.map(Path::to_path_buf).or_else(default_extract_dir) {
        layers.push(Box::new(DirSource { root: dir }));
    }
    if layers.is_empty() {
        bail!("no asset source: pass --paks <dir> or --extract <dir>");
    }
    Ok(LayeredSource { layers })
}

/// One decoded XOB: its collision mesh (if any), node table (if any) and the sidecar built
/// from it.
pub struct Asset {
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

/// Which COLL records a BLAS is built from (T-090.12.4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayerPolicy {
    /// Every record — the physics shells included (the T-090.11 emit).
    All,
    /// Only records whose layer preset stops a projectile
    /// ([`preset_stops_projectile`]): what the engine's `Projectile` trace sees.
    #[default]
    Projectile,
}

impl Asset {
    #[must_use]
    pub fn has_collision(&self) -> bool {
        self.sidecar_bytes.is_some()
    }

    /// Triangles the sidecar carries (the [`LayerPolicy`] survivors).
    #[must_use]
    pub fn kept_tris(&self) -> usize {
        self.kept.iter().filter(|k| **k).count()
    }

    #[must_use]
    pub fn kind_counts(&self) -> (usize, usize, usize) {
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
    pub fn bounds(&self) -> Option<([f64; 3], [f64; 3])> {
        self.coll.as_ref().map(|m| xob::aabb(&m.verts))
    }
}

/// Per-triangle kinds from the COLL subrange materials (+ the record's layer preset as the
/// only opinion when a record carries no materials), with `--kind <record>=<kind>` overrides.
pub fn classify_kinds(
    mesh: &XobMesh,
    nodes: Option<&XobNodes>,
    overrides: &[(u16, SurfaceKind)],
) -> (Vec<SurfaceKind>, Vec<String>) {
    let layers: Vec<String> = mesh
        .records
        .iter()
        .map(|r| {
            nodes
                .and_then(|n| n.name(u32::from(r.layer_idx)))
                .unwrap_or("?")
                .to_string()
        })
        .collect();
    let kinds = (0..mesh.tris.len())
        .map(|t| {
            let rec = mesh.tri_submesh[t];
            if let Some((_, k)) = overrides.iter().find(|(r, _)| *r == rec) {
                return *k;
            }
            let material = mesh.tri_material[t];
            if material != u32::MAX {
                if let Some(name) = nodes.and_then(|n| n.name(material)) {
                    return kind_for_gamemat(name);
                }
            }
            layers
                .get(rec as usize)
                .and_then(|l| kind_for_layer(l))
                .unwrap_or(SurfaceKind::Opaque)
        })
        .collect();
    (kinds, layers)
}

/// Decode one XOB (bytes already read) into an [`Asset`].
pub fn decode_asset(
    path: &str,
    data: &[u8],
    overrides: &[(u16, SurfaceKind)],
    policy: LayerPolicy,
) -> Asset {
    let stem = Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string());
    let nodes = parse_head_nodes(data).ok();
    let coll = if xob::has_coll(data) {
        xob::parse_coll(data).ok()
    } else {
        None
    };
    let (kinds, layers) = match &coll {
        Some(m) => classify_kinds(m, nodes.as_ref(), overrides),
        None => (Vec::new(), Vec::new()),
    };
    // The policy decides per record; a `--kind` override on a record keeps it (the operator
    // has looked at it), an unknown preset is kept.
    let (kept, layer_census) = match &coll {
        Some(m) => {
            let keep_rec: Vec<bool> = (0..layers.len())
                .map(|rec| {
                    policy == LayerPolicy::All
                        || overrides.iter().any(|(r, _)| usize::from(*r) == rec)
                        || super::surface_kind::preset_stops_projectile(&layers[rec]) != Some(false)
                })
                .collect();
            let kept: Vec<bool> = (0..m.tris.len())
                .map(|t| {
                    keep_rec
                        .get(m.tri_submesh[t] as usize)
                        .copied()
                        .unwrap_or(true)
                })
                .collect();
            let mut census: Vec<(String, usize, usize)> =
                layers.iter().map(|l| (l.clone(), 0, 0)).collect();
            for (t, keep) in kept.iter().enumerate() {
                let rec = m.tri_submesh[t] as usize;
                if let Some(c) = census.get_mut(rec) {
                    if *keep {
                        c.1 += 1;
                    } else {
                        c.2 += 1;
                    }
                }
            }
            (kept, census)
        }
        None => (Vec::new(), Vec::new()),
    };
    let sidecar_bytes = coll.as_ref().and_then(|m| {
        let verts_f32 = quantize_verts(&m.verts);
        if verts_f32.iter().flatten().any(|c| !c.is_finite()) || m.tris.is_empty() {
            return None;
        }
        let tris: Vec<[u32; 3]> = m
            .tris
            .iter()
            .zip(&kept)
            .filter(|(_, k)| **k)
            .map(|(t, _)| *t)
            .collect();
        let kinds_kept: Vec<SurfaceKind> = kinds
            .iter()
            .zip(&kept)
            .filter(|(_, k)| **k)
            .map(|(k, _)| *k)
            .collect();
        if tris.is_empty() {
            return None;
        }
        let lifted = lift_verts(&verts_f32);
        let bvh = Bvh::build(&lifted, &tris);
        let bytes = emit_bytes(&verts_f32, &tris, &kinds_kept, &bvh);
        BvhSidecar::parse(&bytes).ok().map(|_| bytes)
    });
    Asset {
        path: path.to_string(),
        stem,
        node_count: nodes.as_ref().map_or(0, |n| n.nodes.len()),
        coll,
        nodes,
        kinds,
        layers,
        kept,
        layer_census,
        sidecar_bytes,
    }
}

/// Memoized XOB decoding; stems are kept unique across paths.
pub struct AssetCache<'a> {
    source: &'a dyn AssetSource,
    overrides_for: Option<(String, Vec<(u16, SurfaceKind)>)>,
    policy: LayerPolicy,
    by_path: HashMap<String, Rc<Asset>>,
    stems: HashMap<String, String>,
}

impl<'a> AssetCache<'a> {
    pub fn new(source: &'a dyn AssetSource) -> Self {
        Self {
            source,
            overrides_for: None,
            policy: LayerPolicy::default(),
            by_path: HashMap::new(),
            stems: HashMap::new(),
        }
    }

    /// The record policy for every decode from now on (set before the first `load`).
    pub fn set_policy(&mut self, policy: LayerPolicy) {
        self.policy = policy;
    }

    #[must_use]
    pub fn policy(&self) -> LayerPolicy {
        self.policy
    }

    /// `--kind` overrides apply to one XOB (the shell).
    pub fn set_overrides(&mut self, xob_path: &str, overrides: Vec<(u16, SurfaceKind)>) {
        self.overrides_for = Some((super::pak::normalize_path(xob_path), overrides));
    }

    pub fn load(&mut self, xob_path: &str) -> Result<Rc<Asset>> {
        let key = super::pak::normalize_path(xob_path);
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

    pub fn loaded(&self) -> impl Iterator<Item = &Rc<Asset>> {
        self.by_path.values()
    }
}

/// Furniture / prop cover heuristic on the prefab path (the Workbench extractor's rule,
/// extended): storage furniture is full cover, seats and wall decoration none, the rest of
/// the furniture low.
#[must_use]
pub fn cover_for_prefab(path: &str) -> (bool, CoverTier) {
    let p = path.to_ascii_lowercase();
    let furniture = p.contains("/furniture/")
        || [
            "table",
            "chair",
            "bed",
            "cupboard",
            "wardrobe",
            "bench",
            "crate",
            "sofa",
            "dresser",
            "kitchen",
            "fridge",
            "stove",
            "piano",
            "rack",
            "shelf",
            "desk",
            "cabinet",
            "boiler",
            "workbench",
            "pallet",
            "box",
        ]
        .iter()
        .any(|k| p.contains(k));
    let none_kw = [
        "chair",
        "stool",
        "lamp",
        "light",
        "painting",
        "clock",
        "curtain",
        "plant",
        "mirror",
        "switch",
        "faucet",
        "drain",
        "grate",
        "skull",
        "hide",
        "radio",
        "notebook",
        "paintcan",
        "bucket",
        "broom",
        "extinguisher",
        "jerrycan",
        "wateringcan",
        "basket",
        "litter",
        "cardboard_0",
        "sack",
        "suitcase",
        "kindling",
        "ladder",
    ];
    let full_kw = [
        "cupboard", "wardrobe", "fridge", "kitchen", "dresser", "rack", "stove", "boiler", "shelf",
        "cabinet", "piano",
    ];
    let tier = if none_kw.iter().any(|k| p.contains(k)) {
        CoverTier::None
    } else if full_kw.iter().any(|k| p.contains(k)) {
        CoverTier::Full
    } else if furniture {
        CoverTier::Low
    } else {
        CoverTier::None
    };
    (furniture, tier)
}

/// What kind of instance a resolved prefab (with a collision mesh) is.
#[must_use]
pub fn classify_prefab(prefab: &ResolvedPrefab, asset: &Asset) -> InstanceKind {
    let p = prefab.path.to_ascii_lowercase();
    let (opaque, glass, foliage) = asset.kind_counts();
    if prefab.door.is_some() || prefab.sliding.is_some() {
        return InstanceKind::DoorLeaf;
    }
    if p.contains("/vegetation/") || p.contains("/tree/") {
        return InstanceKind::Tree;
    }
    if (glass > 0 && opaque == 0 && foliage == 0)
        || p.contains("/glass")
        || p.contains("glass_")
        || prefab.class.to_ascii_lowercase().contains("glass")
    {
        return InstanceKind::Glass;
    }
    if p.contains("/doors/") {
        return InstanceKind::DoorFrame;
    }
    if p.contains("/windows/") {
        return InstanceKind::WindowFrame;
    }
    if cover_for_prefab(&prefab.path).0 {
        return InstanceKind::Furniture;
    }
    InstanceKind::Prop
}

/// The child-entity walk state.
pub struct Walker<'a> {
    pub resolver: PrefabResolver<'a>,
    pub assets: AssetCache<'a>,
    pub instances: Vec<InstanceRecord>,
    pub notes: Vec<String>,
}

impl<'a> Walker<'a> {
    pub fn new(source: &'a dyn AssetSource) -> Self {
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
    pub fn walk(
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
                        id: id.to_string(),
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
pub struct SceneSpec {
    pub entries: Vec<SceneEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneEntry {
    pub id: String,
    pub prefab: String,
    pub pos: [f64; 3],
    #[serde(default)]
    pub angles_deg: [f64; 3],
    #[serde(default = "one")]
    pub scale: f64,
}

fn one() -> f64 {
    1.0
}

pub(super) fn slug_of(prefab_path: &str) -> String {
    Path::new(prefab_path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "prefab".into())
}

fn validate_instances(file: &InstancesFile, schema_path: &Path) -> Result<serde_json::Value> {
    let value = serde_json::to_value(file)?;
    let schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(schema_path).with_context(|| schema_path.display().to_string())?,
    )?;
    let validator =
        jsonschema::validator_for(&schema).map_err(|e| anyhow::anyhow!("schema compile: {e}"))?;
    let errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|e| format!("{} @ {}", e, e.instance_path()))
        .collect();
    if !errors.is_empty() {
        bail!(
            "instances JSON fails the schema:\n  {}",
            errors.join("\n  ")
        );
    }
    Ok(value)
}

pub(super) fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<bool> {
    if fs::read(path).map(|old| old == bytes).unwrap_or(false) {
        return Ok(false);
    }
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    fs::write(path, bytes).with_context(|| path.display().to_string())?;
    Ok(true)
}

/// `map bvh-batch --prefab <Prefabs/…/X.et> [--slug <s>] [--out <dir>] [--paks <dir>]
/// [--extract <dir>] [--scene <spec.json>] [--kind <record>=<kind>]… [--dry-run]`
pub fn run_bvh_batch(args: &[String]) -> Result<u8> {
    // T-090.12.2 — the whole-catalogue lane lives in `library.rs`.
    if args.iter().any(|a| a == "--all-prefabs") {
        return super::library_cli::run(args);
    }
    let mut prefab: Option<String> = None;
    let mut slug: Option<String> = None;
    let mut out: Option<PathBuf> = None;
    let mut paks: Option<PathBuf> = None;
    let mut extract: Option<PathBuf> = None;
    let mut scene: Option<PathBuf> = None;
    let mut overrides: Vec<(u16, SurfaceKind)> = Vec::new();
    let mut dry_run = false;
    let mut all_layers = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--prefab" if i + 1 < args.len() => {
                prefab = Some(args[i + 1].clone());
                i += 2;
            }
            "--slug" if i + 1 < args.len() => {
                slug = Some(args[i + 1].clone());
                i += 2;
            }
            "--out" if i + 1 < args.len() => {
                out = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--paks" if i + 1 < args.len() => {
                paks = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--extract" if i + 1 < args.len() => {
                extract = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--scene" if i + 1 < args.len() => {
                scene = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--kind" if i + 1 < args.len() => {
                let o = parse_kind_override(&args[i + 1]).with_context(|| {
                    format!(
                        "--kind expects <record>=<opaque|glass|foliage>, got {}",
                        args[i + 1]
                    )
                })?;
                overrides.push(o);
                i += 2;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            "--all-layers" => {
                all_layers = true;
                i += 1;
            }
            other => {
                eprintln!(
                    "bvh-batch: unknown arg {other} (usage: --prefab <Prefabs/…/X.et> [--slug <s>] [--out <dir>] \
                     [--paks <dir>] [--extract <dir>] [--scene <spec.json>] [--kind <record>=<kind>]… [--dry-run])"
                );
                return Ok(1);
            }
        }
    }
    let prefab = prefab.context("--prefab <Prefabs/…/X.et> is required")?;
    let root = crate::root::find_repo_root()?;
    let out_dir = out.unwrap_or_else(|| root.join("packages/map-assets/everon/prefabs"));
    let schema = root.join("packages/tbd-schema/schema/building-instances.schema.json");
    let slug = slug.unwrap_or_else(|| slug_of(&prefab));

    let source = open_sources(paks.as_deref(), extract.as_deref())?;
    let mut walker = Walker::new(&source);
    // The shell's --kind overrides bind to the building's own model.
    let root_prefab = walker.resolver.resolve(&prefab)?;
    let shell_xob = root_prefab
        .mesh
        .clone()
        .with_context(|| format!("{prefab}: no MeshObject in its chain — not a building"))?;
    if all_layers {
        walker.assets.set_policy(LayerPolicy::All);
    }
    if !overrides.is_empty() {
        walker.assets.set_overrides(&shell_xob, overrides);
    }
    let shell = walker
        .walk(
            &prefab,
            &slug,
            None,
            Rigid::identity(),
            PlacementSource::PrefabCoords,
            true,
            0,
        )?
        .context("shell model missing")?;
    let shell_bytes = shell.sidecar_bytes.clone().with_context(|| {
        format!("{shell_xob}: no collision chunk — cannot build the shell sidecar")
    })?;

    // Scene roots (trees etc.) go to a second document.
    let mut scene_walker_instances: Vec<InstanceRecord> = Vec::new();
    let mut scene_notes: Vec<String> = Vec::new();
    if let Some(spec_path) = &scene {
        let spec: SceneSpec = serde_json::from_str(
            &fs::read_to_string(spec_path).with_context(|| spec_path.display().to_string())?,
        )
        .with_context(|| format!("parse scene spec {}", spec_path.display()))?;
        let before = walker.instances.len();
        for e in &spec.entries {
            let t = Rigid::from_enfusion(e.pos, e.angles_deg, e.scale);
            if let Err(err) =
                walker.walk(&e.prefab, &e.id, None, t, PlacementSource::Scene, false, 1)
            {
                walker
                    .notes
                    .push(format!("scene {}: {} ({err:#})", e.id, e.prefab));
            }
        }
        scene_walker_instances = walker.instances.split_off(before);
        // Notes written while walking scene roots belong to the scene document.
        scene_notes = walker
            .notes
            .iter()
            .filter(|n| spec.entries.iter().any(|e| n.starts_with(&e.id)))
            .cloned()
            .collect();
        walker
            .notes
            .retain(|n| !spec.entries.iter().any(|e| n.starts_with(&e.id)));
    }

    let instances = InstancesFile {
        schema_version: INSTANCES_SCHEMA_VERSION.into(),
        prefab_id: slug.clone(),
        resource_name: prefab.clone(),
        shell_bvh: format!("{slug}.bvh"),
        instances: walker.instances.clone(),
        notes: walker.notes.clone(),
    };
    let value = validate_instances(&instances, &schema)?;
    let scene_doc =
        (!scene_walker_instances.is_empty() || scene.is_some()).then(|| InstancesFile {
            schema_version: INSTANCES_SCHEMA_VERSION.into(),
            prefab_id: slug.clone(),
            resource_name: prefab.clone(),
            shell_bvh: String::new(),
            instances: scene_walker_instances,
            notes: scene_notes,
        });
    let scene_value = match &scene_doc {
        Some(d) => Some(validate_instances(d, &schema)?),
        None => None,
    };

    // Report.
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    for inst in &instances.instances {
        *by_kind
            .entry(format!("{:?}", inst.kind).to_ascii_lowercase())
            .or_default() += 1;
    }
    let by_source: BTreeMap<String, usize> =
        instances
            .instances
            .iter()
            .fold(BTreeMap::new(), |mut m, i| {
                *m.entry(format!("{:?}", i.source)).or_default() += 1;
                m
            });
    let used: Vec<&str> = instances.blas_paths();
    let scene_used: Vec<&str> = scene_doc
        .as_ref()
        .map(|d| d.blas_paths())
        .unwrap_or_default();
    let (so, sg, sf) = shell.kind_counts();
    println!(
        "bvh-batch {slug}: shell {} ({} tris · kinds opaque {so} / glass {sg} / foliage {sf}) · {} instances {:?} · sources {:?} · {} BLAS · {} notes{}",
        shell_xob,
        shell.kinds.len(),
        instances.instances.len(),
        by_kind,
        by_source,
        used.len() + scene_used.iter().filter(|p| !used.contains(p)).count(),
        instances.notes.len(),
        if dry_run {
            " (dry run — nothing written)"
        } else {
            ""
        }
    );
    for n in &instances.notes {
        println!("  note: {n}");
    }
    if let Some(d) = &scene_doc {
        println!("  scene: {} instances", d.instances.len());
        for n in &d.notes {
            println!("  scene note: {n}");
        }
    }
    if dry_run {
        return Ok(0);
    }

    // Write: shell, every referenced BLAS (deduplicated by stem), the two documents.
    let mut written = 0usize;
    written += usize::from(write_if_changed(
        &out_dir.join("buildings").join(format!("{slug}.bvh")),
        &shell_bytes,
    )?);
    let mut blas_written: Vec<String> = Vec::new();
    for asset in walker.assets.loaded() {
        let rel = format!("blas/{}.bvh", asset.stem);
        if !used.contains(&rel.as_str()) && !scene_used.contains(&rel.as_str()) {
            continue;
        }
        if let Some(bytes) = &asset.sidecar_bytes {
            if write_if_changed(&out_dir.join(&rel), bytes)? {
                blas_written.push(rel);
            }
        }
    }
    written += blas_written.len();
    let json = serde_json::to_string_pretty(&value)? + "\n";
    written += usize::from(write_if_changed(
        &out_dir
            .join("buildings")
            .join(format!("{slug}.instances.json")),
        json.as_bytes(),
    )?);
    if let Some(v) = &scene_value {
        let json = serde_json::to_string_pretty(v)? + "\n";
        written += usize::from(write_if_changed(
            &out_dir.join("buildings").join(format!("{slug}.scene.json")),
            json.as_bytes(),
        )?);
    }
    println!(
        "  wrote {written} file(s) under {} ({} BLAS changed)",
        out_dir.display(),
        blas_written.len()
    );
    Ok(0)
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
