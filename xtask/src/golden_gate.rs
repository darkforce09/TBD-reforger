//! T-165.4 — the semantic golden gate S2–S9 + S11–S15 (port of
//! `packages/tbd-schema/scripts/verify-map-object-golden.mjs`). Shape validation (S1) lives in
//! `schema validate`; enum drift (S10) in `schema map-object-enums`. Uses the shared tbd-tools
//! compute libs (geometry/density/forest) — the same code the world builder + phase gates run.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::root::find_repo_root as repo_root;
use map_engine_core::world::binary::chunk_container::{CONTAINER_VERSION, HEADER_BYTES};
use map_engine_core::world::binary::pod::POD_BYTES;
use map_engine_core::world::{
    build_prefab_maps, narrow_prefab_rows, parse_chunk, parse_chunk_bin_for,
};
use tbd_tools::density::{
    DENSITY_CELL_M, DENSITY_CHANNELS, DENSITY_COLS, DENSITY_ROWS, TBDD_FILE_BYTES, TBDD_VERSION,
    accumulate_corners, slice_chunk_corners,
};
use tbd_tools::forest::{
    DENSITY_THRESHOLD, DOMINANT_SHARE, MIN_COMPONENT_CELLS, REGION_CELL_M, Tree,
    derive_forest_regions,
};
use tbd_tools::geometry::{cell_of, check_anchors, chunk_key};
use tbd_tools::world::binary_emit::{class_code_table, pods_from_rows, write_chunk_bin};

fn read_json(p: &PathBuf) -> Result<Value> {
    let raw = fs::read_to_string(p).with_context(|| format!("read {}", p.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", p.display()))
}

fn is_chunk_tuple(row: &Value) -> bool {
    row.as_array().and_then(|a| a.first()).map(Value::is_number) == Some(true)
}
fn inst_id(row: &Value) -> String {
    match row {
        Value::Array(a) => {
            if is_chunk_tuple(row) {
                format!(
                    "[{}]",
                    a.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else {
                a.first()
                    .map(|v| v.as_str().unwrap_or("?").to_string())
                    .unwrap_or_default()
            }
        }
        other => other["id"].as_str().unwrap_or("?").to_string(),
    }
}
fn inst_prefab_id(row: &Value) -> Option<f64> {
    match row {
        Value::Array(a) => {
            if is_chunk_tuple(row) {
                a.first().and_then(Value::as_f64)
            } else {
                a.get(1).and_then(Value::as_f64)
            }
        }
        other => other["prefabId"].as_f64(),
    }
}

/* ───────── S15 — the TBDC binary twin of the chunk golden (T-935.12, spec §2 / §3.1) ───────── */

/// One `WorldChunk` f32 column against another, by BITS: `==` calls `-0.0` equal to `+0.0`, and
/// `-0.0` is the exact value `binary_emit.rs` records the JSON round trip as not preserving.
fn f32_col(what: &str, bin: &[f32], json: &[f32], errs: &mut Vec<String>) {
    let (nb, nj) = (bin.len(), json.len());
    if nb != nj {
        errs.push(format!("{what}: bin {nb} values, json {nj}"));
        return;
    }
    for (i, (b, j)) in bin.iter().zip(json).enumerate() {
        if b.to_bits() != j.to_bits() {
            errs.push(format!("{what}[{i}]: bin {b} != json {j}"));
        }
    }
}

/// Every way `map-object-chunk-sample.bin` can disagree with `map-object-chunk-sample.json`.
///
/// **A** re-emits the golden through [`write_chunk_bin`] and demands byte identity — the golden is
/// emitter output, never hand-written, and this is the check one flipped byte fails. Three oracles
/// byte identity alone cannot give sit on top: **B** the shipped binary loader must decode to the
/// columns the shipped JSON loader reads out of the golden JSON, and neither of those is the
/// emitter; **C/D** a fixed-offset `from_le_bytes` decode of header AND rows, touching neither
/// `bytemuck` nor `TbdcHeader`/`ObjectInstancePod`, must agree — a field REORDER moves the emitter
/// and every cast-based reader together, so only raw offsets can see it; **E** every position must
/// lie inside the chunk the header itself declares, because a `.bin` framed for the wrong tile is
/// byte-perfect, parses cleanly, and files another tile's objects under this id forever.
fn chunk_bin_errors(sample: &Value, prefabs_sample: &Value, committed: &[u8]) -> Vec<String> {
    let mut errs: Vec<String> = Vec::new();
    let cx = sample["cx"].as_i64().unwrap_or(0);
    let cy = sample["cy"].as_i64().unwrap_or(0);
    let id = format!("{cx}_{cy}");
    let size_m = sample["chunkSizeM"].as_f64().unwrap_or(512.0);
    let raw = &sample["chunk"];
    let rows = raw["instances"].as_array().cloned().unwrap_or_default();
    let doc = json!({ "prefabs": prefabs_sample.clone() });
    let (by_id, _) = build_prefab_maps(narrow_prefab_rows(&doc));
    let Some(oracle) = parse_chunk(&id, raw, &by_id) else {
        return vec![format!("parse_chunk({id}) read no instances")];
    };
    let (n, len) = (oracle.count as usize, committed.len());

    // A. BYTE IDENTITY against a fresh emit from the golden JSON.
    let tmp = std::env::temp_dir().join(format!("t935-12-golden-{}.bin", std::process::id()));
    let pods = pods_from_rows(&rows, &class_code_table(&doc));
    let reemit = (|| -> Result<Vec<u8>> {
        write_chunk_bin(&tmp, cx, cy, &pods)?;
        fs::read(&tmp).map_err(anyhow::Error::from)
    })();
    let _ = fs::remove_file(&tmp);
    match reemit {
        Err(e) => errs.push(format!("re-emit failed: {e:#}")),
        Ok(want) => {
            let w = want.len();
            if w != len {
                errs.push(format!(".bin is {len} B, emitter writes {w} B"));
            }
            if let Some(o) = want.iter().zip(committed).position(|(a, b)| a != b) {
                let (c, w) = (committed[o], want[o]);
                errs.push(format!("byte {o}: {c:#04x} != emitter {w:#04x}"));
            }
        }
    }
    if len < HEADER_BYTES {
        errs.push(format!("{len} B, under the 32 B TBDC header"));
        return errs;
    }

    // C/D. Header, at fixed offsets, and its MEANING as well as its layout: rows that parse at the
    // right stride can still be framed for the wrong tile or by an unread container version.
    let u16at = |o: usize| u16::from_le_bytes([committed[o], committed[o + 1]]);
    let count = u32::from_le_bytes(committed[8..12].try_into().unwrap_or_default()) as usize;
    let (magic, rsv, ver, flags) = (&committed[..4], &committed[16..32], u16at(4), u16at(6));
    let (hcx, hcy) = (u16at(12) as i16, u16at(14) as i16);
    if magic != b"TBDC" {
        errs.push(format!("magic {magic:?}, want TBDC"));
    }
    if ver != CONTAINER_VERSION {
        errs.push(format!("containerVersion {ver} != {CONTAINER_VERSION}"));
    }
    if flags != 0 || rsv.iter().any(|b| *b != 0) {
        errs.push(format!("flags/reserved not zero: {flags}/{rsv:?}"));
    }
    if i64::from(hcx) != cx || i64::from(hcy) != cy {
        errs.push(format!("header frames {hcx}_{hcy}, JSON says {id}"));
    }
    if count != n {
        errs.push(format!("header count {count} != {n} JSON rows"));
    }
    let want_len = HEADER_BYTES + POD_BYTES * count;
    if len != want_len {
        errs.push(format!("file is {len} B, want {want_len}"));
    }
    let payload = &committed[HEADER_BYTES..];
    let names = ["x", "y", "z", "yaw", "pitch", "roll", "scale"];
    for i in 0..n.min(payload.len() / POD_BYTES) {
        let b = &payload[i * POD_BYTES..];
        let f = |o: usize| f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        let p = &oracle.positions;
        let got = [f(0), f(4), f(8), f(12), f(16), f(20), f(24)];
        let head = [p[2 * i], p[2 * i + 1], oracle.z[i], oracle.rotations[i]];
        let tail = [oracle.pitch[i], oracle.roll[i], oracle.scale[i]];
        for (k, (g, w)) in got.iter().zip(head.iter().chain(&tail)).enumerate() {
            let name = names[k];
            if g.to_bits() != w.to_bits() {
                errs.push(format!("row {i} {name}: bin {g} != json {w}"));
            }
        }
        let (pid, cls, pad) = (u16::from_le_bytes([b[28], b[29]]), b[30], b[31]);
        let (wpid, wcls) = (oracle.prefab_idx[i], oracle.cls_codes[i]);
        if pid != wpid {
            errs.push(format!("row {i} prefabId {pid} != json {wpid}"));
        }
        if cls != wcls {
            errs.push(format!("row {i} classCode {cls} != json {wcls}"));
        }
        if pad != 0 {
            errs.push(format!("row {i}: _pad is {pad}, must be 0"));
        }
    }

    // B. Shipped binary loader vs shipped JSON loader, column for column. E rides on it, because
    // `dec.cx`/`dec.cy` come from the HEADER — not from the file name, not from the golden JSON.
    match parse_chunk_bin_for(&id, committed) {
        Err(e) => errs.push(format!("parse_chunk_bin_for({id}): {e}")),
        Ok(dec) => {
            let (bid, bn, jn) = (&dec.id, dec.count, oracle.count);
            if bid != &oracle.id || bn != jn {
                errs.push(format!("bin {bid}/{bn}, json {id}/{jn}"));
            }
            f32_col("positions", &dec.positions, &oracle.positions, &mut errs);
            f32_col("z", &dec.z, &oracle.z, &mut errs);
            f32_col("rotations", &dec.rotations, &oracle.rotations, &mut errs);
            f32_col("pitch", &dec.pitch, &oracle.pitch, &mut errs);
            f32_col("roll", &dec.roll, &oracle.roll, &mut errs);
            f32_col("scale", &dec.scale, &oracle.scale, &mut errs);
            for (what, same) in [
                ("prefab_idx", dec.prefab_idx == oracle.prefab_idx),
                ("cls_codes", dec.cls_codes == oracle.cls_codes),
                ("rows_by_class", dec.rows_by_class == oracle.rows_by_class),
            ] {
                if !same {
                    errs.push(format!("{what}: bin decode != json decode"));
                }
            }
            let (hx, hy) = (dec.cx, dec.cy);
            for i in 0..dec.count as usize {
                let x = f64::from(dec.positions[2 * i]);
                let y = f64::from(dec.positions[2 * i + 1]);
                let (lo_x, lo_y) = (hx * size_m, hy * size_m);
                if x < lo_x || x >= lo_x + size_m || y < lo_y || y >= lo_y + size_m {
                    errs.push(format!("row {i} ({x},{y}) outside header {hx}_{hy}"));
                }
            }
        }
    }
    errs.truncate(8);
    errs
}

pub fn map_object_golden() -> Result<u8> {
    let root = repo_root()?;
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

    struct Gate {
        id: &'static str,
        label: &'static str,
        errs: Vec<String>,
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
                if let Some(pid) = inst_prefab_id(row) {
                    if let Some(p) = by_id.get(&pid.to_bits()) {
                        if p["kind"].as_str().unwrap_or("").is_empty()
                            || p["class"].as_str().unwrap_or("").is_empty()
                        {
                            errs.push(format!(
                                "{}: instance {} resolves to prefab without kind/class",
                                t.label,
                                inst_id(row)
                            ));
                        }
                    }
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

    // S11
    {
        let mut errs = Vec::new();
        let v_instance = jsonschema::validator_for(&read_json(
            &sroot.join("schema/map-object-instance.schema.json"),
        )?)
        .map_err(|e| anyhow::anyhow!("compile: {e}"))?;
        let cx = chunk_sample["cx"].as_f64().unwrap_or(0.0);
        let cy = chunk_sample["cy"].as_f64().unwrap_or(0.0);
        let chunk_size = chunk_sample["chunkSizeM"].as_f64().unwrap_or(512.0);
        let rows = arr(&chunk_sample["chunk"]["instances"]);
        if rows.is_empty() {
            errs.push("chunk-sample: empty instances".into());
        }
        let mut prev: Option<Vec<f64>> = None;
        let mut wide = 0usize;
        for (i, row) in rows.iter().enumerate() {
            // T-090.12.1 (objects schemaVersion 1.1.0): rows are exactly 5 or 8 numbers; an 8-wide row
            // must carry a non-trivial pitch / roll / scale (the converter writes trivial trailers
            // as a 5-wide row, so a padded row would be a canonicality bug in the emitter).
            let nums: Option<Vec<f64>> = row
                .as_array()
                .filter(|a| a.iter().all(Value::is_number))
                .map(|a| a.iter().filter_map(Value::as_f64).collect());
            let Some(t) = nums.filter(|t| t.len() == 5 || t.len() == 8) else {
                errs.push(format!(
                    "chunk-sample[{i}]: not an all-number 5- or 8-tuple"
                ));
                continue;
            };
            if t.len() == 8 {
                wide += 1;
                if t[5] == 0.0 && t[6] == 0.0 && t[7] == 1.0 {
                    errs.push(format!(
                        "chunk-sample[{i}]: 8-wide row with trivial pitch/roll/scale (must be 5-wide)"
                    ));
                }
            }
            if v_instance.iter_errors(row).next().is_some() {
                errs.push(format!("chunk-sample[{i}]: schema invalid"));
            }
            let (x, y) = (t[1], t[2]);
            if x < cx * chunk_size || x >= (cx + 1.0) * chunk_size {
                errs.push(format!(
                    "chunk-sample[{i}]: x {x} outside [{}, {})",
                    cx * chunk_size,
                    (cx + 1.0) * chunk_size
                ));
            }
            if y < cy * chunk_size || y >= (cy + 1.0) * chunk_size {
                errs.push(format!(
                    "chunk-sample[{i}]: y {y} outside [{}, {})",
                    cy * chunk_size,
                    (cy + 1.0) * chunk_size
                ));
            }
            if let Some(p) = &prev {
                let sorted = p[1] < x || (p[1] == x && (p[2] < y || (p[2] == y && p[0] <= t[0])));
                if !sorted {
                    errs.push(format!(
                        "chunk-sample[{i}]: rows not sorted by (x, y, prefabId)"
                    ));
                }
            }
            prev = Some(t);
        }
        if wide == 0 {
            errs.push(
                "chunk-sample: no 8-wide row (T-090.12.1 full-transform branch needs golden coverage)"
                    .into(),
            );
        }
        if !arr(&prefabs_sample)
            .iter()
            .any(|p| p["render"]["importanceZoom"].is_number())
        {
            errs.push("prefabs-sample: no prefab carries render.importanceZoom (T-090.3.1 bump needs golden coverage)".into());
        }
        gates.push(Gate {
            id: "S11",
            label: "chunk golden — 5/8-tuple rows, canonical trailers, bounds, sort order, importanceZoom coverage",
            errs,
        });
    }

    // S12
    {
        let mut errs = Vec::new();
        let world = anchor_fixture["worldSizeM"].as_f64().unwrap_or(0.0);
        let chunk_size = anchor_fixture["chunkSizeM"].as_f64().unwrap_or(512.0);
        let raw = arr(&anchor_fixture["rawEntities"]);
        let expected = &anchor_fixture["expected"];
        let prefabs = arr(&expected["prefabs"]);
        let building_anchors: Vec<Value> = raw
            .iter()
            .filter(|r| {
                let rn = r["resourceName"].as_str().unwrap_or("");
                !rn.is_empty()
                    && prefabs
                        .iter()
                        .any(|p| p["resourceName"] == rn && p["kind"] == "building")
            })
            .cloned()
            .collect();
        if building_anchors.is_empty() {
            errs.push("anchor-fixture: no building anchors".into());
        }
        let chunks = expected["chunks"].clone();
        errs.extend(check_anchors(
            &building_anchors,
            &prefabs,
            |cx, cy| {
                chunks
                    .get(chunk_key(cx, cy))
                    .filter(|v| !v.is_null())
                    .cloned()
            },
            chunk_size,
            world,
            2.0,
        ));
        let mut expected_total = 0usize;
        for (key, chunk) in chunks.as_object().into_iter().flatten() {
            for row in chunk["instances"].as_array().into_iter().flatten() {
                expected_total += 1;
                let a = arr(row);
                let k = chunk_key(
                    cell_of(
                        a.get(1).and_then(Value::as_f64).unwrap_or(0.0),
                        chunk_size,
                        world,
                    ),
                    cell_of(
                        a.get(2).and_then(Value::as_f64).unwrap_or(0.0),
                        chunk_size,
                        world,
                    ),
                );
                if &k != key {
                    errs.push(format!(
                        "anchor-fixture chunk {key}: row {row} partitions to {k}"
                    ));
                }
            }
        }
        if expected_total != building_anchors.len() {
            errs.push(format!("anchor-fixture: expected chunks hold {expected_total} instances, raw has {} building rows (exclusion rule broken)", building_anchors.len()));
        }
        gates.push(Gate {
            id: "S12",
            label: "anchor fixture — shared checkAnchors PASS + partition consistency + exclusions",
            errs,
        });
    }

    // S13
    {
        let mut errs = Vec::new();
        let world = density_fixture["worldSizeM"].as_f64().unwrap_or(0.0);
        let ccx = density_fixture["chunk"]["cx"].as_u64().unwrap_or(0) as usize;
        let ccy = density_fixture["chunk"]["cy"].as_u64().unwrap_or(0) as usize;
        let pos = |key: &str| -> Vec<(f64, f64)> {
            arr(&density_fixture[key])
                .iter()
                .filter_map(|r| Some((r["x"].as_f64()?, r["y"].as_f64()?)))
                .collect()
        };
        let (t_grid, t_size) = accumulate_corners(pos("treePositions").into_iter(), world);
        let (r_grid, r_size) = accumulate_corners(pos("rockPositions").into_iter(), world);
        let t_slice = slice_chunk_corners(&t_grid, t_size, ccx, ccy);
        let r_slice = slice_chunk_corners(&r_grid, r_size, ccx, ccy);
        let rebuilt = map_engine_core::geometry::tbdd::encode_tbdd(
            DENSITY_CELL_M,
            DENSITY_COLS,
            DENSITY_ROWS,
            &[&t_slice, &r_slice],
        );
        let expected_bytes = density_fixture["expectedFileBytes"].as_u64().unwrap_or(0) as usize;
        if expected_bytes != TBDD_FILE_BYTES {
            errs.push(format!("fixture expectedFileBytes {expected_bytes} != lib TBDD_FILE_BYTES {TBDD_FILE_BYTES}"));
        }
        if density_bin.len() != TBDD_FILE_BYTES {
            errs.push(format!(
                "committed bin {} bytes, want {TBDD_FILE_BYTES}",
                density_bin.len()
            ));
        }
        if rebuilt != density_bin {
            errs.push("encode(fixture) != committed density-fixture.bin".into());
        }
        match map_engine_core::geometry::tbdd::decode_tbdd(&density_bin) {
            Ok(dec) => {
                if dec.version != TBDD_VERSION
                    || dec.cell_m != DENSITY_CELL_M
                    || dec.cols != DENSITY_COLS
                    || dec.rows != DENSITY_ROWS
                    || dec.channels.len() != DENSITY_CHANNELS.len()
                {
                    errs.push(format!(
                        "decoded header mismatch: v={} cellM={} cols={} rows={} ch={}",
                        dec.version,
                        dec.cell_m,
                        dec.cols,
                        dec.rows,
                        dec.channels.len()
                    ));
                }
                let mut sparse: HashMap<(u64, u64), (u64, u64)> = HashMap::new();
                for e in arr(&density_fixture["expectedCorners"]) {
                    sparse.insert(
                        (e["i"].as_u64().unwrap_or(0), e["j"].as_u64().unwrap_or(0)),
                        (
                            e["tree"].as_u64().unwrap_or(0),
                            e["rock"].as_u64().unwrap_or(0),
                        ),
                    );
                }
                let cols = usize::from(DENSITY_COLS);
                for j in 0..usize::from(DENSITY_ROWS) {
                    for i in 0..cols {
                        let (etree, erock) =
                            sparse.get(&(i as u64, j as u64)).copied().unwrap_or((0, 0));
                        let tree = u64::from(dec.channels[0][j * cols + i]);
                        let rock = u64::from(dec.channels[1][j * cols + i]);
                        if tree != etree || rock != erock {
                            errs.push(format!("corner ({i},{j}): decoded tree={tree}/rock={rock}, expected tree={etree}/rock={erock}"));
                        }
                    }
                }
            }
            Err(e) => errs.push(format!("decode failed: {e}")),
        }
        errs.truncate(8);
        gates.push(Gate {
            id: "S13",
            label: "TBDD density fixture — encode byte-identity, header contract, expected corners",
            errs,
        });
    }

    // S14
    {
        let mut errs = Vec::new();
        let world = region_fixture["worldSizeM"].as_f64().unwrap_or(0.0);
        let terrain = region_fixture["terrainId"].as_str().unwrap_or("everon");
        let trees: Vec<Tree> = arr(&region_fixture["trees"])
            .iter()
            .filter_map(|t| {
                Some(Tree {
                    x: t["x"].as_f64()?,
                    y: t["y"].as_f64()?,
                    class: t["class"].as_str()?.to_string(),
                })
            })
            .collect();
        let res = derive_forest_regions(&trees, world, terrain);
        let exp = &region_fixture["expected"];
        let params = json!({
            "cellM": REGION_CELL_M as i64, "densityThreshold": DENSITY_THRESHOLD,
            "minComponentCells": MIN_COMPONENT_CELLS, "dominantShare": DOMINANT_SHARE,
        });
        if params != exp["params"] {
            errs.push(format!("params drift: {params} != {}", exp["params"]));
        }
        if res.unassigned_trees != exp["unassignedTrees"].as_u64().unwrap_or(u64::MAX) {
            errs.push(format!(
                "unassignedTrees {} != expected {}",
                res.unassigned_trees, exp["unassignedTrees"]
            ));
        }
        let derived = Value::Array(res.regions.clone());
        if derived != exp["regions"] {
            errs.push("derived regions differ from expected (rings or aggregates)".into());
        }
        let sum: u64 = res
            .regions
            .iter()
            .map(|r| r["treeCount"].as_u64().unwrap_or(0))
            .sum::<u64>()
            + res.unassigned_trees;
        if sum != trees.len() as u64 {
            errs.push(format!(
                "F2 identity on fixture: {sum} != {} trees",
                trees.len()
            ));
        }
        if !res
            .regions
            .iter()
            .any(|r| r["polygon"].as_array().map(Vec::len).unwrap_or(0) > 1)
        {
            errs.push("fixture no longer exercises a hole ring".into());
        }
        if !res
            .regions
            .iter()
            .any(|r| r["dominantSpeciesClass"] == "mixed")
        {
            errs.push("fixture no longer exercises the mixed dominant rule".into());
        }
        gates.push(Gate { id: "S14", label: "forest-region derivation fixture — deterministic rings + aggregates + F2 identity", errs });
    }

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
