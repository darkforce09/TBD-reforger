use super::*;
use developer_tools::repository_layout::glyph_manifest_path;

pub fn map_object_enums() -> Result<u8> {
    let root = repo_root()?;
    let sroot = schema_root(&root);
    let enums = read_json(&sroot.join("definitions/map-object-enums.schema.json"))?;
    let defs = &enums["$defs"];
    let set = |name: &str| -> HashSet<String> {
        defs[name]["enum"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let sets: BTreeMap<&str, HashSet<String>> = BTreeMap::from([
        ("kind", set("kind")),
        ("buildingClass", set("buildingClass")),
        ("roadClass", set("roadClass")),
        ("speciesClass", set("speciesClass")),
        ("forestClass", set("forestClass")),
        ("rockClass", set("rockClass")),
        ("propClass", set("propClass")),
        ("utilityClass", set("utilityClass")),
        ("waterClass", set("waterClass")),
        // T-244 vehicle lane. This entry is NOT optional bookkeeping: the last check in check_row
        // is `sets[enum_name]`, and BTreeMap's Index impl PANICS on a missing key. Adding a kind to
        // class_enum_for_kind below WITHOUT adding its enum here turns a clean FAIL into a crash.
        ("vehicleClass", set("vehicleClass")),
    ]);
    let class_enum_for_kind: BTreeMap<&str, &str> = BTreeMap::from([
        ("building", "buildingClass"),
        ("road", "roadClass"),
        ("tree", "speciesClass"),
        ("vegetation", "speciesClass"),
        ("rock", "rockClass"),
        ("prop", "propClass"),
        ("utility", "utilityClass"),
        ("water", "waterClass"),
        ("vehicle", "vehicleClass"),
    ]);

    let mut errors: Vec<String> = Vec::new();
    let mut check_row = |src: String, kind: Option<&str>, class: Option<&str>| {
        let Some(kind) = kind else {
            return;
        };
        if !sets["kind"].contains(kind) {
            errors.push(format!(
                "{src}: kind '{kind}' not in map-object-enums#/$defs/kind"
            ));
            return;
        }
        let Some(enum_name) = class_enum_for_kind.get(kind) else {
            errors.push(format!(
                "{src}: kind '{kind}' has no class-enum mapping (regions carry no prefab class)"
            ));
            return;
        };
        if let Some(klass) = class
            && !sets[enum_name].contains(klass)
        {
            errors.push(format!(
                "{src}: class '{klass}' not in {enum_name} (kind={kind})"
            ));
        }
    };

    let prefabs = read_json(&sroot.join("fixtures/map/map-object-prefabs-sample.json"))?;
    let prefab_count = prefabs.as_array().map(Vec::len).unwrap_or(0);
    for p in prefabs.as_array().into_iter().flatten() {
        check_row(
            format!("golden prefab {}", p["prefabId"]),
            p["kind"].as_str(),
            p["class"].as_str(),
        );
    }

    let classify = read_json(&sroot.join("rules/prefab-classify.json"))?;
    for (i, r) in classify["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check_row(
            format!("prefab-classify rule[{i}]"),
            r["kind"].as_str(),
            r["class"].as_str(),
        );
    }
    if classify["fallback"].is_object() {
        check_row(
            "prefab-classify fallback".to_string(),
            classify["fallback"]["kind"].as_str(),
            classify["fallback"]["class"].as_str(),
        );
    }

    let regions = read_json(&sroot.join("fixtures/map/map-object-regions-everon-sample.json"))?;
    for reg in regions.as_array().into_iter().flatten() {
        let id = &reg["id"];
        if let Some(kind) = reg["kind"].as_str()
            && !sets["kind"].contains(kind)
        {
            errors.push(format!("region {id}: kind '{kind}' not in kind enum"));
        }
        if let Some(d) = reg["dominantSpeciesClass"].as_str()
            && !sets["forestClass"].contains(d)
        {
            errors.push(format!(
                "region {id}: dominantSpeciesClass '{d}' not in forestClass"
            ));
        }
    }

    let glyphs_doc = read_json(&glyph_manifest_path(&root))?;
    let glyphs = glyphs_doc["glyphs"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    for key in glyphs.keys() {
        let kind_tok = key.split('-').next().unwrap_or("");
        if !sets["kind"].contains(kind_tok) {
            errors.push(format!(
                "glyph '{key}': kind prefix '{kind_tok}' not in kind enum"
            ));
        }
    }

    if errors.is_empty() {
        println!(
            "verify-map-object-enums: OK ({prefab_count} prefabs, {} glyphs, enums single-source)",
            glyphs.len()
        );
        Ok(0)
    } else {
        eprintln!("verify-map-object-enums: FAIL");
        for e in &errors {
            eprintln!("  {e}");
        }
        Ok(1)
    }
}
