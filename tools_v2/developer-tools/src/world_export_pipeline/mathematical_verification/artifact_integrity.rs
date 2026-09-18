use super::*;

pub(super) fn verify(g: &mut Gates, data: ArtifactData<'_>, validators: ArtifactValidators<'_>) {
    let ArtifactData {
        prefabs,
        rows_by_key,
        roads_doc,
        inventory,
        chunk_manifest,
    } = data;
    let ArtifactValidators {
        v_prefab,
        v_instance,
        v_roads,
        v_inventory,
        v_resolved,
    } = validators;
    let first_err = |v: &jsonschema::Validator, val: &Value| -> String {
        v.iter_errors(val)
            .next()
            .map(|e| e.to_string())
            .unwrap_or_default()
    };

    {
        let mut errs = Vec::new();
        for p in prefabs {
            if !v_prefab.is_valid(p) {
                errs.push(format!(
                    "prefab {}: {}",
                    p["prefabId"],
                    first_err(v_prefab, p)
                ));
            }
        }
        for (key, rows) in rows_by_key {
            for (i, row) in rows.iter().enumerate() {
                if !v_instance.is_valid(row) {
                    errs.push(format!("chunk {key}[{i}]: {}", first_err(v_instance, row)));
                } else {
                    // T-090.12.1 — rows are exactly 5 or 8 numbers; an 8-wide row must carry a
                    // non-trivial pitch / roll / scale (trivial trailers are written 5-wide).
                    let n = row.as_array().map_or(0, Vec::len);
                    let num = |k: usize| row[k].as_f64();
                    if !(n == 5 || n == 8) || !row[0].is_number() {
                        errs.push(format!("chunk {key}[{i}]: not a 5- or 8-number tuple"));
                    } else if n == 8
                        && num(5) == Some(0.0)
                        && num(6) == Some(0.0)
                        && num(7) == Some(1.0)
                    {
                        errs.push(format!(
                            "chunk {key}[{i}]: 8-wide row with trivial trailers (must be 5-wide)"
                        ));
                    }
                }
            }
        }
        if !v_roads.is_valid(roads_doc) {
            errs.push(format!("roads.json.gz: {}", first_err(v_roads, roads_doc)));
        }
        if !v_inventory.is_valid(inventory) {
            errs.push(format!(
                "type-inventory.json: {}",
                first_err(v_inventory, inventory)
            ));
        }
        g.gate(
            "G1",
            "schema valid (prefabs, chunk rows, roads, inventory)",
            errs,
        );
    }

    // ---- G2 resolved materialization ----
    {
        let mut errs = Vec::new();
        for (key, rows) in rows_by_key {
            for (i, row) in rows.iter().enumerate() {
                let Some(p) = row[0].as_u64().and_then(|id| prefabs.get(id as usize)) else {
                    continue; // G3's finding
                };
                let resolved = json!({
                    "id": format!("{key}:{i}"),
                    "prefabId": p["prefabId"], "resourceName": p["resourceName"],
                    "kind": p["kind"], "class": p["class"],
                    "label": p["label"].as_str().unwrap_or(""),
                    "taxonomyPath": p["ai"]["taxonomyPath"], "summary": p["ai"]["summary"],
                    "x": row[1], "y": row[2], "z": row[3], "rotationDeg": row[4],
                    "spatial": p["spatial"], "gameplay": p["gameplay"],
                    "tags": p["tags"].as_array().cloned().unwrap_or_default(),
                });
                if !v_resolved.is_valid(&resolved) {
                    errs.push(format!(
                        "resolved {key}:{i}: {}",
                        first_err(v_resolved, &resolved)
                    ));
                }
            }
        }
        g.gate(
            "G2",
            "all instances materialize to valid ResolvedWorldObject",
            errs,
        );
    }

    // ---- G3 / G12 prefab bijection + orphans ----
    {
        let mut errs = Vec::new();
        let mut referenced = vec![0u64; prefabs.len()];
        for (key, rows) in rows_by_key {
            for (i, row) in rows.iter().enumerate() {
                match row[0].as_i64() {
                    Some(id) if id >= 0 && (id as usize) < prefabs.len() => {
                        referenced[id as usize] += 1
                    }
                    other => errs.push(format!(
                        "chunk {key}[{i}]: prefabId {} out of range",
                        other.map_or_else(|| row[0].to_string(), |v| v.to_string())
                    )),
                }
            }
        }
        g.gate("G3", "prefabId bijection (0 <= id < prefabs.length)", errs);
        let orphans: Vec<String> = prefabs
            .iter()
            .enumerate()
            .filter(|(i, p)| {
                referenced[*i] == 0
                    && !p["tags"]
                        .as_array()
                        .is_some_and(|t| t.iter().any(|x| x == "prefabOnly"))
            })
            .map(|(_, p)| {
                format!(
                    "prefab {} {} has 0 instances",
                    p["prefabId"],
                    p["resourceName"].as_str().unwrap_or("")
                )
            })
            .collect();
        g.gate("G12", "no orphan prefabs", orphans);
    }

    // ---- G5 derived-id uniqueness + sidecar consistency ----
    {
        let mut errs = Vec::new();
        let cells = chunk_manifest["cells"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let sidecar_keys: HashSet<String> = cells
            .iter()
            .map(|c| {
                chunk_key(
                    c["cx"].as_i64().unwrap_or(-1),
                    c["cy"].as_i64().unwrap_or(-1),
                )
            })
            .collect();
        if sidecar_keys.len() != cells.len() {
            errs.push("chunks/manifest.json: duplicate (cx,cy) cells".into());
        }
        let by_key: HashMap<&str, usize> = rows_by_key
            .iter()
            .map(|(k, r)| (k.as_str(), r.len()))
            .collect();
        for c in &cells {
            let key = chunk_key(
                c["cx"].as_i64().unwrap_or(-1),
                c["cy"].as_i64().unwrap_or(-1),
            );
            match by_key.get(key.as_str()) {
                None => errs.push(format!("sidecar cell {key}: chunk file missing")),
                Some(&n) if n as u64 != c["instanceCount"].as_u64().unwrap_or(0) => {
                    errs.push(format!(
                        "sidecar cell {key}: instanceCount {} != actual {n}",
                        c["instanceCount"]
                    ))
                }
                _ => {}
            }
        }
        for (key, _) in rows_by_key {
            if !sidecar_keys.contains(key) {
                errs.push(format!("chunk file {key} not in sidecar manifest"));
            }
        }
        g.gate(
            "G5",
            "derived instance ids unique (sidecar <-> files consistent)",
            errs,
        );
    }
}

pub(super) struct ArtifactData<'a> {
    pub(super) prefabs: &'a Vec<Value>,
    pub(super) rows_by_key: &'a Vec<(String, Vec<Value>)>,
    pub(super) roads_doc: &'a Value,
    pub(super) inventory: &'a Value,
    pub(super) chunk_manifest: &'a Value,
}
pub(super) struct ArtifactValidators<'a> {
    pub(super) v_prefab: &'a jsonschema::Validator,
    pub(super) v_instance: &'a jsonschema::Validator,
    pub(super) v_roads: &'a jsonschema::Validator,
    pub(super) v_inventory: &'a jsonschema::Validator,
    pub(super) v_resolved: &'a jsonschema::Validator,
}
