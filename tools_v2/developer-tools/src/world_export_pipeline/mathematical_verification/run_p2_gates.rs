use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn run_p2_gates(
    g: &mut Gates,
    prefabs: &[Value],
    inventory: &Value,
    rows_by_key: &[(String, Vec<Value>)],
    raw_kind_counts: &HashMap<String, u64>,
    committed_kind_counts: &HashMap<String, u64>,
    raw_rock_rows: &[(f64, f64)],
    objects_dir: &Path,
    world_size_m: f64,
    terrain: &str,
    v_region: &jsonschema::Validator,
) -> Result<()> {
    let phase_kind_set: HashSet<&str> = ["building", "tree", "water"].into_iter().collect();
    let tree_prefabs: Vec<&Value> = prefabs.iter().filter(|p| p["kind"] == "tree").collect();
    let mut tree_rows: Vec<(f64, f64, String)> = Vec::new();
    for (_, rows) in rows_by_key {
        for row in rows {
            if let Some(p) = row[0].as_u64().and_then(|i| prefabs.get(i as usize))
                && p["kind"] == "tree"
            {
                tree_rows.push((
                    row[1].as_f64().unwrap_or(0.0),
                    row[2].as_f64().unwrap_or(0.0),
                    p["class"].as_str().unwrap_or("").to_string(),
                ));
            }
        }
    }

    {
        let mut errs: Vec<String> = prefabs
            .iter()
            .filter(|p| !phase_kind_set.contains(p["kind"].as_str().unwrap_or("")))
            .map(|p| {
                format!(
                    "prefab {} kind={}",
                    p["prefabId"],
                    p["kind"].as_str().unwrap_or("")
                )
            })
            .collect();
        if tree_prefabs.is_empty() {
            errs.push("no kind=tree prefabs in catalog".into());
        }
        if committed_kind_counts.get("building").copied().unwrap_or(0) == 0 {
            errs.push("cumulative rule broken: 0 building instances in P2 catalog".into());
        }
        g.gate(
            "PH-P2-1",
            "cumulative P1+P2 catalog; kinds subset {building, tree}; trees present",
            errs,
        );
    }
    g.gate(
        "PH-P2-2",
        "tree prefabs cover=soft (dead exception)",
        tree_prefabs
            .iter()
            .filter(|p| p["gameplay"]["cover"]["type"] != "soft" && p["class"] != "dead")
            .map(|p| {
                format!(
                    "prefab {} {} cover={}",
                    p["prefabId"],
                    p["resourceName"].as_str().unwrap_or(""),
                    p["gameplay"]["cover"]["type"]
                )
            })
            .collect(),
    );
    {
        let tall = tree_prefabs
            .iter()
            .filter(|p| p["spatial"]["heightM"].as_f64().unwrap_or(0.0) >= 2.0)
            .count();
        let pct = if tree_prefabs.is_empty() {
            0.0
        } else {
            tall as f64 / tree_prefabs.len() as f64
        };
        g.gate(
            "PH-P2-3",
            "heightM >= 2 for >= 95% of tree prefabs",
            if pct >= 0.95 {
                vec![]
            } else {
                vec![format!("only {:.2}% >= 2 m", pct * 100.0)]
            },
        );
    }
    {
        let raw_tree = raw_kind_counts.get("tree").copied().unwrap_or(0);
        let committed_tree = committed_kind_counts.get("tree").copied().unwrap_or(0);
        g.gate(
            "PH-P2-4",
            "G11 count conservation for kind=tree only",
            if raw_tree == committed_tree {
                vec![]
            } else {
                vec![format!(
                    "raw tree count {raw_tree} != committed tree instances {committed_tree}"
                )]
            },
        );
    }

    let (tree_grid, tree_size) =
        density::accumulate_corners(tree_rows.iter().map(|(x, y, _)| (*x, *y)), world_size_m);
    let (rock_grid, rock_size) =
        density::accumulate_corners(raw_rock_rows.iter().copied(), world_size_m);
    // T-176 A2 — the committed tree channel is canopy-blurred (density.rs / build.rs); recompute the
    // same global blur so D2 stays byte-exact. PH-P2-5 below still asserts on the RAW `tree_grid`.
    let tree_canopy =
        density::box_blur_corners(&tree_grid, tree_size, density::CANOPY_KERNEL_RADIUS_CELLS);

    {
        let sum: u64 = tree_grid.iter().map(|&v| u64::from(v)).sum();
        let committed_tree = committed_kind_counts.get("tree").copied().unwrap_or(0);
        g.gate(
            "PH-P2-5",
            "density insert identity (sum of global tree corners = tree instances)",
            if sum == committed_tree {
                vec![]
            } else {
                vec![format!(
                    "corner sum {sum} != tree instances {committed_tree}"
                )]
            },
        );
    }

    {
        let mut d1 = Vec::new();
        let mut d2 = Vec::new();
        let density_dir = objects_dir.join("density");
        let grid_cells = (world_size_m / CHUNK_SIZE_M).round() as usize;
        let mut expected: HashSet<String> = HashSet::new();
        for cy in 0..grid_cells {
            for cx in 0..grid_cells {
                expected.insert(chunk_key(cx as i64, cy as i64));
            }
        }
        if density_dir.exists() {
            for f in std::fs::read_dir(&density_dir)? {
                let name = f?.file_name().into_string().unwrap_or_default();
                if name.ends_with(".bin") && !expected.contains(name.trim_end_matches(".bin")) {
                    d1.push(format!("unexpected density file {name}"));
                }
            }
        }
        let mut keys: Vec<&String> = expected.iter().collect();
        keys.sort();
        for key in keys {
            let p = density_dir.join(format!("{key}.bin"));
            if !p.exists() {
                d1.push(format!("missing density file {key}.bin"));
                continue;
            }
            let buf = std::fs::read(&p)?;
            if buf.len() != density::TBDD_FILE_BYTES {
                d1.push(format!(
                    "{key}.bin: {} bytes, want {}",
                    buf.len(),
                    density::TBDD_FILE_BYTES
                ));
                continue;
            }
            let dec = match website_map_engine::io::density::tbdd::decode_tbdd(&buf) {
                Ok(d) => d,
                Err(e) => {
                    d1.push(format!("{key}.bin: {e}"));
                    continue;
                }
            };
            if dec.version != density::TBDD_VERSION
                || dec.cell_m != density::DENSITY_CELL_M
                || dec.cols != density::DENSITY_COLS
                || dec.rows != density::DENSITY_ROWS
                || dec.channels.len() != density::DENSITY_CHANNELS.len()
            {
                d1.push(format!("{key}.bin: header mismatch"));
                continue;
            }
            let mut it = key.split('_').map(|v| v.parse::<usize>().unwrap_or(0));
            let (cx, cy) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
            let rebuilt = website_map_engine::io::density::tbdd::encode_tbdd(
                density::DENSITY_CELL_M,
                density::DENSITY_COLS,
                density::DENSITY_ROWS,
                &[
                    &density::slice_chunk_corners(&tree_canopy, tree_size, cx, cy),
                    &density::slice_chunk_corners(&rock_grid, rock_size, cx, cy),
                ],
            );
            if buf != rebuilt {
                d2.push(format!(
                    "{key}.bin differs from recompute (committed chunks + raw rocks)"
                ));
            }
        }
        g.gate(
            "D1",
            &format!(
                "density files complete ({} cells), TBDD header + size exact",
                expected.len()
            ),
            d1,
        );
        g.gate(
            "D2",
            "density byte-identical to recompute from committed chunks + staged raw rocks",
            d2,
        );
    }

    {
        let regions_path = objects_dir.join("forest-regions.json.gz");
        if !regions_path.exists() {
            g.gate(
                "F1",
                "forest regions present + rows schema-valid",
                vec!["objects/forest-regions.json.gz missing".into()],
            );
        } else {
            let doc = gunzip_json(&regions_path)?;
            let regions = doc["regions"].as_array().cloned().unwrap_or_default();
            let mut f1 = Vec::new();
            for (i, r) in regions.iter().enumerate() {
                if !v_region.is_valid(r) {
                    f1.push(format!("region[{i}] {}: invalid", r["id"]));
                }
            }
            g.gate(
                "F1",
                &format!(
                    "forest regions present + {} rows schema-valid",
                    regions.len()
                ),
                f1,
            );

            let mut f2 = Vec::new();
            let region_tree_sum: u64 = regions
                .iter()
                .map(|r| r["treeCount"].as_u64().unwrap_or(0))
                .sum();
            let inv_tree = inventory["byKind"]["tree"]["instances"]
                .as_u64()
                .unwrap_or(0);
            let inv_region = &inventory["byRegionKind"]["forest"];
            if inv_region.is_null() {
                f2.push("inventory.byRegionKind.forest missing".into());
            } else {
                if inv_region["treeCount"].as_u64() != Some(region_tree_sum) {
                    f2.push(format!(
                        "inventory forest.treeCount {} != regions file sum {region_tree_sum}",
                        inv_region["treeCount"]
                    ));
                }
                if inv_region["count"].as_u64() != Some(regions.len() as u64) {
                    f2.push(format!(
                        "inventory forest.count {} != regions {}",
                        inv_region["count"],
                        regions.len()
                    ));
                }
            }
            let unassigned = inventory["unassignedTrees"].as_u64().unwrap_or(0);
            if region_tree_sum + unassigned != inv_tree {
                f2.push(format!("F2 identity broken: {region_tree_sum} + {unassigned} != byKind.tree.instances {inv_tree}"));
            }
            if inv_tree != committed_kind_counts.get("tree").copied().unwrap_or(0) {
                f2.push(format!(
                    "inventory tree instances {inv_tree} != committed tree rows {}",
                    committed_kind_counts.get("tree").copied().unwrap_or(0)
                ));
            }
            g.gate(
                "F2",
                "forest.treeCount + unassignedTrees = byKind.tree.instances (exact)",
                f2,
            );

            let mut f6 = Vec::new();
            let trees: Vec<Tree> = tree_rows
                .iter()
                .map(|(x, y, class)| Tree {
                    x: *x,
                    y: *y,
                    class: class.clone(),
                })
                .collect();
            let redo = derive_forest_regions(&trees, world_size_m, terrain);
            if Value::Array(redo.regions.clone()) != Value::Array(regions.clone()) {
                f6.push("re-derived regions differ from committed rings/aggregates".into());
            }
            if redo.unassigned_trees != inventory["unassignedTrees"].as_u64().unwrap_or(u64::MAX) {
                f6.push(format!(
                    "re-derived unassignedTrees {} != inventory {}",
                    redo.unassigned_trees, inventory["unassignedTrees"]
                ));
            }
            let _ = forest::REGION_CELL_M;
            g.gate(
                "F6",
                "Path B derivation reproducible from committed chunk tree instances",
                f6,
            );
        }
    }
    Ok(())
}
