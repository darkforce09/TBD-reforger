use super::*;

pub(super) fn assemble_manifest(
    terrain: &str,
    descriptors: &[PrefabDescriptor],
    blas: &BTreeMap<String, Vec<u8>>,
    census: &HashMap<u32, u64>,
    hot_n: usize,
    canopy_hull: u32,
) -> Result<BlasManifest> {
    let mut blas_entries = Vec::with_capacity(blas.len());
    for (path, bytes) in blas {
        let sc = BvhSidecar::parse(bytes).with_context(|| format!("parse {path}"))?;
        let (o, g, f) = sc.kind_counts();
        blas_entries.push(BlasEntry {
            path: path.clone(),
            bytes: bytes.len() as u64,
            tris: u32::try_from(sc.tris.len()).unwrap_or(u32::MAX),
            kinds: [o as u32, g as u32, f as u32],
        });
    }
    let mut totals = Totals::default();
    let mut desc_entries = Vec::with_capacity(descriptors.len());
    let mut attributed: HashSet<&str> = HashSet::new();
    for d in descriptors {
        let kt = totals.by_kind.entry(d.kind.clone()).or_default();
        kt.prefabs += 1;
        totals.prefabs += 1;
        if d.blocks {
            kt.blocks += 1;
            totals.blocks += 1;
        } else {
            match d.reason.as_deref() {
                Some("no-mesh") => kt.no_mesh += 1,
                Some("model-unreadable") => kt.model_unreadable += 1,
                Some("no-coll") => kt.no_coll += 1,
                Some("empty-coll") => kt.empty_coll += 1,
                Some("no-fire-geo") => kt.no_fire_geo += 1,
                _ => kt.unresolved += 1,
            }
        }
        if d.canopy {
            totals.canopy += 1;
        }
        let paths: Vec<String> = d.blas_paths().iter().map(ToString::to_string).collect();
        for p in &d.blas_paths() {
            if attributed.insert(p) {
                kt.bytes += blas.get(*p).map_or(0, |b| b.len() as u64);
            }
        }
        desc_entries.push(DescEntry {
            pid: d.prefab_id,
            path: format!("descriptors/{}.json", d.prefab_id),
            kind: d.kind.clone(),
            blocks: d.blocks,
            canopy: d.canopy,
            blas: paths,
            instance_count: u32::try_from(d.instances.len()).unwrap_or(u32::MAX),
            instances_in_world: census.get(&d.prefab_id).copied().unwrap_or(0),
        });
    }
    totals.canopy_hull = canopy_hull;
    totals.blas_files = u32::try_from(blas.len()).unwrap_or(u32::MAX);
    totals.blas_bytes = blas.values().map(|b| b.len() as u64).sum();
    let mut hot: Vec<(u64, u32)> = desc_entries
        .iter()
        .filter(|d| d.blocks)
        .map(|d| (d.instances_in_world, d.pid))
        .collect();
    hot.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let hot = hot.into_iter().take(hot_n).map(|(_, p)| p).collect();
    Ok(BlasManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.into(),
        terrain_id: terrain.to_string(),
        blas: blas_entries,
        descriptors: desc_entries,
        hot,
        totals,
    })
}

/// Validate `value` against the JSON schema at `schema_path`.
pub fn validate_against(value: &Value, schema_path: &Path, label: &str) -> Result<()> {
    let schema: Value = serde_json::from_str(
        &fs::read_to_string(schema_path).with_context(|| schema_path.display().to_string())?,
    )?;
    let validator =
        jsonschema::validator_for(&schema).map_err(|e| anyhow::anyhow!("schema compile: {e}"))?;
    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|e| format!("{} @ {}", e, e.instance_path()))
        .take(8)
        .collect();
    if !errors.is_empty() {
        bail!(
            "{label} fails {}:\n  {}",
            schema_path.display(),
            errors.join("\n  ")
        );
    }
    Ok(())
}

pub(super) fn pretty_nl(v: &Value) -> Vec<u8> {
    let mut s = serde_json::to_string_pretty(v).expect("serialize");
    s.push('\n');
    s.into_bytes()
}

/// Validate every document against the schemas and write the library under `out_dir`
/// (descriptors, BLAS, and the manifest unless `partial`). Returns the number of files written.
pub fn write_library(
    out_dir: &Path,
    lib: &Library,
    schema_dir: &Path,
    partial: bool,
) -> Result<usize> {
    let desc_schema = schema_dir.join("prefab-descriptor.schema.json");
    let manifest_schema = schema_dir.join("blas-manifest.schema.json");
    let mut written = 0usize;
    for d in &lib.descriptors {
        let v = serde_json::to_value(d)?;
        validate_against(&v, &desc_schema, &format!("descriptor {}", d.prefab_id))?;
        let path = out_dir
            .join("descriptors")
            .join(format!("{}.json", d.prefab_id));
        written += usize::from(write_if_changed(&path, &pretty_nl(&v))?);
    }
    for (rel, bytes) in &lib.blas {
        written += usize::from(write_if_changed(&out_dir.join(rel), bytes)?);
    }
    if !partial {
        let v = serde_json::to_value(&lib.manifest)?;
        validate_against(&v, &manifest_schema, "blas-manifest")?;
        written += usize::from(write_if_changed(
            &out_dir.join("blas-manifest.json"),
            &pretty_nl(&v),
        )?);
    }
    Ok(written)
}
