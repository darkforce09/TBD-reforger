use super::*;

pub(super) fn tempdir(prefix: &str) -> Result<PathBuf> {
    // mkdtemp equivalent without a dep: pid+counter suffix under the system tmpdir.
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let d = std::env::temp_dir().join(format!(
        "{prefix}{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&d)?;
    Ok(d)
}

pub(super) fn list_files(dir: &Path) -> Result<Vec<String>> {
    fn walk(dir: &Path, base: &Path, acc: &mut Vec<String>) -> Result<()> {
        for e in std::fs::read_dir(dir)? {
            let e = e?;
            let p = e.path();
            if p.is_dir() {
                walk(&p, base, acc)?;
            } else {
                acc.push(p.strip_prefix(base).unwrap().to_string_lossy().into_owned());
            }
        }
        Ok(())
    }
    let mut acc = Vec::new();
    walk(dir, dir, &mut acc)?;
    acc.sort();
    Ok(acc)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_p1_gates(
    g: &mut Gates,
    prefabs: &[Value],
    inventory: &Value,
    manifest: &Value,
    anchor_pool: &[Value],
    rows_by_key: &[(String, Vec<Value>)],
    world_size_m: f64,
) {
    let buildings: Vec<&Value> = prefabs.iter().filter(|p| p["kind"] == "building").collect();
    {
        let allowed: HashSet<&str> = phase_kinds(
            manifest["objects"]["importPhaseMax"]
                .as_str()
                .unwrap_or("P1_buildings"),
        )
        .unwrap_or(&["building"])
        .iter()
        .copied()
        .collect();
        let mut errs: Vec<String> = prefabs
            .iter()
            .filter(|p| !allowed.contains(p["kind"].as_str().unwrap_or("")))
            .map(|p| {
                format!(
                    "prefab {} kind={} outside importPhaseMax kinds",
                    p["prefabId"],
                    p["kind"].as_str().unwrap_or("")
                )
            })
            .collect();
        if buildings.is_empty() {
            errs.push("no kind=building prefabs in catalog".into());
        }
        g.gate(
            "P1-1",
            "building prefabs present; catalog kinds within committed importPhaseMax",
            errs,
        );
    }
    {
        let exempt = |p: &Value| {
            p["tags"]
                .as_array()
                .is_some_and(|t| t.iter().any(|x| x == "ruin-open"))
                || p["class"] == "tent"
        };
        let hard = buildings
            .iter()
            .filter(|p| p["gameplay"]["cover"]["type"] == "hard" || exempt(p))
            .count();
        let pct = if buildings.is_empty() {
            1.0
        } else {
            hard as f64 / buildings.len() as f64
        };
        let errs = if pct >= 0.995 {
            vec![]
        } else {
            vec![format!(
                "only {:.2}% hard: {}",
                pct * 100.0,
                buildings
                    .iter()
                    .filter(|p| p["gameplay"]["cover"]["type"] != "hard" && !exempt(p))
                    .map(|p| p["resourceName"].as_str().unwrap_or("").to_string())
                    .take(5)
                    .collect::<Vec<_>>()
                    .join(", ")
            )]
        };
        g.gate(
            "P1-2",
            "cover=hard >= 99.5% (ruin-open + tent exceptions allowed)",
            errs,
        );
    }
    g.gate(
        "P1-3",
        "footprint or OBB volume > 0 per building prefab",
        buildings
            .iter()
            .filter(|p| {
                let fp = p["spatial"]["footprintM2"].as_f64().unwrap_or(0.0);
                let he = &p["spatial"]["halfExtentsM"];
                let vol = he["x"].as_f64().unwrap_or(0.0)
                    * he["y"].as_f64().unwrap_or(0.0)
                    * he["z"].as_f64().unwrap_or(0.0);
                !(fp > 0.0 || vol > 0.0)
            })
            .map(|p| {
                format!(
                    "prefab {} {}",
                    p["prefabId"],
                    p["resourceName"].as_str().unwrap_or("")
                )
            })
            .collect(),
    );
    {
        // P1-4 — K=32 deterministic anchors (sort, even spacing, min/max x, boundary rows).
        const K: usize = 32;
        let mut pool = anchor_pool.to_vec();
        pool.sort_by(|a, b| {
            a["resourceName"]
                .as_str()
                .cmp(&b["resourceName"].as_str())
                .then(
                    a["x"]
                        .as_f64()
                        .partial_cmp(&b["x"].as_f64())
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
                .then(
                    a["z"]
                        .as_f64()
                        .partial_cmp(&b["z"].as_f64())
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });
        let errs = if pool.is_empty() {
            vec!["no building rows in staged raw".to_string()]
        } else {
            let mut picks: HashSet<usize> = HashSet::new();
            for i in 0..K {
                picks.insert(
                    ((i as f64 * (pool.len() - 1) as f64) / (K - 1) as f64).round() as usize,
                );
            }
            let mut by_x: Vec<usize> = (0..pool.len()).collect();
            by_x.sort_by(|&a, &b| {
                pool[a]["x"]
                    .as_f64()
                    .partial_cmp(&pool[b]["x"].as_f64())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            picks.insert(by_x[0]);
            picks.insert(*by_x.last().unwrap());
            let mut boundary = 0;
            for (i, r) in pool.iter().enumerate() {
                if boundary >= 4 {
                    break;
                }
                let rx = round2(r["x"].as_f64().unwrap_or(0.0));
                let rz = round2(r["z"].as_f64().unwrap_or(0.0));
                if rx % CHUNK_SIZE_M < 1.0 || rz % CHUNK_SIZE_M < 1.0 {
                    picks.insert(i);
                    boundary += 1;
                }
            }
            let mut idx: Vec<usize> = picks.into_iter().collect();
            idx.sort_unstable();
            let anchors: Vec<Value> = idx.into_iter().map(|i| pool[i].clone()).collect();
            let by_key: HashMap<String, &Vec<Value>> =
                rows_by_key.iter().map(|(k, r)| (k.clone(), r)).collect();
            check_anchors(
                &anchors,
                prefabs,
                |cx, cy| {
                    by_key
                        .get(&geometry::chunk_key(cx, cy))
                        .map(|rows| json!({ "instances": rows }))
                },
                CHUNK_SIZE_M,
                world_size_m,
                2.0,
            )
        };
        g.gate(
            "P1-4",
            &format!(
                "K=32 anchor sample <= 2 m via committed chunks ({} anchors)",
                std::cmp::min(32 + 6, anchor_pool.len())
            ),
            errs,
        );
    }
    {
        let mut errs = Vec::new();
        let classes = inventory["byBuildingClass"]
            .as_object()
            .map(|m| m.len())
            .unwrap_or(0);
        if classes == 0 {
            errs.push("byBuildingClass empty".into());
        }
        let unknown = inventory["byBuildingClass"]["unknown"]["instances"]
            .as_f64()
            .unwrap_or(0.0);
        let raw_total = inventory["byKind"]["building"]["instances"]
            .as_f64()
            .unwrap_or(0.0);
        let total = if raw_total == 0.0 { 1.0 } else { raw_total };
        if unknown / total >= 0.005 {
            errs.push(format!("byBuildingClass.unknown {unknown}/{total} >= 0.5%"));
        }
        g.gate(
            "P1-6",
            "byBuildingClass populated; unknown < 0.5% of building instances",
            errs,
        );
    }
}
