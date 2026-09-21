use super::*;

pub(super) fn gunzip_json(path: &Path) -> Result<Value> {
    let bytes = fs::read(path).with_context(|| path.display().to_string())?;
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes.as_slice())
        .read_to_end(&mut out)
        .with_context(|| format!("gunzip {}", path.display()))?;
    serde_json::from_slice(&out).with_context(|| format!("parse {}", path.display()))
}

/// The catalogue rows, sorted by pid.
pub fn load_prefab_rows(prefabs_gz: &Path) -> Result<Vec<PrefabRow>> {
    let doc = gunzip_json(prefabs_gz)?;
    let rows = doc["prefabs"]
        .as_array()
        .context("prefabs.json.gz: no prefabs array")?;
    let mut out = Vec::with_capacity(rows.len());
    for p in rows {
        let (Some(pid), Some(rn), Some(kind)) = (
            p["prefabId"].as_u64(),
            p["resourceName"].as_str(),
            p["kind"].as_str(),
        ) else {
            continue;
        };
        out.push(PrefabRow {
            pid: u32::try_from(pid).context("prefabId exceeds u32")?,
            resource_name: rn.to_string(),
            kind: kind.to_string(),
        });
    }
    if out.is_empty() {
        bail!("prefabs.json.gz: no prefab rows");
    }
    out.sort_by_key(|r| r.pid);
    Ok(out)
}

/// How many chunk rows place each pid (`objects/chunks/*.json.gz`).
pub fn world_census(chunks_dir: &Path) -> Result<HashMap<u32, u64>> {
    let mut files: Vec<PathBuf> = fs::read_dir(chunks_dir)
        .with_context(|| chunks_dir.display().to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".json.gz"))
        .collect();
    files.sort();
    let mut census = HashMap::new();
    for f in &files {
        for row in load_rows(f)? {
            *census
                .entry(u32::try_from(row.pid).unwrap_or(u32::MAX))
                .or_insert(0) += 1;
        }
    }
    Ok(census)
}

/// The 26 axis / face-diagonal / corner directions the canopy hull samples.
pub(super) fn hull_directions() -> Vec<[f64; 3]> {
    let mut dirs = Vec::with_capacity(26);
    for x in -1..=1_i32 {
        for y in -1..=1_i32 {
            for z in -1..=1_i32 {
                if (x, y, z) == (0, 0, 0) {
                    continue;
                }
                let v = [f64::from(x), f64::from(y), f64::from(z)];
                let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                dirs.push([v[0] / n, v[1] / n, v[2] / n]);
            }
        }
    }
    dirs
}

/// The extreme vertex of `verts` along each of 26 directions (a k-DOP sample of the convex
/// hull, ≤ 26 points). `hull_triangles` is O(n⁴) — meant for colliders of a few dozen
/// vertices — so a visual LOD0 (thousands of vertices) is never fed to it directly.
#[must_use]
pub fn hull_sample(verts: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let mut out: Vec<[f64; 3]> = Vec::with_capacity(26);
    for d in hull_directions() {
        let mut best: Option<(f64, [f64; 3])> = None;
        for v in verts {
            let s = v[0] * d[0] + v[1] * d[1] + v[2] * d[2];
            if best.is_none_or(|(bs, _)| s > bs) {
                best = Some((s, *v));
            }
        }
        if let Some((_, p)) = best
            && !out.contains(&p)
        {
            out.push(p);
        }
    }
    out
}

/// An all-Foliage sidecar over the convex hull of the 26-direction extreme points of `verts`
/// (the visual LOD0 canopy fallback).
pub(super) fn canopy_sidecar(all_verts: &[[f64; 3]]) -> Option<Vec<u8>> {
    let verts = hull_sample(all_verts);
    let tris = hull_triangles(&verts);
    if tris.is_empty() {
        return None;
    }
    let q = quantize_verts(&verts);
    if q.iter().flatten().any(|c| !c.is_finite()) {
        return None;
    }
    let lifted = lift_verts(&q);
    let bvh = Bvh::build(&lifted, &tris);
    let kinds = vec![SurfaceKind::Foliage; tris.len()];
    let bytes = emit_bytes(&q, &tris, &kinds, &bvh);
    BvhSidecar::parse(&bytes).ok().map(|_| bytes)
}

/// Why a closure with no collision-bearing instance blocks nothing.
pub(super) fn no_block_reason(
    walker: &mut Walker,
    path: &str,
    root: Option<&Asset>,
) -> &'static str {
    let mesh = walker
        .resolver
        .resolve(path)
        .ok()
        .and_then(|p| p.mesh.clone());
    match (mesh, root) {
        (None, _) => "no-mesh",
        (Some(_), None) => "model-unreadable",
        (Some(_), Some(a)) => match &a.coll {
            Some(m) if m.tris.is_empty() => "empty-coll",
            // Collision exists, but every record sits on a preset projectiles pass through.
            Some(m) if !m.tris.is_empty() && a.kept_tris() == 0 => "no-fire-geo",
            _ => "no-coll",
        },
    }
}

/// Walk every selected catalogue prefab and assemble descriptors, BLAS bytes and the manifest.
pub fn build_library(
    source: &dyn AssetSource,
    rows: &[PrefabRow],
    census: &HashMap<u32, u64>,
    opts: &LibraryOptions,
) -> Result<Library> {
    let mut walker = Walker::new(source);
    walker.assets.set_policy(opts.layer_policy);
    let mut descriptors: Vec<PrefabDescriptor> = Vec::new();
    let mut blas: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut canopy_hull = 0u32;
    let selected = rows
        .iter()
        .filter(|r| opts.only_kinds.is_empty() || opts.only_kinds.iter().any(|k| k == &r.kind))
        .take(opts.limit.unwrap_or(usize::MAX));
    let started = std::time::Instant::now();
    for (done, row) in selected.enumerate() {
        if done > 0 && done % 100 == 0 {
            eprintln!(
                "  … {done} prefabs walked, {} BLAS, {:.0} s",
                blas.len(),
                started.elapsed().as_secs_f64()
            );
        }
        let path = strip_guid(&row.resource_name).to_string();
        let slug = slug_of(&path);
        walker.instances.clear();
        walker.notes.clear();
        // Walked exactly as `bvh-batch` walks a building (the root as the shell), so the child
        // ids and parents are the instances file's; the root record is synthesised.
        let walked = walker.walk(
            &path,
            &slug,
            None,
            Rigid::identity(),
            PlacementSource::PrefabCoords,
            true,
            0,
        );
        let mut instances = std::mem::take(&mut walker.instances);
        let mut notes = std::mem::take(&mut walker.notes);
        let mut reason: Option<String> = None;
        let mut root_asset = None;
        match walked {
            Err(e) => {
                reason = Some("unresolved".into());
                notes.push(format!("{slug}: {path} unresolved ({e:#})"));
            }
            Ok(asset) => {
                root_asset = asset;
                // The root record: a building's shell reads as `Shell`; every other kind keeps
                // the walker's classification (tree / prop / furniture / glass) so a hit names
                // what it is.
                if let Some(a) = root_asset.as_ref().filter(|a| a.has_collision()) {
                    let kind = if row.kind == "building" {
                        InstanceKind::Shell
                    } else {
                        walker
                            .resolver
                            .resolve(&path)
                            .map_or(InstanceKind::Prop, |p| classify_prefab(&p, a))
                    };
                    let cover = match kind {
                        InstanceKind::Furniture | InstanceKind::Prop => cover_for_prefab(&path).1,
                        InstanceKind::Tree => CoverTier::Full,
                        _ => CoverTier::None,
                    };
                    instances.insert(
                        0,
                        InstanceRecord {
                            id: slug.clone(),
                            kind,
                            prefab: path.clone(),
                            blas: format!("blas/{}.bvh", a.stem),
                            xob: Some(a.path.clone()),
                            local: LocalTransform::identity(),
                            door: None,
                            cover,
                            source: PlacementSource::PrefabCoords,
                            parent: None,
                        },
                    );
                }
                if instances.is_empty() {
                    reason =
                        Some(no_block_reason(&mut walker, &path, root_asset.as_deref()).into());
                }
            }
        }
        let shell_bvh = instances
            .iter()
            .find(|i| i.id == slug && i.parent.is_none())
            .map(|i| i.blas.clone())
            .unwrap_or_default();
        // Canopy: Foliage triangles in the COLL, else the visual-LOD0 hull.
        let mut canopy = false;
        let mut canopy_bounds: Option<Bounds3> = None;
        if row.kind == "tree" {
            let foliage = root_asset.as_ref().map_or(0, |a| a.kind_counts().2);
            if foliage > 0 {
                canopy = true;
            } else if let Some(mesh) = walker
                .resolver
                .resolve(&path)
                .ok()
                .and_then(|p| p.mesh.clone())
            {
                match source.read(&mesh).and_then(|d| xob::parse_xob(&d, None)) {
                    Ok(vis) => match canopy_sidecar(&vis.verts) {
                        Some(bytes) => {
                            let stem = root_asset
                                .as_ref()
                                .map_or_else(|| slug_of(&mesh), |a| a.stem.clone());
                            let rel = format!("blas/{stem}_canopy.bvh");
                            let (lo, hi) = xob::aabb(&vis.verts);
                            canopy_bounds = Some(Bounds3 { min: lo, max: hi });
                            blas.insert(rel.clone(), bytes);
                            instances.push(InstanceRecord {
                                id: format!("{slug}/canopy"),
                                kind: InstanceKind::TreeCanopy,
                                prefab: path.clone(),
                                blas: rel,
                                xob: Some(mesh.clone()),
                                local: LocalTransform::identity(),
                                door: None,
                                cover: CoverTier::None,
                                source: PlacementSource::PrefabCoords,
                                parent: Some(slug.clone()),
                            });
                            canopy = true;
                            canopy_hull += 1;
                            reason = None;
                            notes.push(format!(
                                "{slug}/canopy: convex hull of the visual LOD0 ({} verts) tagged Foliage — the COLL carries no Foliage triangle",
                                vis.verts.len()
                            ));
                        }
                        None => notes.push(format!(
                            "{slug}: visual LOD0 hull degenerate — no canopy (COLL carries no Foliage triangle)"
                        )),
                    },
                    Err(e) => notes.push(format!(
                        "{slug}: visual LOD0 unreadable for the canopy hull ({e:#}) — no canopy"
                    )),
                }
            }
        }
        let blocks = !instances.is_empty();
        // Placed bounds, object frame.
        let mut bounds: Option<Bounds3> = None;
        for inst in &instances {
            let b = if inst.kind == InstanceKind::TreeCanopy {
                canopy_bounds
            } else {
                inst.xob
                    .as_deref()
                    .and_then(|x| walker.assets.load(x).ok())
                    .and_then(|a| a.bounds())
                    .map(|(min, max)| Bounds3 { min, max })
            };
            if let Some(b) = b {
                let (min, max) = inst.local.rigid().aabb_of(b.min, b.max);
                let placed = Bounds3 { min, max };
                bounds = Some(bounds.map_or(placed, |u| u.union(placed)));
            }
        }
        // Sidecar bytes for every referenced BLAS (canopies were inserted above).
        for inst in &instances {
            if blas.contains_key(&inst.blas) {
                continue;
            }
            if let Some(a) = inst.xob.as_deref().and_then(|x| walker.assets.load(x).ok())
                && let Some(bytes) = &a.sidecar_bytes
            {
                blas.insert(inst.blas.clone(), bytes.clone());
            }
        }
        descriptors.push(PrefabDescriptor {
            schema_version: DESCRIPTOR_SCHEMA_VERSION.into(),
            prefab_id: row.pid,
            slug,
            resource_name: path,
            kind: row.kind.clone(),
            blocks,
            reason: if blocks { None } else { reason },
            canopy,
            local_bounds: bounds,
            shell_bvh,
            instances,
            notes,
        });
    }
    let manifest = assemble_manifest(
        &opts.terrain,
        &descriptors,
        &blas,
        census,
        opts.hot,
        canopy_hull,
    )?;
    // The layer-policy census: per preset, triangles kept / dropped and the meshes
    // the policy emptied (their prefabs carry `blocks: false`, reason `no-fire-geo`).
    {
        let mut by_preset: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
        let mut emptied = 0usize;
        let mut with_drops = 0usize;
        for a in walker.assets.loaded() {
            let mut dropped_here = false;
            for (preset, kept, dropped) in &a.layer_census {
                let e = by_preset.entry(preset.clone()).or_default();
                e.0 += kept;
                e.1 += dropped;
                if *dropped > 0 {
                    e.2 += 1;
                    dropped_here = true;
                }
            }
            if dropped_here {
                with_drops += 1;
            }
            if a.coll.as_ref().is_some_and(|m| !m.tris.is_empty()) && a.kept_tris() == 0 {
                emptied += 1;
            }
        }
        eprintln!(
            "layer policy {:?}: {} meshes lost records, {} emptied",
            walker.assets.policy(),
            with_drops,
            emptied
        );
        for (preset, (kept, dropped, meshes)) in &by_preset {
            eprintln!(
                "  {preset:<18} kept {kept:>8} tris · dropped {dropped:>8} tris · in {meshes} meshes"
            );
        }
    }
    Ok(Library {
        descriptors,
        blas,
        manifest,
    })
}
