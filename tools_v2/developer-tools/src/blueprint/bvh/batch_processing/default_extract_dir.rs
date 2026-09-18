use super::*;

/// The operator's hand-extracted tree, layered under the paks when present.
pub(super) fn default_extract_dir() -> Option<PathBuf> {
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
            if material != u32::MAX
                && let Some(name) = nodes.and_then(|n| n.name(material))
            {
                return kind_for_gamemat(name);
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
                        || super::super::surface_kind::preset_stops_projectile(&layers[rec])
                            != Some(false)
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

pub(super) fn one() -> f64 {
    1.0
}

pub(crate) fn slug_of(prefab_path: &str) -> String {
    Path::new(prefab_path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "prefab".into())
}

pub(super) fn validate_instances(
    file: &InstancesFile,
    schema_path: &Path,
) -> Result<serde_json::Value> {
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

pub(crate) fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<bool> {
    if fs::read(path).map(|old| old == bytes).unwrap_or(false) {
        return Ok(false);
    }
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    fs::write(path, bytes).with_context(|| path.display().to_string())?;
    Ok(true)
}
