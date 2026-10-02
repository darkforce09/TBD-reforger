use super::*;
use crate::repository_layout::terrain_dir;

pub(super) fn obj_get<'a>(v: &'a Value, k: &str) -> Option<&'a Value> {
    v.as_object().and_then(|m| m.get(k))
}

/// The rule that produced a committed row, identified by its `kind`+`class` pair — the same
/// lookup the world-object build uses to recover `render.importanceZoom` for a census bucket. `None` when
/// the pair is the fallback's (or no rule claims it), which is itself the answer: the fallback.
pub(crate) fn rule_for_kind_class<'a>(rules: &'a Rules, kind: &str, class: &str) -> &'a Value {
    if let Some(arr) = rules.doc["rules"].as_array()
        && let Some(r) = arr
            .iter()
            .find(|r| r["kind"] == json!(kind) && r["class"] == json!(class))
    {
        return r;
    }
    &rules.doc["fallback"]
}

/// Keep a measured OBB, re-template a fallback one.
///
/// The world-object build writes `spatial` from per-axis medians of sampled engine halfExtents when it has
/// samples, and copies the matched rule's `spatial` template when it does not. Only the second
/// case is reproducible here, and it is detectable: a committed `spatial` byte-equal to its
/// producing rule's template was a template. Anything else is a measurement that exists nowhere
/// else in the repo, so it is preserved.
///
/// The one indistinguishable case — a measurement that happens to equal its own template — is
/// re-templated. That is only observable when the row also changes kind, and it swaps one
/// hand-authored template for another rather than destroying a measurement.
pub(super) fn respatialize(committed: &Value, old_rule: &Value, new_rule: &Value) -> Value {
    if committed == &old_rule["spatial"] {
        new_rule["spatial"].clone()
    } else {
        committed.clone()
    }
}

/// Rebuild one prefab row from the rules, preserving identity (`prefabId`, `resourceName`,
/// `label`) and measured geometry. Key order matches the build's emitted row exactly —
/// serde_json is built with `preserve_order`, so this is a byte-level contract, not cosmetics.
pub(super) fn rebuild_row(
    rules: &Rules,
    classify: &mut Classifier,
    row: &Value,
) -> (Value, Option<Drift>) {
    let rn = row["resourceName"].as_str().unwrap_or_default().to_string();
    let old_kind = row["kind"].as_str().unwrap_or_default().to_string();
    let old_class = row["class"].as_str().unwrap_or_default().to_string();
    let cls = classify.classify(&rn);
    let new_rule = rules.rule(cls.rule_idx).clone();
    let old_rule = rule_for_kind_class(rules, &old_kind, &old_class).clone();

    let mut ai = Map::new();
    ai.insert("summary".into(), new_rule["ai"]["summary"].clone());
    ai.insert(
        "taxonomyPath".into(),
        new_rule["ai"]["taxonomyPath"].clone(),
    );
    ai.insert("classificationSource".into(), json!("rules-v1/prefab-name"));
    ai.insert(
        "confidence".into(),
        if new_rule["ai"]["confidence"].is_null() {
            json!(0.5)
        } else {
            new_rule["ai"]["confidence"].clone()
        },
    );
    ai.insert("needsReview".into(), json!(!cls.matched));

    let mut out = Map::new();
    out.insert("prefabId".into(), row["prefabId"].clone());
    out.insert("resourceName".into(), json!(rn));
    out.insert("kind".into(), json!(cls.kind));
    out.insert("class".into(), json!(cls.class));
    out.insert("label".into(), row["label"].clone());
    out.insert("ai".into(), Value::Object(ai));
    out.insert(
        "spatial".into(),
        respatialize(
            obj_get(row, "spatial").unwrap_or(&Value::Null),
            &old_rule,
            &new_rule,
        ),
    );
    out.insert("gameplay".into(), new_rule["gameplay"].clone());
    if !new_rule["render"].is_null() {
        out.insert("render".into(), new_rule["render"].clone());
    }
    if !new_rule["tags"].is_null() {
        out.insert("tags".into(), new_rule["tags"].clone());
    }

    let drift = (cls.kind != old_kind || cls.class != old_class).then(|| Drift {
        prefab_id: row["prefabId"].as_u64().unwrap_or_default(),
        resource_name: rn,
        old_kind,
        old_class,
        new_kind: cls.kind,
        new_class: cls.class,
    });
    (Value::Object(out), drift)
}

/// The pure core: committed prefab rows + rules → rebuilt rows + a drift report.
///
/// Split out from all IO so it is testable against a synthetic catalogue, and so the perturbation
/// proof does not depend on a 1,623-row artifact being present.
pub fn reclassify_rows(rules: &Rules, committed: &[Value]) -> Result<(Vec<Value>, Report)> {
    let mut classify = Classifier::new(rules);
    let mut out = Vec::with_capacity(committed.len());
    let mut drift = Vec::new();
    let mut kinds_before: BTreeMap<String, u64> = BTreeMap::new();
    let mut kinds_after: BTreeMap<String, u64> = BTreeMap::new();
    let (mut unclassified_before, mut unclassified_after, mut matched) = (0u64, 0u64, 0usize);

    for row in committed {
        *kinds_before
            .entry(row["kind"].as_str().unwrap_or_default().to_string())
            .or_default() += 1;
        if row["class"] == json!("unknown") {
            unclassified_before += 1;
        }
        let (new_row, d) = rebuild_row(rules, &mut classify, row);
        *kinds_after
            .entry(new_row["kind"].as_str().unwrap_or_default().to_string())
            .or_default() += 1;
        if new_row["class"] == json!("unknown") {
            unclassified_after += 1;
        }
        if new_row["ai"]["needsReview"] == json!(false) {
            matched += 1;
        }
        if let Some(d) = d {
            drift.push(d);
        }
        out.push(new_row);
    }

    // Non-vacuity guard, and the one that catches a gutted rules file: a rules doc
    // that classifies nothing would silently re-stamp every row as the fallback `prop/unknown`
    // and report a large, confident, wrong drift. Refuse instead.
    super::super::refuse_empty_write(
        "reclassify catalogue",
        out.is_empty() || (matched == 0 && !out.is_empty()),
        if out.is_empty() {
            "zero prefab rows in the committed catalogue"
        } else {
            "no prefab matched any rule — prefab-classify.json is empty or unreadable"
        },
    )?;

    let new_kinds = kinds_after
        .keys()
        .filter(|k| !kinds_before.contains_key(*k))
        .cloned()
        .collect();
    let report = Report {
        rows: out.len(),
        matched,
        drift,
        kinds_before,
        kinds_after,
        unclassified_before,
        unclassified_after,
        new_kinds,
    };
    Ok((out, report))
}

/// Instance count per `prefabId` from the committed chunk files — the census input that
/// the world-object build gets from the raw stream and that we get from the artifact it wrote.
pub(super) fn instances_by_prefab(chunks_dir: &Path, n_prefabs: usize) -> Result<Vec<u64>> {
    let mut counts = vec![0u64; n_prefabs];
    let mut chunk_files = 0u64;
    let rd = std::fs::read_dir(chunks_dir).with_context(|| chunks_dir.display().to_string())?;
    for e in rd.filter_map(std::result::Result::ok) {
        let p = e.path();
        if !p.to_string_lossy().ends_with(".json.gz") {
            continue;
        }
        chunk_files += 1;
        let doc: Value = serde_json::from_slice(&gunzip(&std::fs::read(&p)?)?)
            .with_context(|| p.display().to_string())?;
        for inst in doc["instances"].as_array().into_iter().flatten() {
            let id = inst[0].as_u64().unwrap_or(u64::MAX) as usize;
            let Some(slot) = counts.get_mut(id) else {
                bail!(
                    "{}: instance references prefabId {id}, catalogue has {n_prefabs} rows",
                    p.display()
                );
            };
            *slot += 1;
        }
    }
    // A silently-empty chunks dir would produce an all-zero census that still validates.
    super::super::refuse_empty_write(
        "reclassify census",
        chunk_files == 0 || counts.iter().all(|c| *c == 0),
        "no committed chunk instances found — census would be vacuously zero",
    )?;
    Ok(counts)
}

/// Recompute the count lanes of `type-inventory.json` from the rebuilt rows, preserving every
/// other key and its order. Mirrors the build's census block.
/// Re-derive `needsReview` from the live rules; the number it corrects is the one
/// the ticket that touched it was named after.
///
/// The entries are staging-derived (`instanceCount` and `reason` come from a Workbench export
/// this repo cannot reproduce), which is why a blind carry-through is tempting. But
/// membership is NOT staging-derived: a prefab needs review exactly when the rules still fail to
/// classify it, and that is decidable here. Preserving the list wholesale meant a rule edit that
/// classified 420 of the 443 left the artifact still publishing `needsReview.prefabTypes = 443` —
/// the shipped file contradicted the catalogue beside it, and `verify type-inventory` is
/// shape-only so nothing caught it. Found by the wave 237 verifier.
///
/// Each surviving entry keeps its own recorded text; only entries the rules now classify are
/// dropped. `prefabTypes` is recounted from what remains.
pub(super) fn rebuild_needs_review(committed: &Value, classify: &mut Classifier) -> Option<Value> {
    let listed = committed["needsReview"]["prefabs"].as_array()?;
    let kept: Vec<Value> = listed
        .iter()
        .filter(|e| {
            let rn = e["resourceName"].as_str().unwrap_or_default();
            !classify.classify(rn).matched
        })
        .cloned()
        .collect();
    Some(json!({ "prefabTypes": kept.len(), "prefabs": kept }))
}

pub(super) fn rebuild_inventory(
    committed: &Value,
    rules: &Rules,
    classify: &mut Classifier,
    prefabs: &[Value],
    inst: &[u64],
    road_census: Option<(u64, Map<String, Value>)>,
) -> Value {
    let mut by_kind: Map<String, Value> = super::super::INSTANCE_KINDS
        .iter()
        .map(|k| {
            let mut m = Map::from_iter([
                ("prefabTypes".to_string(), json!(0)),
                ("instances".to_string(), json!(0)),
            ]);
            if *k == "road" {
                // Roads come from `.topo`, not the prefab lane — but the census is derivable from
                // the COMMITTED roads.json.gz, so recompute it rather than carrying a value that
                // is not hardcoded to 0 (`chunk_partitioner::road_census`). It falls back to the
                // committed number when the file cannot be read.
                m.insert(
                    "segments".into(),
                    road_census
                        .as_ref()
                        .map(|(n, _)| json!(n))
                        .unwrap_or_else(|| committed["byKind"]["road"]["segments"].clone()),
                );
            }
            (k.to_string(), Value::Object(m))
        })
        .collect();
    let mut by_building: Map<String, Value> = Map::new();
    let mut by_species: Map<String, Value> = Map::new();
    let rules_arr = rules.doc["rules"].as_array().cloned().unwrap_or_default();

    for (i, p) in prefabs.iter().enumerate() {
        let kind = p["kind"].as_str().unwrap_or_default();
        let class = p["class"].as_str().unwrap_or_default();
        let n = inst.get(i).copied().unwrap_or(0);
        if let Some(bk) = by_kind.get_mut(kind).and_then(Value::as_object_mut) {
            *bk.get_mut("prefabTypes").unwrap() = json!(bk["prefabTypes"].as_u64().unwrap() + 1);
            *bk.get_mut("instances").unwrap() = json!(bk["instances"].as_u64().unwrap() + n);
        }
        let target = match kind {
            "building" => Some(&mut by_building),
            "tree" | "vegetation" => Some(&mut by_species),
            _ => None,
        };
        if let Some(target) = target {
            let b = target
                .entry(class.to_string())
                .or_insert_with(|| json!({ "prefabTypes": 0, "instances": 0 }))
                .as_object_mut()
                .unwrap();
            *b.get_mut("prefabTypes").unwrap() = json!(b["prefabTypes"].as_u64().unwrap() + 1);
            *b.get_mut("instances").unwrap() = json!(b["instances"].as_u64().unwrap() + n);
            let iz = rules_arr
                .iter()
                .find(|r| r["kind"] == json!(kind) && r["class"] == json!(class))
                .map(|r| r["render"]["importanceZoom"].clone())
                .unwrap_or(Value::Null);
            if iz.is_number() {
                b.insert("importanceZoom".into(), iz);
            }
        }
    }
    let sorted = |m: Map<String, Value>| -> Value {
        let mut keys: Vec<String> = m.keys().cloned().collect();
        keys.sort();
        Value::Object(
            keys.into_iter()
                .map(|k| (k.clone(), m[&k].clone()))
                .collect(),
        )
    };

    let mut out = committed.as_object().cloned().unwrap_or_default();
    out.insert("byKind".into(), Value::Object(by_kind));
    out.insert("byBuildingClass".into(), sorted(by_building));
    out.insert("bySpeciesClass".into(), sorted(by_species));
    if let Some((_, by_class)) = road_census {
        out.insert("byRoadClass".into(), Value::Object(by_class));
    }
    if let Some(nr) = rebuild_needs_review(committed, classify) {
        out.insert("needsReview".into(), nr);
    }
    Value::Object(out)
}

/// `world reclassify` — see the module docs. Returns the process exit code: 0 = the committed
/// catalogue already agrees with the rules (or the rebuild was written), 1 = drift in check mode.
pub fn reclassify_terrain(terrain: &str, mode: Mode, out_base: Option<&Path>) -> Result<u8> {
    let terrain_dir = terrain_dir(&repo_root(), terrain);
    let objects = terrain_dir.join("objects");
    let prefabs_path = objects.join("prefabs.json.gz");
    if !prefabs_path.exists() {
        bail!(
            "reclassify: no committed catalogue at {} — this rebuilds an existing catalogue's \
             classification, it does not create one (that is `world build-objects`, which needs \
             the Workbench staging export)",
            prefabs_path.display()
        );
    }
    let doc: Value = serde_json::from_slice(&gunzip(&std::fs::read(&prefabs_path)?)?)
        .with_context(|| prefabs_path.display().to_string())?;
    let committed: Vec<Value> = doc["prefabs"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("{}: no prefabs array", prefabs_path.display()))?
        .clone();

    let rules = load_rules()?;
    let (rebuilt, report) = reclassify_rows(&rules, &committed)?;

    println!(
        "reclassify: {terrain} — {} prefab rows, {} matched a rule, {} fell through to fallback",
        report.rows,
        report.matched,
        report.rows - report.matched
    );
    println!("reclassify:   kinds before {:?}", report.kinds_before);
    println!("reclassify:   kinds after  {:?}", report.kinds_after);
    println!(
        "reclassify:   class=unknown {} -> {} ({:.1}% -> {:.1}% of catalogue)",
        report.unclassified_before,
        report.unclassified_after,
        100.0 * report.unclassified_before as f64 / report.rows as f64,
        100.0 * report.unclassified_after as f64 / report.rows as f64,
    );
    for d in &report.drift {
        println!(
            "reclassify:   #{} {}/{} -> {}/{}  {}",
            d.prefab_id, d.old_kind, d.old_class, d.new_kind, d.new_class, d.resource_name
        );
    }
    if !report.new_kinds.is_empty() {
        println!(
            "reclassify:   NEW census buckets {:?} — every consumer of type-inventory.json must \
             accept these before the rebuilt artifact can land",
            report.new_kinds
        );
    }

    if mode == Mode::Check {
        if report.is_clean() {
            println!("reclassify: CLEAN — the committed catalogue matches prefab-classify.json.");
            return Ok(0);
        }
        println!(
            "reclassify: DRIFT — {} prefab(s) classify differently than the committed catalogue. \
             The rules have changed since the last rebuild and the change is LATENT. \
             Re-run with --write to apply.",
            report.drift.len()
        );
        return Ok(1);
    }

    let out_objects = out_base.map_or_else(|| objects.clone(), |b| b.join("objects"));
    std::fs::create_dir_all(&out_objects)?;
    let inst = instances_by_prefab(&objects.join("chunks"), rebuilt.len())?;

    let mut prefabs_doc = json!({
        "schemaVersion": doc["schemaVersion"].clone(),
        "terrainId": doc["terrainId"].clone(),
        "prefabs": rebuilt,
    });
    js_normalize(&mut prefabs_doc);
    let bytes = serde_json::to_string(&prefabs_doc)?;
    std::fs::write(out_objects.join("prefabs.json.gz"), gz9(bytes.as_bytes())?)?;

    let inv_path = objects.join("type-inventory.json");
    if inv_path.exists() {
        let committed_inv: Value = serde_json::from_str(&std::fs::read_to_string(&inv_path)?)?;
        let prefab_rows: Vec<Value> = prefabs_doc["prefabs"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut classify = Classifier::new(&rules);
        let census = super::super::chunk_partitioner::road_census(&objects);
        let mut inv = rebuild_inventory(
            &committed_inv,
            &rules,
            &mut classify,
            &prefab_rows,
            &inst,
            census,
        );
        js_normalize(&mut inv);
        std::fs::write(
            out_objects.join("type-inventory.json"),
            serde_json::to_string_pretty(&inv)? + "\n",
        )?;
        println!(
            "reclassify:   type-inventory.json byKind/byBuildingClass/bySpeciesClass and \
             needsReview recomputed; generatedAt and each entry's instanceCount/reason preserved \
             (staging-derived, not reproducible from the repo)"
        );
    }
    println!("reclassify: WROTE {}", out_objects.display());
    // The rkyv twins must not go stale when classification is rewritten. The emitter
    // re-reads the JSON just written (same contract as build-world-objects).
    let terrain_for_rkyv = out_objects.parent().unwrap_or(out_objects.as_path());
    for (path, bytes) in catalog_emit::emit_catalog_archives(terrain_for_rkyv)? {
        println!("reclassify:   rkyv → {} ({bytes} bytes)", path.display());
    }
    Ok(0)
}

/// Resolve `--out` against the repo root so callers can stage a rebuild outside `packages/`.
#[must_use]
pub fn resolve_out_base(out: Option<&Path>) -> Option<PathBuf> {
    out.map(|p| {
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            repo_root().join(p)
        }
    })
}
