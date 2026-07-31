//! T-278 — regenerate the prefab catalogue's **classification lane** from committed artifacts
//! alone: no Enfusion Workbench run, no hand-copied staging, no game install.
//!
//! THE PROBLEM THIS EXISTS FOR
//! ---------------------------
//! `packages/tbd-schema/rules/prefab-classify.json` says it in its own description: *"EDITING
//! THIS FILE CHANGES NOTHING UNTIL THE CATALOGUE IS REBUILT."* The only rebuild path was
//! `world build-objects`, which hard-requires `packages/map-assets/<terrain>/staging/export/
//! raw-entities.jsonl` — a ~1.2M-row Workbench export that is **gitignored** (`.gitignore:18`)
//! and absent from every clone. So a rule edit is unverifiable and unshippable by anyone who is
//! not sitting in front of Workbench, and `make map-export` exits 2 for everybody else.
//!
//! T-244 is the measured consequence: it added a `vehicle` kind plus wreck rules, the gates went
//! green against the *rules*, and the shipped `prefabs.json.gz` never changed. Its own agent
//! disclosed the change was latent. It stayed latent.
//!
//! WHAT THIS DOES, AND WHAT IT HONESTLY CANNOT
//! -------------------------------------------
//! Classification is a pure function of `resourceName` (`classify.rs`: first rule whose
//! `match.resourceNameContains` substring hits, case-sensitive, file order = priority). Every
//! `resourceName` already lives in the committed `objects/prefabs.json.gz`, and every instance
//! count is recoverable from the committed `objects/chunks/*.json.gz`. So the whole
//! classification lane — `kind`, `class`, `ai`, `gameplay`, `render`, `tags` — plus the census
//! counts derived from it are reproducible from the repo. That is what this rebuilds.
//!
//! It does NOT invent placements. `spatial.halfExtentsM` is measured from engine halfExtents
//! sampled during the Workbench export, and those samples are not in the repo, so a measured
//! `spatial` is **preserved verbatim**. A row whose committed `spatial` is byte-equal to the
//! template of the rule that produced it was a template fallback, not a measurement, so it is
//! re-templated from the new rule (see `respatialize`). `needsReview.prefabs[]` counts raw rows
//! that were excluded from the catalogue entirely and is likewise not derivable here; it is
//! preserved and reported as staging-derived rather than silently rewritten to a subset.
//!
//! Default mode is CHECK: read-only, exit 1 on drift. That is the gate this repo did not have —
//! run on the day T-244 landed it would have gone red with the 16 rows it was about to strand.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

use super::build::{gunzip, gz9};
use super::classify::{Classifier, Rules, load_rules};
use super::jsval::js_normalize;
use crate::serve::repo_root;

/// One prefab whose classification the current rules disagree with the committed catalogue on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drift {
    pub prefab_id: u64,
    pub resource_name: String,
    pub old_kind: String,
    pub old_class: String,
    pub new_kind: String,
    pub new_class: String,
}

/// What a reclassify pass found. `kinds_*` are ordered histograms for the report.
#[derive(Debug)]
pub struct Report {
    pub rows: usize,
    pub matched: usize,
    pub drift: Vec<Drift>,
    pub kinds_before: BTreeMap<String, u64>,
    pub kinds_after: BTreeMap<String, u64>,
    pub unclassified_before: u64,
    pub unclassified_after: u64,
    /// Kinds present after the pass that no committed row carries — the new census buckets a
    /// consumer schema has to accept before the artifact can land.
    pub new_kinds: Vec<String>,
}

impl Report {
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.drift.is_empty()
    }
}

fn obj_get<'a>(v: &'a Value, k: &str) -> Option<&'a Value> {
    v.as_object().and_then(|m| m.get(k))
}

/// The rule that produced a committed row, identified by its `kind`+`class` pair — the same
/// lookup `build.rs` uses to recover `render.importanceZoom` for a census bucket. `None` when
/// the pair is the fallback's (or no rule claims it), which is itself the answer: the fallback.
fn rule_for_kind_class<'a>(rules: &'a Rules, kind: &str, class: &str) -> &'a Value {
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
/// `build.rs` writes `spatial` from per-axis medians of sampled engine halfExtents when it has
/// samples, and copies the matched rule's `spatial` template when it does not. Only the second
/// case is reproducible here, and it is detectable: a committed `spatial` byte-equal to its
/// producing rule's template was a template. Anything else is a measurement that exists nowhere
/// else in the repo, so it is preserved.
///
/// The one indistinguishable case — a measurement that happens to equal its own template — is
/// re-templated. That is only observable when the row also changes kind, and it swaps one
/// hand-authored template for another rather than destroying a measurement.
fn respatialize(committed: &Value, old_rule: &Value, new_rule: &Value) -> Value {
    if committed == &old_rule["spatial"] {
        new_rule["spatial"].clone()
    } else {
        committed.clone()
    }
}

/// Rebuild one prefab row from the rules, preserving identity (`prefabId`, `resourceName`,
/// `label`) and measured geometry. Key order matches `build.rs`'s emitted row exactly —
/// serde_json is built with `preserve_order`, so this is a byte-level contract, not cosmetics.
fn rebuild_row(rules: &Rules, classify: &mut Classifier, row: &Value) -> (Value, Option<Drift>) {
    let rn = row["resourceName"].as_str().unwrap_or_default().to_string();
    let old_kind = row["kind"].as_str().unwrap_or_default().to_string();
    let old_class = row["class"].as_str().unwrap_or_default().to_string();
    let cls = classify.classify(&rn);
    let new_rule = rules.rule(cls.rule_idx).clone();
    let old_rule = rule_for_kind_class(rules, &old_kind, &old_class).clone();

    let mut ai = Map::new();
    ai.insert("summary".into(), new_rule["ai"]["summary"].clone());
    ai.insert("taxonomyPath".into(), new_rule["ai"]["taxonomyPath"].clone());
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

    // T-537/T-383 non-vacuity guard, and the one that catches a gutted rules file: a rules doc
    // that classifies nothing would silently re-stamp every row as the fallback `prop/unknown`
    // and report a large, confident, wrong drift. Refuse instead.
    super::refuse_empty_write(
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
/// `build.rs` gets from the raw stream and that we get from the artifact it wrote.
fn instances_by_prefab(chunks_dir: &Path, n_prefabs: usize) -> Result<Vec<u64>> {
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
    super::refuse_empty_write(
        "reclassify census",
        chunk_files == 0 || counts.iter().all(|c| *c == 0),
        "no committed chunk instances found — census would be vacuously zero",
    )?;
    Ok(counts)
}

/// Recompute the count lanes of `type-inventory.json` from the rebuilt rows, preserving every
/// other key and its order. Mirrors `build.rs`'s census block.
fn rebuild_inventory(committed: &Value, rules: &Rules, prefabs: &[Value], inst: &[u64]) -> Value {
    let mut by_kind: Map<String, Value> = super::INSTANCE_KINDS
        .iter()
        .map(|k| {
            let mut m = Map::from_iter([
                ("prefabTypes".to_string(), json!(0)),
                ("instances".to_string(), json!(0)),
            ]);
            if *k == "road" {
                // Roads come from `.topo`, not the prefab lane; carry the committed value.
                m.insert(
                    "segments".into(),
                    committed["byKind"]["road"]["segments"].clone(),
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
        Value::Object(keys.into_iter().map(|k| (k.clone(), m[&k].clone())).collect())
    };

    let mut out = committed.as_object().cloned().unwrap_or_default();
    out.insert("byKind".into(), Value::Object(by_kind));
    out.insert("byBuildingClass".into(), sorted(by_building));
    out.insert("bySpeciesClass".into(), sorted(by_species));
    Value::Object(out)
}

/// `--write` writes the artifacts; the default reports and touches nothing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Check,
    Write,
}

/// `world reclassify` — see the module docs. Returns the process exit code: 0 = the committed
/// catalogue already agrees with the rules (or the rebuild was written), 1 = drift in check mode.
pub fn reclassify_terrain(terrain: &str, mode: Mode, out_base: Option<&Path>) -> Result<u8> {
    let terrain_dir = repo_root().join("packages/map-assets").join(terrain);
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
        let prefab_rows: Vec<Value> = prefabs_doc["prefabs"].as_array().cloned().unwrap_or_default();
        let mut inv = rebuild_inventory(&committed_inv, &rules, &prefab_rows, &inst);
        js_normalize(&mut inv);
        std::fs::write(
            out_objects.join("type-inventory.json"),
            serde_json::to_string_pretty(&inv)? + "\n",
        )?;
        println!(
            "reclassify:   type-inventory.json byKind/byBuildingClass/bySpeciesClass recomputed; \
             needsReview + generatedAt preserved (staging-derived, not reproducible from the repo)"
        );
    }
    println!("reclassify: WROTE {}", out_objects.display());
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

#[cfg(test)]
mod tests {
    use super::{Rules, reclassify_rows, rule_for_kind_class};
    use serde_json::{Value, json};

    fn rules(extra: Vec<Value>) -> Rules {
        let mut rs = vec![json!({
            "kind": "building", "class": "hut",
            "match": { "resourceNameContains": ["/Hut"] },
            "ai": { "summary": "a hut", "taxonomyPath": "building/hut", "confidence": 0.9 },
            "spatial": { "model": "obb", "pivot": "center" },
            "gameplay": { "cover": { "type": "full" } },
            "render": { "iconKey": "hut" }
        })];
        rs.extend(extra);
        Rules {
            doc: json!({
                "rules": rs,
                "fallback": {
                    "kind": "prop", "class": "unknown",
                    "ai": { "summary": "?", "taxonomyPath": "prop/unknown" },
                    "spatial": { "model": "obb", "pivot": "center", "fallback": true },
                    "gameplay": {}
                }
            }),
        }
    }

    fn wreck_rule() -> Value {
        json!({
            "kind": "vehicle", "class": "armor",
            "match": { "resourceNameContains": ["Props/Wrecks/"] },
            "ai": { "summary": "a wreck", "taxonomyPath": "vehicle/armor", "confidence": 0.8 },
            "spatial": { "model": "obb", "pivot": "center", "wreck": true },
            "gameplay": {}
        })
    }

    fn committed() -> Vec<Value> {
        vec![
            json!({
                "prefabId": 0, "resourceName": "{A}Prefabs/Hut.et",
                "kind": "building", "class": "hut", "label": "Hut",
                "ai": { "needsReview": false },
                "spatial": { "model": "obb", "pivot": "center", "halfExtentsM": { "x": 4.0 } },
                "gameplay": { "cover": { "type": "full" } }
            }),
            json!({
                "prefabId": 1, "resourceName": "{B}Prefabs/Props/Wrecks/T62.et",
                "kind": "prop", "class": "unknown", "label": "T62",
                "ai": { "needsReview": true },
                "spatial": { "model": "obb", "pivot": "center", "fallback": true },
                "gameplay": {}
            }),
        ]
    }

    /// GREEN: rules unchanged since the catalogue was built → no drift, nothing to do.
    #[test]
    fn no_drift_when_rules_match_the_catalogue() {
        let (rows, rep) = reclassify_rows(&rules(vec![]), &committed()).expect("ok");
        assert!(rep.is_clean(), "unexpected drift: {:?}", rep.drift);
        assert_eq!(rows.len(), 2);
        assert_eq!(rep.unclassified_before, 1);
        assert_eq!(rep.unclassified_after, 1);
        assert!(rep.new_kinds.is_empty());
    }

    /// RED: this is T-244's exact shape — append a rule, and the committed artifact is stale.
    #[test]
    fn appending_a_rule_reports_drift_and_a_new_census_bucket() {
        let (rows, rep) = reclassify_rows(&rules(vec![wreck_rule()]), &committed()).expect("ok");
        assert_eq!(rep.drift.len(), 1, "{:?}", rep.drift);
        let d = &rep.drift[0];
        assert_eq!((d.prefab_id, &*d.old_kind, &*d.new_kind), (1, "prop", "vehicle"));
        assert_eq!(rows[1]["kind"], json!("vehicle"));
        assert_eq!(rows[1]["class"], json!("armor"));
        assert_eq!(rep.new_kinds, vec!["vehicle".to_string()]);
        assert_eq!(rep.unclassified_after, 0, "the wreck is no longer unknown");
        // Identity survives a reclassification; only the classification lane moves.
        assert_eq!(rows[1]["prefabId"], json!(1));
        assert_eq!(rows[1]["label"], json!("T62"));
        assert_eq!(rows[1]["ai"]["needsReview"], json!(false));
    }

    /// A measured OBB is not recoverable from the repo, so it must survive a kind change.
    #[test]
    fn measured_spatial_is_preserved_and_template_spatial_is_reclaimed() {
        let (rows, _) = reclassify_rows(&rules(vec![wreck_rule()]), &committed()).expect("ok");
        // Row 0 carries a measured halfExtents → untouched even though the rule has no such key.
        assert_eq!(rows[0]["spatial"]["halfExtentsM"]["x"], json!(4.0));
        // Row 1's committed spatial IS the fallback template → re-templated from the new rule.
        assert_eq!(rows[1]["spatial"]["wreck"], json!(true));
        assert!(rows[1]["spatial"].get("fallback").is_none());
    }

    /// The house defect, refused: an empty rules file classifies nothing, which would re-stamp
    /// every row `prop/unknown` and report a confident, enormous, wrong drift.
    #[test]
    fn empty_rules_are_refused_not_reported_as_massive_drift() {
        let empty = Rules {
            doc: json!({ "rules": [], "fallback": { "kind": "prop", "class": "unknown" } }),
        };
        let err = reclassify_rows(&empty, &committed()).expect_err("must refuse");
        let msg = format!("{err:#}");
        assert!(msg.contains("refusing empty write (reclassify catalogue)"), "{msg}");
        assert!(msg.contains("no prefab matched any rule"), "{msg}");
    }

    #[test]
    fn empty_catalogue_is_refused() {
        let err = reclassify_rows(&rules(vec![]), &[]).expect_err("must refuse");
        assert!(format!("{err:#}").contains("zero prefab rows"), "{err:#}");
    }

    #[test]
    fn rule_lookup_falls_back_when_no_rule_owns_the_pair() {
        let r = rules(vec![]);
        assert_eq!(rule_for_kind_class(&r, "building", "hut")["class"], json!("hut"));
        assert_eq!(
            rule_for_kind_class(&r, "prop", "unknown")["spatial"]["fallback"],
            json!(true),
            "an unclaimed kind/class pair must resolve to the fallback template"
        );
    }
}
