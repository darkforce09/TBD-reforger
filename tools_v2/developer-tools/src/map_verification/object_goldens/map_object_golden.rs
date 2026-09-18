use super::*;

pub fn map_object_golden(root: &Path) -> Result<u8> {
    let sroot = root.join("packages/tbd-schema");
    let mo = |parts: &[&str]| -> PathBuf {
        let mut p = sroot.join("golden/map-objects");
        for x in parts {
            p = p.join(x);
        }
        p
    };

    let enums = read_json(&sroot.join("schema/map-object-enums.schema.json"))?["$defs"].clone();
    let enum_vec = |name: &str| -> Vec<String> {
        enums[name]["enum"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let enum_set = |name: &str| -> HashSet<String> { enum_vec(name).into_iter().collect() };
    let class_enum_for_kind: BTreeMap<&str, &str> = BTreeMap::from([
        ("building", "buildingClass"),
        ("road", "roadClass"),
        ("tree", "speciesClass"),
        ("vegetation", "speciesClass"),
        ("rock", "rockClass"),
        ("prop", "propClass"),
        ("utility", "utilityClass"),
        ("water", "waterClass"),
        // T-244. Arms S3 (≥1 golden example for the kind) and, via the entry below, S9 (one
        // golden example per vehicleClass). Both are backed by real Everon wreck prefabs.
        ("vehicle", "vehicleClass"),
    ]);
    let expected_classes_for_kind: BTreeMap<&str, Vec<String>> = BTreeMap::from([
        (
            "tree",
            ["conifer", "deciduous", "palm", "dead", "unknown"]
                .map(String::from)
                .to_vec(),
        ),
        (
            "vegetation",
            ["bush", "grass", "fern", "dead", "unknown"]
                .map(String::from)
                .to_vec(),
        ),
        ("building", enum_vec("buildingClass")),
        ("road", enum_vec("roadClass")),
        ("rock", enum_vec("rockClass")),
        ("prop", enum_vec("propClass")),
        ("utility", enum_vec("utilityClass")),
        ("water", enum_vec("waterClass")),
        ("vehicle", enum_vec("vehicleClass")),
    ]);

    let prefabs_sample = read_json(&mo(&["map-object-prefabs-sample.json"]))?;
    let instances_sample = read_json(&mo(&["map-object-instances-sample.json"]))?;
    let regions_sample = read_json(&mo(&["map-object-regions-everon-sample.json"]))?;
    let roads_sample = read_json(&mo(&["map-object-roads-sample.json"]))?;
    let resolved_sample = read_json(&mo(&["map-object-resolved-sample.json"]))?;
    let chunk_sample = read_json(&mo(&["map-object-chunk-sample.json"]))?;
    let anchor_fixture = read_json(&mo(&["phased", "P1-anchor-fixture.json"]))?;
    let density_fixture = read_json(&mo(&["density", "density-fixture.json"]))?;
    let density_bin = fs::read(mo(&["density", "density-fixture.bin"]))?;
    let region_fixture = read_json(&mo(&["regions-derivation-fixture.json"]))?;
    let catalog_bundles = [
        (
            "map-object-catalog-everon-sample.json",
            read_json(&mo(&["map-object-catalog-everon-sample.json"]))?,
        ),
        (
            "phased/P1-buildings.json",
            read_json(&mo(&["phased", "P1-buildings.json"]))?,
        ),
        (
            "phased/P2-trees.json",
            read_json(&mo(&["phased", "P2-trees.json"]))?,
        ),
    ];

    struct Table {
        label: String,
        prefabs: Vec<Value>,
        instances: Vec<Value>,
        road_segments: Vec<Value>,
    }
    let arr = |v: &Value| v.as_array().cloned().unwrap_or_default();
    let mut tables = vec![
        Table {
            label: "prefabs-sample".into(),
            prefabs: arr(&prefabs_sample),
            instances: arr(&instances_sample),
            road_segments: arr(&roads_sample["roadSegments"]),
        },
        Table {
            label: "chunk-sample".into(),
            prefabs: arr(&prefabs_sample),
            instances: arr(&chunk_sample["chunk"]["instances"]),
            road_segments: vec![],
        },
    ];
    for (label, data) in &catalog_bundles {
        tables.push(Table {
            label: (*label).into(),
            prefabs: arr(&data["prefabs"]),
            instances: arr(&data["instances"]),
            road_segments: arr(&data["roadSegments"]),
        });
    }

    let mut gates: Vec<Gate> = Vec::new();

    // S2
    {
        let mut errs = Vec::new();
        for t in &tables {
            let by_id: HashMap<u64, &Value> = t
                .prefabs
                .iter()
                .filter_map(|p| Some((p["prefabId"].as_f64()?.to_bits(), p)))
                .collect();
            for p in &t.prefabs {
                if p["kind"].as_str().unwrap_or("").is_empty() {
                    errs.push(format!(
                        "{}: prefab {} missing kind",
                        t.label, p["prefabId"]
                    ));
                }
                if p["class"].as_str().unwrap_or("").is_empty() {
                    errs.push(format!(
                        "{}: prefab {} missing class",
                        t.label, p["prefabId"]
                    ));
                }
            }
            for row in &t.instances {
                if let Some(pid) = inst_prefab_id(row)
                    && let Some(p) = by_id.get(&pid.to_bits())
                    && (p["kind"].as_str().unwrap_or("").is_empty()
                        || p["class"].as_str().unwrap_or("").is_empty())
                {
                    errs.push(format!(
                        "{}: instance {} resolves to prefab without kind/class",
                        t.label,
                        inst_id(row)
                    ));
                }
            }
        }
        gates.push(Gate {
            id: "S2",
            label: "every prefab + instance row has resolvable kind + class",
            errs,
        });
    }

    // S3
    {
        let have: HashSet<&str> = arr(&prefabs_sample)
            .iter()
            .filter_map(|p| p["kind"].as_str())
            .map(|s| Box::leak(s.to_string().into_boxed_str()) as &str)
            .collect();
        let errs: Vec<String> = class_enum_for_kind
            .keys()
            .filter(|k| !have.contains(**k))
            .map(|k| format!("prefabs-sample: no prefab example for kind '{k}'"))
            .collect();
        gates.push(Gate {
            id: "S3",
            label: "≥1 prefab example per instance kind",
            errs,
        });
    }

    // S4
    {
        let mut errs = Vec::new();
        let road_enum = enum_set("roadClass");
        for t in &tables {
            for seg in &t.road_segments {
                let rc = seg["roadClass"].as_str().unwrap_or("");
                if !road_enum.contains(rc) {
                    errs.push(format!(
                        "{}: segment {} roadClass '{rc}' invalid",
                        t.label, seg["id"]
                    ));
                }
            }
            for p in t.prefabs.iter().filter(|p| p["kind"] == "road") {
                let c = p["class"].as_str().unwrap_or("");
                if !road_enum.contains(c) {
                    errs.push(format!(
                        "{}: road prefab {} class '{c}' not a roadClass",
                        t.label, p["prefabId"]
                    ));
                }
            }
        }
        gates.push(Gate {
            id: "S4",
            label: "road segments + road prefabs use valid roadClass",
            errs,
        });
    }

    // S5
    {
        let mut errs = Vec::new();
        for t in &tables {
            let mut seen_id = HashSet::new();
            let mut seen_res = HashSet::new();
            for p in &t.prefabs {
                let pid = p["prefabId"].as_f64().unwrap_or(f64::NAN).to_bits();
                let rn = p["resourceName"].as_str().unwrap_or("").to_string();
                if !seen_id.insert(pid) {
                    errs.push(format!("{}: duplicate prefabId {}", t.label, p["prefabId"]));
                }
                if !seen_res.insert(rn.clone()) {
                    errs.push(format!("{}: duplicate resourceName {rn}", t.label));
                }
            }
            for row in &t.instances {
                if row.is_array() {
                    continue;
                }
                for key in ["resourceName", "kind", "class", "bounds"] {
                    if row.get(key).is_some() {
                        errs.push(format!(
                            "{}: instance {} duplicates prefab field '{key}'",
                            t.label,
                            row["id"].as_str().unwrap_or("?")
                        ));
                    }
                }
            }
        }
        gates.push(Gate {
            id: "S5",
            label: "prefab dedup — unique prefabId/resourceName; instances carry no type fields",
            errs,
        });
    }

    // S6
    {
        let mut errs = Vec::new();
        for t in &tables {
            let ids: HashSet<u64> = t
                .prefabs
                .iter()
                .filter_map(|p| p["prefabId"].as_f64().map(f64::to_bits))
                .collect();
            for row in &t.instances {
                match inst_prefab_id(row) {
                    Some(pid) if ids.contains(&pid.to_bits()) => {}
                    Some(pid) => errs.push(format!(
                        "{}: instance {} prefabId {pid} does not resolve",
                        t.label,
                        inst_id(row)
                    )),
                    None => errs.push(format!(
                        "{}: instance {} prefabId missing",
                        t.label,
                        inst_id(row)
                    )),
                }
            }
        }
        gates.push(Gate {
            id: "S6",
            label: "every instance prefabId resolves in its own prefab table",
            errs,
        });
    }

    // S7
    {
        let mut errs = Vec::new();
        for t in &tables {
            for p in &t.prefabs {
                if p["ai"]["summary"].as_str().unwrap_or("").is_empty() {
                    errs.push(format!(
                        "{}: prefab {} missing ai.summary",
                        t.label, p["prefabId"]
                    ));
                }
                if p["ai"]["taxonomyPath"].as_str().unwrap_or("").is_empty() {
                    errs.push(format!(
                        "{}: prefab {} missing ai.taxonomyPath",
                        t.label, p["prefabId"]
                    ));
                }
                if p["gameplay"]["cover"].get("type").is_none() {
                    errs.push(format!(
                        "{}: prefab {} missing gameplay.cover.type",
                        t.label, p["prefabId"]
                    ));
                }
                if p["spatial"].get("heightM").is_none() {
                    errs.push(format!(
                        "{}: prefab {} missing spatial.heightM",
                        t.label, p["prefabId"]
                    ));
                }
            }
        }
        gates.push(Gate { id: "S7", label: "every prefab has ai.summary + ai.taxonomyPath + gameplay.cover.type + spatial.heightM", errs });
    }

    // S8 — resolved rows against the registered resolved schema.
    {
        let mut errs = Vec::new();
        let mut registered = Vec::new();
        for f in [
            "map-object-enums.schema.json",
            "map-object-prefab.schema.json",
            "map-object-resolved.schema.json",
        ] {
            let doc = read_json(&sroot.join("schema").join(f))?;
            let id = doc["$id"].as_str().unwrap_or_default().to_string();
            registered.push((id, doc));
        }
        let registry = jsonschema::Registry::new()
            .extend(registered.iter().map(|(id, doc)| {
                (
                    id.as_str(),
                    jsonschema::Resource::from_contents(doc.clone()),
                )
            }))
            .map_err(|e| anyhow::anyhow!("registry: {e}"))?
            .prepare()
            .map_err(|e| anyhow::anyhow!("registry: {e}"))?;
        let v_resolved = jsonschema::options()
            .with_registry(&registry)
            .build(&json!({"$ref": "https://schema.tbdevent.eu/map-object-resolved/v1.json"}))
            .map_err(|e| anyhow::anyhow!("compile: {e}"))?;
        for (i, row) in arr(&resolved_sample).iter().enumerate() {
            for e in v_resolved.iter_errors(row) {
                let p = e.instance_path().to_string();
                errs.push(format!(
                    "resolved[{i}] {}: {} {e}",
                    row["id"].as_str().unwrap_or("?"),
                    if p.is_empty() { "/".into() } else { p }
                ));
            }
        }
        gates.push(Gate {
            id: "S8",
            label: "resolved samples validate map-object-resolved.schema.json",
            errs,
        });
    }

    // S9
    {
        let mut errs = Vec::new();
        let mut by_kind: HashMap<String, HashSet<String>> = HashMap::new();
        for p in arr(&prefabs_sample) {
            if let (Some(k), Some(c)) = (p["kind"].as_str(), p["class"].as_str()) {
                by_kind
                    .entry(k.to_string())
                    .or_default()
                    .insert(c.to_string());
            }
        }
        for (kind, expected) in &expected_classes_for_kind {
            let empty = HashSet::new();
            let have = by_kind.get(*kind).unwrap_or(&empty);
            for cls in expected {
                if !have.contains(cls) {
                    errs.push(format!("prefabs-sample: missing enum example {kind}/{cls}"));
                }
            }
        }
        let seg_classes: HashSet<&str> = arr(&roads_sample["roadSegments"])
            .iter()
            .filter_map(|s| s["roadClass"].as_str())
            .map(|s| Box::leak(s.to_string().into_boxed_str()) as &str)
            .collect();
        for cls in enum_vec("roadClass") {
            if !seg_classes.contains(cls.as_str()) {
                errs.push(format!(
                    "roads-sample: missing segment example roadClass '{cls}'"
                ));
            }
        }
        let region_kinds: HashSet<String> = arr(&regions_sample)
            .iter()
            .filter_map(|r| r["kind"].as_str().map(String::from))
            .collect();
        for kind in enum_vec("regionKind") {
            if !region_kinds.contains(&kind) {
                errs.push(format!(
                    "regions-sample: missing region example kind '{kind}'"
                ));
            }
        }
        gates.push(Gate {
            id: "S9",
            label: "full closed-enum coverage (prefab classes + road segments + region kinds)",
            errs,
        });
    }

    spatial_invariants::append_spatial_gates(
        spatial_invariants::SpatialFixtures {
            sroot: &sroot,
            chunk_sample: &chunk_sample,
            prefabs_sample: &prefabs_sample,
            anchor_fixture: &anchor_fixture,
            density_fixture: &density_fixture,
            density_bin: &density_bin,
            region_fixture: &region_fixture,
        },
        &mut gates,
    )?;

    // S15 — T-935.12. Read as BYTES, and a missing or unreadable golden is a FAIL rather than a
    // silently skipped sub-gate: T-975 is exactly that bug on the density fixture next door.
    {
        let p = mo(&["map-object-chunk-sample.bin"]);
        let errs = match fs::read(&p) {
            Ok(bytes) => chunk_bin_errors(&chunk_sample, &prefabs_sample, &bytes),
            Err(e) => vec![format!("read {}: {e}", p.display())],
        };
        gates.push(Gate { id: "S15", label: "TBDC chunk golden — emitter byte identity, bin/json column parity, offset decode, header extent", errs });
    }

    let mut failures = 0usize;
    for g in &gates {
        if g.errs.is_empty() {
            println!("  PASS  {} — {}", g.id, g.label);
        } else {
            failures += g.errs.len();
            println!("  FAIL  {} — {}", g.id, g.label);
            for e in &g.errs {
                println!("        {e}");
            }
        }
    }
    if failures > 0 {
        eprintln!("\nverify-map-object-golden: FAIL ({failures} error(s))");
        Ok(1)
    } else {
        println!(
            "\nverify-map-object-golden: OK (S2–S9 + S11–S15; {} prefabs, {} instances, {} chunk rows, {} segments, {} regions, {} resolved; zero missing enum examples)",
            arr(&prefabs_sample).len(),
            arr(&instances_sample).len(),
            arr(&chunk_sample["chunk"]["instances"]).len(),
            arr(&roads_sample["roadSegments"]).len(),
            arr(&regions_sample).len(),
            arr(&resolved_sample).len()
        );
        Ok(0)
    }
}
