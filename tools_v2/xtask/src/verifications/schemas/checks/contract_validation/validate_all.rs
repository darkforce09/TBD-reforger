use super::*;
use developer_tools::repository_layout::{
    terrain_dir, terrain_manifest_path, terrain_registry_path,
};

/// The full contract-validation suite:
/// golden missions + registries + compat FK walkers + addon/variant provenance + bridge samples +
/// terrain manifests/anchors + ENF-4 Enfusion DTO fixtures + the map-object goldens.
/// Cross-file `$ref`s resolve through a `referencing::Registry` keyed by each schema's `$id`
/// (the ajv `addSchema` equivalent); ENF-4 pointer validators are built as `{"$ref": "<id>#/$defs/<n>"}`.
pub fn validate_all() -> Result<u8> {
    let root = repo_root()?;
    let sroot = schema_root(&root);
    let schema = |name: &str| read_json(&definition_path(&root, name));
    let reg_file = |name: &str| registry_fixtures_dir(&root).join(name);
    // Live Workbench exports sit beside the fixtures, not among them: the arsenal the platform
    // actually ingests must never be satisfiable by a sample file.
    let catalog_file = |name: &str| contract_catalogs_dir(&root).join(name);

    // Register every map-object schema (plus mission for the ENF-4 pointers) by $id.
    let mut registered: Vec<(String, Value)> = Vec::new();
    for f in [
        "map-object-enums.schema.json",
        "map-object-prefab.schema.json",
        "map-object-instance.schema.json",
        "map-object-region.schema.json",
        "map-object-roads.schema.json",
        "map-object-catalog.schema.json",
        "map-object-resolved.schema.json",
        "map-object-type-inventory.schema.json",
        "terrain-registry.schema.json",
        "mission.schema.json",
    ] {
        let doc = schema(f)?;
        let id = doc["$id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("{f}: missing $id"))?
            .to_string();
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
        .map_err(|e| anyhow::anyhow!("registry prepare: {e}"))?;
    let compile = |doc: &Value| -> Result<jsonschema::Validator> {
        jsonschema::options()
            .with_registry(&registry)
            .build(doc)
            .map_err(|e| anyhow::anyhow!("schema compile: {e}"))
    };
    let by_id = |name: &str| -> Result<jsonschema::Validator> {
        compile(&serde_json::json!({
            "$ref": format!("https://schema.tbdevent.eu/{name}/v1.json")
        }))
    };

    let failures = std::cell::Cell::new(0usize);
    let check = |label: &str, v: &jsonschema::Validator, data: &Value| {
        let errs: Vec<String> = v
            .iter_errors(data)
            .map(|e| {
                let p = e.instance_path().to_string();
                format!(
                    "        {} {e}",
                    if p.is_empty() { "/".to_string() } else { p }
                )
            })
            .collect();
        if errs.is_empty() {
            println!("  PASS  {label}");
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  {label}");
            for e in errs {
                println!("{e}");
            }
        }
    };

    let v_mission = compile(&schema("mission.schema.json")?)?;
    let v_registry = compile(&schema("registry.schema.json")?)?;
    let v_items = compile(&schema("registry-items.schema.json")?)?;
    let v_compat = compile(&schema("registry-compat.schema.json")?)?;
    let v_loadout = compile(&schema("loadout-export.schema.json")?)?;
    let v_bridge = compile(&read_json(
        &sroot.join("definitions/bridge-messages.schema.json"),
    )?)?;
    let v_tmanifest = compile(&schema("terrain-manifest.schema.json")?)?;
    let v_anchors = compile(&schema("terrain-anchors.schema.json")?)?;
    let v_editor = compile(&schema("mission-editor-payload.schema.json")?)?;
    let v_locations = compile(&schema("locations.schema.json")?)?;
    let v_hlabels = compile(&schema("height-labels.schema.json")?)?;
    let v_faction = compile(&schema("faction-library.schema.json")?)?;
    let v_mo_prefab = by_id("map-object-prefab")?;
    let v_mo_instance = by_id("map-object-instance")?;
    let v_mo_region = by_id("map-object-region")?;
    let v_mo_roads = by_id("map-object-roads")?;
    let v_mo_catalog = by_id("map-object-catalog")?;
    let v_mo_resolved = by_id("map-object-resolved")?;
    let v_mo_inventory = by_id("map-object-type-inventory")?;
    let v_tregistry = by_id("terrain-registry")?;

    let sorted_json_files = |dir: &Path| -> Result<Vec<String>> {
        let mut v: Vec<String> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".json"))
            .collect();
        v.sort();
        Ok(v)
    };

    mission_validation::validate(
        &root,
        &sroot,
        &schema,
        &sorted_json_files,
        &check,
        &failures,
        &v_mission,
    )?;

    registry_validation::validate(
        &reg_file,
        &catalog_file,
        &check,
        &failures,
        &v_registry,
        &v_items,
        &v_compat,
    )?;

    println!("Faction library:");
    check(
        "faction-library.sample.json",
        &v_faction,
        &read_json(&reg_file("faction-library.sample.json"))?,
    );

    println!("Loadout export:");
    check(
        "loadout-export.sample.json",
        &v_loadout,
        &read_json(&reg_file("loadout-export.sample.json"))?,
    );
    check(
        "loadout-export.v2.sample.json",
        &v_loadout,
        &read_json(&reg_file("loadout-export.v2.sample.json"))?,
    );

    println!("Mission editor payload:");
    check(
        "mission-editor-payload.sample.json",
        &v_editor,
        &read_json(&reg_file("mission-editor-payload.sample.json"))?,
    );

    println!("Bridge message samples:");
    let samples = sroot.join("fixtures/bridge_samples");
    for f in sorted_json_files(&samples)? {
        check(&f, &v_bridge, &read_json(&samples.join(&f))?);
    }

    println!("Terrain manifest:");
    check(
        "everon/manifest.json",
        &v_tmanifest,
        &read_json(&terrain_manifest_path(&root, "everon"))?,
    );

    println!("Locations:");
    check(
        "locations-everon-sample.json",
        &v_locations,
        &read_json(&sroot.join("fixtures/map/locations-everon-sample.json"))?,
    );
    let everon_loc = terrain_dir(&root, "everon").join("locations.json");
    if everon_loc.exists() {
        check(
            "map-assets/everon/locations.json",
            &v_locations,
            &read_json(&everon_loc)?,
        );
    }

    println!("Height labels:");
    let hl = terrain_dir(&root, "everon").join("height-labels.json");
    if hl.exists() {
        check(
            "map-assets/everon/height-labels.json",
            &v_hlabels,
            &read_json(&hl)?,
        );
    }

    println!("Terrain anchors example:");
    check(
        "everon/anchors/verification.example.json",
        &v_anchors,
        &read_json(&terrain_dir(&root, "everon").join("anchors/verification.example.json"))?,
    );

    println!("Enfusion DTO fixtures (ENF-4):");
    let mission_id = registered
        .iter()
        .find(|(_, d)| {
            d["$id"]
                .as_str()
                .map(|s| s.contains("mission"))
                .unwrap_or(false)
        })
        .map(|(id, _)| id.clone())
        .unwrap_or_default();
    let enf = sroot.join("fixtures/enfusion_samples");
    for f in sorted_json_files(&enf)? {
        if !f.ends_with(".sample.json") {
            continue;
        }
        let base = f.trim_end_matches(".sample.json");
        let data = read_json(&enf.join(&f))?;
        if base == "root" {
            check(&f, &v_mission, &data);
        } else {
            match compile(&serde_json::json!({ "$ref": format!("{mission_id}#/$defs/{base}") })) {
                Ok(v) => check(&f, &v, &data),
                Err(_) => {
                    failures.set(failures.get() + 1);
                    println!("  FAIL  {f} (no schema for #/$defs/{base})");
                }
            }
        }
    }

    let mo = sroot.join("fixtures/map");
    println!("Map object prefabs (S9 — one row per buildingClass):");
    for (i, row) in read_json(&mo.join("map-object-prefabs-sample.json"))?
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(
            &format!(
                "prefab[{i}] {}/{}",
                row["kind"].as_str().unwrap_or("?"),
                row["class"].as_str().unwrap_or("?")
            ),
            &v_mo_prefab,
            row,
        );
    }

    println!("Map object instances:");
    for (i, row) in read_json(&mo.join("map-object-instances-sample.json"))?
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(&format!("instance[{i}]"), &v_mo_instance, row);
    }

    println!("Map object chunk sample (all-number 5- or 8-tuples):");
    let chunk = read_json(&mo.join("map-object-chunk-sample.json"))?;
    for (i, row) in chunk["chunk"]["instances"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(&format!("chunk-instance[{i}]"), &v_mo_instance, row);
    }

    println!("Map object regions (forest / field):");
    for (i, row) in read_json(&mo.join("map-object-regions-everon-sample.json"))?
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(
            &format!("region[{i}] {}", row["kind"].as_str().unwrap_or("?")),
            &v_mo_region,
            row,
        );
    }

    println!("Map object roads:");
    check(
        "map-object-roads-sample.json",
        &v_mo_roads,
        &read_json(&mo.join("map-object-roads-sample.json"))?,
    );

    println!("Map object catalog bundle (validation-only, N12):");
    check(
        "map-object-catalog-everon-sample.json",
        &v_mo_catalog,
        &read_json(&mo.join("map-object-catalog-everon-sample.json"))?,
    );
    check(
        "phased/P1-buildings.json",
        &v_mo_catalog,
        &read_json(&mo.join("phased/P1-buildings.json"))?,
    );

    println!("ResolvedWorldObject (Eden AI):");
    for (i, row) in read_json(&mo.join("map-object-resolved-sample.json"))?
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(
            &format!("resolved[{i}] {}", row["kind"].as_str().unwrap_or("?")),
            &v_mo_resolved,
            row,
        );
    }

    println!("Terrain registry:");
    check(
        "golden terrain-registry.sample.json",
        &v_tregistry,
        &read_json(&mo.join("terrain-registry.sample.json"))?,
    );
    check(
        "map-assets/terrain-registry.json",
        &v_tregistry,
        &read_json(&terrain_registry_path(&root))?,
    );

    println!("Dual + tile-only terrain manifests:");
    check(
        "everon-dual-tiles",
        &v_tmanifest,
        &read_json(&mo.join("terrain-manifest-everon-dual-tiles.json"))?,
    );
    check(
        "everon-tile-only-satellite",
        &v_tmanifest,
        &read_json(&mo.join("terrain-manifest-everon-tile-only-satellite.json"))?,
    );
    check(
        "everon-unified-satellite",
        &v_tmanifest,
        &read_json(&mo.join("terrain-manifest-everon-unified-satellite.json"))?,
    );

    println!("Map object type inventory (exact counts — pending until export):");
    check(
        "type-inventory-pending-everon.json",
        &v_mo_inventory,
        &read_json(&mo.join("type-inventory-pending-everon.json"))?,
    );
    check(
        "map-assets/everon/objects/type-inventory.json",
        &v_mo_inventory,
        &read_json(&terrain_dir(&root, "everon").join("objects/type-inventory.json"))?,
    );

    println!("TBD_MissionValidator unconsumed-key warnings:");
    {
        let validator_c =
            root.join("apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/TBD_MissionValidator.c");
        let src = fs::read_to_string(&validator_c)
            .with_context(|| format!("read {}", validator_c.display()))?;
        let mut bad = Vec::new();
        if !src.contains("CheckUnconsumedKeys(mission)") {
            bad.push(
                "CheckUnconsumedKeys is not wired from Run() — unconsumed keys would stay silent"
                    .to_string(),
            );
        }
        // `entities` is modeled + spawned — no longer an unconsumed-key warn.
        for key in ["environment", "settings", "layers", "tickets", "radio"] {
            let marker = format!("UNCONSUMED-WARN: {key}");
            if !src.contains(&marker) {
                bad.push(format!("missing marker comment `{marker}`"));
            }
        }
        for (subject, needle) in [
            ("environment", "AddWarning(\"environment\","),
            ("settings", "AddWarning(\"settings\","),
            ("layers", "AddWarning(\"layers\","),
            ("factions.tickets", "AddWarning(\"factions.tickets\","),
            ("orbat.roles.radio", "AddWarning(\"orbat.roles.radio\","),
        ] {
            if !src.contains(needle) {
                bad.push(format!(
                    "missing AddWarning for `{subject}` — authors get no signal for that key"
                ));
            }
        }
        // Regression: the retired entities unconsumed lie must not return.
        if src.contains("AddWarning(\"entities\",") {
            bad.push(
                "entities AddWarning must stay retired (the mod spawns entities[])".to_string(),
            );
        }
        if src.contains("does not spawn mission entities")
            || src.contains("does not spawn the mission document")
        {
            bad.push(
                "forbidden entities[]-never-spawn lie string still present in MissionValidator"
                    .to_string(),
            );
        }
        // `empty-warning-fields.json` is the deliberate all-keys-authored negative-control golden.
        // Still requires `entities` authored (valid mission key) even though it no longer warns.
        let neg = read_json(&sroot.join("fixtures/missions/valid/empty-warning-fields.json"))?;
        for key in [
            "environment",
            "settings",
            "entities",
            "layers",
            "tickets",
            "radio",
        ] {
            let present = match key {
                "tickets" => neg["factions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|f| f.get("tickets").is_some()),
                "radio" => neg["orbat"]
                    .as_object()
                    .into_iter()
                    .flatten()
                    .flat_map(|(_, fv)| fv["groups"].as_array().into_iter().flatten())
                    .flat_map(|g| g["roles"].as_array().into_iter().flatten())
                    .any(|r| r.get("radio").is_some()),
                _ => neg.get(key).is_some(),
            };
            if !present {
                bad.push(format!(
                    "fixtures/missions/valid/empty-warning-fields.json no longer authors `{key}` — \
                     the runtime negative-control fixture drifted"
                ));
            }
        }
        if bad.is_empty() {
            println!(
                "  PASS  TBD_MissionValidator.c (5 unconsumed-key warnings wired; entities retired)"
            );
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  TBD_MissionValidator unconsumed-key warnings");
            for b in &bad {
                println!("        {b}");
            }
        }
    }

    if failures.get() > 0 {
        eprintln!("\n{} validation failure(s).", failures.get());
        Ok(1)
    } else {
        println!("\nAll contracts valid.");
        Ok(0)
    }
}
