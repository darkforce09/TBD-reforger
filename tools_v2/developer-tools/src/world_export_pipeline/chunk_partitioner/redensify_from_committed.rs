use super::*;

/// T-176 A2 — re-derive the TBDD density grids from the **committed** objects (no staging /
/// Workbench). Reads `objects/prefabs.json.gz` (prefabId→kind) + every `objects/chunks/*.json.gz`
/// (`instances:[[prefabId,x,y,z,yaw],…]`), accumulates a global corner grid at `DENSITY_CELL_M`,
/// box-blurs the tree channel into a smooth canopy field (bridges tree gaps, leaves clearings as
/// holes), slices per chunk, and overwrites `objects/density/*.bin`. Rocks stay raw counts (unused
/// by the forest mass; kept for channel parity). Byte-consistent with the staging path
/// (`build_world_objects` shares `box_blur_corners`) and the D2 gate recompute.
pub fn redensify_from_committed(terrain: &str) -> Result<()> {
    let t = terrain_row(terrain)?;
    let b = t["worldBoundsM"].as_array().cloned().unwrap_or_default();
    if b.len() < 4 || b[0].as_f64() != Some(0.0) || b[1].as_f64() != Some(0.0) {
        bail!("worldBoundsM unsupported (expect square, origin 0)");
    }
    let world_size_m = b[2].as_f64().unwrap_or(0.0);
    if world_size_m <= 0.0 || b[2].as_f64() != b[3].as_f64() {
        bail!("worldBoundsM unsupported (expect square)");
    }

    let terrain_dir = repo_root().join("packages/map-assets").join(terrain);
    let objects_dir = terrain_dir.join("objects");
    let chunks_dir = objects_dir.join("chunks");
    let density_dir = objects_dir.join("density");

    // prefabId → kind (from the committed catalog).
    let prefabs_doc: Value = serde_json::from_slice(&gunzip(&std::fs::read(
        objects_dir.join("prefabs.json.gz"),
    )?)?)?;
    let prefabs = prefabs_doc["prefabs"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("prefabs.json.gz: no prefabs array"))?;
    let mut kind_by_id: HashMap<u64, String> = HashMap::new();
    for p in prefabs {
        if let (Some(id), Some(kind)) = (p["prefabId"].as_u64(), p["kind"].as_str()) {
            kind_by_id.insert(id, kind.to_string());
        }
    }

    // Tree + rock positions from every committed chunk.
    let mut trees: Vec<(f64, f64)> = Vec::new();
    let mut rocks: Vec<(f64, f64)> = Vec::new();
    let mut chunk_files: Vec<PathBuf> = std::fs::read_dir(&chunks_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "gz"))
        .collect();
    chunk_files.sort();
    for path in &chunk_files {
        let doc: Value = serde_json::from_slice(&gunzip(&std::fs::read(path)?)?)?;
        let Some(insts) = doc["instances"].as_array() else {
            continue;
        };
        for inst in insts {
            let Some(a) = inst.as_array() else { continue };
            if a.len() < 3 {
                continue;
            }
            let Some(id) = a[0].as_u64() else { continue };
            let (Some(x), Some(y)) = (a[1].as_f64(), a[2].as_f64()) else {
                continue;
            };
            if !x.is_finite() || !y.is_finite() {
                continue;
            }
            match kind_by_id.get(&id).map(String::as_str) {
                Some("tree") => trees.push((x, y)),
                Some("rock") => rocks.push((x, y)),
                _ => {}
            }
        }
    }

    let (tree_grid, tree_size) = density::accumulate_corners(trees.iter().copied(), world_size_m);
    let (rock_grid, rock_size) = density::accumulate_corners(rocks.iter().copied(), world_size_m);
    // T-537: refuse redensifying committed density bins from an empty tree+rock set.
    super::super::refuse_empty_write(
        "redensify density bins",
        trees.is_empty() && rocks.is_empty(),
        "zero trees and rocks in committed chunks — refusing empty density overwrite",
    )?;
    let tree_canopy =
        density::box_blur_corners(&tree_grid, tree_size, density::CANOPY_KERNEL_RADIUS_CELLS);

    let grid_cells = (world_size_m / CHUNK_SIZE_M).round() as usize;
    let _ = clear_density_dir_if_rebuilding(&density_dir, true)?;
    std::fs::create_dir_all(&density_dir)?;
    let mut density_bytes = 0u64;
    for cy in 0..grid_cells {
        for cx in 0..grid_cells {
            let tree_ch = density::slice_chunk_corners(&tree_canopy, tree_size, cx, cy);
            let rock_ch = density::slice_chunk_corners(&rock_grid, rock_size, cx, cy);
            let buf = website_map_engine::io::density::tbdd::encode_tbdd(
                density::DENSITY_CELL_M,
                density::DENSITY_COLS,
                density::DENSITY_ROWS,
                &[&tree_ch, &rock_ch],
            );
            density_bytes += buf.len() as u64;
            std::fs::write(
                density_dir.join(format!("{}.bin", chunk_key(cx as i64, cy as i64))),
                buf,
            )?;
        }
    }

    eprintln!(
        "[redensify] {terrain}: {} trees, {} rocks -> {grid_cells}x{grid_cells} = {} bins, {density_bytes} B; cell {} m, blur r={} (canopy)",
        trees.len(),
        rocks.len(),
        grid_cells * grid_cells,
        density::DENSITY_CELL_M,
        density::CANOPY_KERNEL_RADIUS_CELLS,
    );
    Ok(())
}

/// T-176 A2 — regenerate the golden S13 density fixture (`density-fixture.bin` + `expectedCorners` +
/// `expectedFileBytes` in `density-fixture.json`) from its own `treePositions`/`rockPositions` at the
/// current `DENSITY_CELL_M`. The S13 gate encodes the same slice and checks every corner, so after a
/// cell-size change the committed fixture must be regenerated. No canopy blur — this validates the
/// codec/accumulate pipeline, not the mass.
pub fn gen_density_fixture() -> Result<()> {
    let dir = repo_root().join("packages/tbd-schema/golden/map-objects/density");
    let json_path = dir.join("density-fixture.json");
    let mut fx: Value = serde_json::from_str(&std::fs::read_to_string(&json_path)?)?;
    let world = fx["worldSizeM"].as_f64().unwrap_or(0.0);
    let ccx = fx["chunk"]["cx"].as_u64().unwrap_or(0) as usize;
    let ccy = fx["chunk"]["cy"].as_u64().unwrap_or(0) as usize;
    let pos = |key: &str| -> Vec<(f64, f64)> {
        fx[key]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|r| Some((r["x"].as_f64()?, r["y"].as_f64()?)))
                    .collect()
            })
            .unwrap_or_default()
    };
    let trees = pos("treePositions");
    let rocks = pos("rockPositions");
    // T-537: refuse regenerating the golden density fixture from an empty position set.
    super::super::refuse_empty_write(
        "gen-density-fixture positions",
        trees.is_empty() && rocks.is_empty(),
        "no treePositions/rockPositions — refusing empty overwrite of density-fixture.bin",
    )?;
    let (t_grid, t_size) = density::accumulate_corners(trees.into_iter(), world);
    let (r_grid, r_size) = density::accumulate_corners(rocks.into_iter(), world);
    let t_slice = density::slice_chunk_corners(&t_grid, t_size, ccx, ccy);
    let r_slice = density::slice_chunk_corners(&r_grid, r_size, ccx, ccy);
    let buf = website_map_engine::io::density::tbdd::encode_tbdd(
        density::DENSITY_CELL_M,
        density::DENSITY_COLS,
        density::DENSITY_ROWS,
        &[&t_slice, &r_slice],
    );

    let cols = density::DENSITY_COLS as usize;
    let rows = density::DENSITY_ROWS as usize;
    let mut corners: Vec<Value> = Vec::new();
    for j in 0..rows {
        for i in 0..cols {
            let tree = t_slice[j * cols + i];
            let rock = r_slice[j * cols + i];
            if tree != 0 || rock != 0 {
                corners.push(json!({ "i": i, "j": j, "tree": tree, "rock": rock }));
            }
        }
    }
    super::super::refuse_empty_write(
        "gen-density-fixture corners",
        corners.is_empty() || buf.is_empty(),
        "zero nonzero corners / empty TBDD buffer — refusing empty overwrite of density-fixture.bin",
    )?;
    std::fs::write(dir.join("density-fixture.bin"), &buf)?;

    let half = density::DENSITY_CELL_M / 2;
    fx["description"] = json!(format!(
        "S13 synthetic TBDD fixture — encode(fixture) must equal density-fixture.bin byte-for-byte; decode(bin) must match expectedCorners. Corner (i,j) of chunk (cx,cy) counts instances in [X-{half},X+{half}) x [Y-{half},Y+{half}), X = cx*512+i*{}.",
        density::DENSITY_CELL_M
    ));
    let n_corners = corners.len();
    fx["expectedCorners"] = Value::Array(corners);
    fx["expectedFileBytes"] = json!(buf.len());
    std::fs::write(&json_path, serde_json::to_string_pretty(&fx)? + "\n")?;
    eprintln!(
        "[gen-density-fixture] {} bytes, {n_corners} nonzero corners, cell {} m",
        buf.len(),
        density::DENSITY_CELL_M
    );
    Ok(())
}

/// build-roads-from-topo.mjs port. Determinism: records sorted by (type, first x, first y,
/// vertexCount); ids assigned after the sort; points rounded to 2 dp; gzip level 9.
/// The road census, read from the COMMITTED `roads.json.gz` — T-946/T-960.
///
/// Roads never pass through prefab classification: they export as prefab-less `RoadEntity` rows
/// (`resourceName` empty), so `classify.rs` — a pure function of the resource name — never sees
/// one. That is why appending classification rules could not make the census non-zero, and why
/// both slots were simply hardcoded: `byKind.road.segments` to 0 and `byRoadClass` to `{}`. The
/// artifact therefore published "0 road segments" while `roads.json.gz` beside it shipped 887.
///
/// `roads.json.gz` is committed and repo-reproducible, so the census is derivable here with no
/// Workbench staging export — which also lets `world reclassify` correct the shipped artifact.
/// Returns `None` when the file is absent or unreadable: a missing census stays absent rather
/// than being asserted as zero.
#[must_use]
pub fn road_census(objects_dir: &Path) -> Option<(u64, Map<String, Value>)> {
    let raw = std::fs::read(objects_dir.join("roads.json.gz")).ok()?;
    let doc: Value = serde_json::from_slice(&gunzip(&raw).ok()?).ok()?;
    let segs = doc["roadSegments"].as_array()?;
    let mut by_class: Map<String, Value> = Map::new();
    for r in segs {
        let Some(c) = r["roadClass"].as_str() else {
            continue;
        };
        let n = by_class.get(c).and_then(Value::as_u64).unwrap_or(0);
        by_class.insert(c.to_string(), json!(n + 1));
    }
    let mut keys: Vec<String> = by_class.keys().cloned().collect();
    keys.sort();
    // The schema's `classBucket` shape: a road has no prefab type, and a segment IS the instance.
    let sorted: Map<String, Value> = keys
        .into_iter()
        .map(|k| {
            let n = by_class[&k].as_u64().unwrap_or(0);
            (k, json!({ "prefabTypes": 0, "instances": n }))
        })
        .collect();
    Some((segs.len() as u64, sorted))
}

pub fn build_roads_from_topo(
    terrain: &str,
    out_base: Option<&Path>,
    ops_log: bool,
) -> Result<Value> {
    build_roads_from_topo_opt(terrain, out_base, ops_log, false)
}

pub fn build_roads_from_topo_opt(
    terrain: &str,
    out_base: Option<&Path>,
    ops_log: bool,
    quiet: bool,
) -> Result<Value> {
    let vfs = PakVfs::open_default()?;
    let topo = decode_topo(&vfs, terrain)?;
    let road_class = |ty: u8| -> Option<&'static str> {
        match ty {
            TOPO_AIRFIELD => Some("runway"),
            TOPO_RIVER => Some("highway_paved"),
            TOPO_STREAM => Some("road_paved"),
            TOPO_ROAD_A => Some("road_dirt"),
            TOPO_ROAD_B => Some("track"),
            _ => None,
        }
    };
    struct Rec {
        ty: u8,
        points: Vec<(f64, f64)>,
    }
    let mut records: Vec<Rec> = topo
        .records
        .iter()
        .filter(|r| road_class(r.rec_type).is_some())
        .map(|r| {
            let mut points = Vec::with_capacity(r.verts.len() / 2);
            for i in (0..r.verts.len()).step_by(2) {
                points.push((
                    round2(f64::from(r.verts[i])),
                    round2(topo.world_size_m - f64::from(r.verts[i + 1])),
                ));
            }
            Rec {
                ty: r.rec_type,
                points,
            }
        })
        .collect();
    records.sort_by(|a, b| {
        a.ty.cmp(&b.ty)
            .then(a.points[0].0.partial_cmp(&b.points[0].0).unwrap())
            .then(a.points[0].1.partial_cmp(&b.points[0].1).unwrap())
            .then(a.points.len().cmp(&b.points.len()))
    });
    let segments: Vec<Value> = records
        .iter()
        .enumerate()
        .map(|(i, r)| {
            json!({
                "id": format!("road-{terrain}-{i:04}"),
                "roadClass": road_class(r.ty).unwrap(),
                "points": r.points.iter().map(|(x, y)| Value::Array(vec![js_num(*x), js_num(*y)])).collect::<Vec<_>>(),
            })
        })
        .collect();
    let doc = json!({ "schemaVersion": "1.0.0", "terrainId": terrain, "roadSegments": segments });
    // T-537: refuse writing empty roads.json.gz over the committed 887-segment catalog.
    super::super::refuse_empty_write(
        "build-roads-from-topo",
        segments.is_empty(),
        "zero road segments — refusing empty roads.json.gz overwrite",
    )?;
    let out_base: PathBuf = out_base
        .map(Path::to_path_buf)
        .unwrap_or_else(|| repo_root().join("packages/map-assets").join(terrain));
    let objects_dir = out_base.join("objects");
    std::fs::create_dir_all(&objects_dir)?;
    std::fs::write(
        objects_dir.join("roads.json.gz"),
        gz9(compact(&doc).as_bytes())?,
    )?;

    let mut by_class: Map<String, Value> = Map::new();
    for r in &records {
        let c = road_class(r.ty).unwrap();
        let n = by_class.get(c).and_then(Value::as_u64).unwrap_or(0);
        by_class.insert(c.to_string(), json!(n + 1));
    }
    let summary = json!({
        "slice": "T-090.3.3",
        "source": "decode-topo section 1",
        "classMappingProvisional": false,
        "classByTopoType": { "0": "runway", "1": "highway_paved", "2": "road_paved", "3": "road_dirt", "5": "track" },
        "segments": records.len(),
        "byClass": by_class,
        "points": records.iter().map(|r| r.points.len()).sum::<usize>(),
    });
    if ops_log {
        let ops_path = repo_root()
            .join(".ai/artifacts")
            .join(format!("map_export_{terrain}.json"));
        let mut ops: Value = if ops_path.exists() {
            serde_json::from_str(&std::fs::read_to_string(&ops_path)?)?
        } else {
            json!({ "terrainId": terrain })
        };
        if !ops["fullExport"].is_object() {
            ops["fullExport"] = json!({});
        }
        ops["fullExport"]["roads"] = summary.clone();
        std::fs::write(&ops_path, pretty_nl(&ops))?;
    }
    if !quiet {
        println!("build-roads-from-topo: {terrain} — {}", compact(&summary));
    }
    Ok(summary)
}
