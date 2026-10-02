use super::*;
use crate::repository_layout::{export_operations_log, map_scratch_dir};

/// The export stage this gate accepts an operations log from: the subregion spike, whose log
/// carries the K-gate verdicts and the sampled rows the checks below read.
const SPIKE_SLICE: &str = "spike-subregion-export";

pub fn verify_spike_ops_log(terrain: &str) -> Result<u8> {
    let root = repo_root();
    let ops_path = export_operations_log(&root, terrain);
    let staging = map_scratch_dir(&root, terrain).join("spike");
    let raw_path = staging.join("raw-entities.jsonl");
    if !ops_path.exists() {
        eprintln!(
            "verify-spike-ops-log: FAIL — ops log not found: {}",
            ops_path.display()
        );
        return Ok(1);
    }
    let ops: Value = match serde_json::from_str(&std::fs::read_to_string(&ops_path)?) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("verify-spike-ops-log: FAIL — ops log is not valid JSON: {e}");
            return Ok(1);
        }
    };
    let mut fail: Vec<String> = Vec::new();
    let is_str = |v: &Value| v.as_str().is_some_and(|s| !s.is_empty());
    let size_gt0 =
        |p: &Path| p.exists() && std::fs::metadata(p).map(|m| m.len() > 0).unwrap_or(false);
    let resolve_artifact = |p: &Value| -> Option<PathBuf> {
        let s = p.as_str()?;
        if s.is_empty() {
            return None;
        }
        let cands = [
            if Path::new(s).is_absolute() {
                Some(PathBuf::from(s))
            } else {
                None
            },
            Some(root.join(s)),
            Some(staging.join(s)),
        ];
        for c in cands.into_iter().flatten() {
            if c.exists() {
                return Some(c);
            }
        }
        Some(if Path::new(s).is_absolute() {
            PathBuf::from(s)
        } else {
            root.join(s)
        })
    };

    for k in [
        "schemaVersion",
        "terrainId",
        "slice",
        "generatedAt",
        "subregionBBoxM",
        "probes",
        "gates",
        "handednessRemap",
        "forestSource",
        "tileFindings",
        "sampleRows",
        "mcpToolsUsed",
    ] {
        if ops.get(k).is_none() {
            fail.push(format!("missing required key: {k}"));
        }
    }
    if ops.get("terrainId").is_some() && ops["terrainId"] != terrain {
        fail.push(format!("terrainId {} !== {terrain}", ops["terrainId"]));
    }
    if ops.get("slice").is_some() && ops["slice"] != SPIKE_SLICE {
        fail.push(format!("slice {} !== {SPIKE_SLICE}", ops["slice"]));
    }
    if ops.get("subregionBBoxM").is_some()
        && !(ops["subregionBBoxM"]
            .as_array()
            .is_some_and(|a| a.len() == 4 && a.iter().all(is_finite)))
    {
        fail.push("subregionBBoxM must be [minX,minY,maxX,maxY] finite numbers".into());
    }

    let gates = &ops["gates"];
    for g in ["K1", "K1b", "K2", "K3", "K4", "K5", "K6", "K7"] {
        if gates[g] != "pass" && gates[g] != "fail" {
            fail.push(format!(
                "gates.{g} must be \"pass\" or \"fail\" (lowercase), got {}",
                gates[g]
            ));
        }
    }
    let is_pass = |g: &str| gates[g] == "pass";

    if is_pass("K6") {
        let h = &ops["handednessRemap"];
        if !is_str(&h["enfusionBasis"]) {
            fail.push("K6 pass requires handednessRemap.enfusionBasis non-empty".into());
        }
        if !is_str(&h["editorToExport"]) {
            fail.push("K6 pass requires handednessRemap.editorToExport non-empty".into());
        }
        if h["sampleEntity"]
            .as_object()
            .is_none_or(serde_json::Map::is_empty)
        {
            fail.push("K6 pass requires handednessRemap.sampleEntity non-empty".into());
        }
    }
    if is_pass("K5") {
        if ops["forestSource"] != "engine-mask" && ops["forestSource"] != "derived-hull-mandated" {
            fail.push(format!(
                "K5 pass requires forestSource ∈ {{engine-mask, derived-hull-mandated}}, got {}",
                ops["forestSource"]
            ));
        }
        let ev = if !ops["probes"]["S5"]["evidence"].is_null() {
            &ops["probes"]["S5"]["evidence"]
        } else {
            &ops["probes"]["S5"]["note"]
        };
        if !is_str(ev) {
            fail.push("K5 pass requires probes.S5.evidence (MCP citation) non-empty".into());
        }
    }

    let rules = load_rules()?;
    let raw_entries = if raw_path.exists() {
        Some(classified_rows(&rules, &raw_path)?)
    } else {
        None
    };
    let sample_rows = ops["sampleRows"].as_array().cloned().unwrap_or_default();
    if !ops["sampleRows"].is_array() || sample_rows.len() != 3 {
        fail.push(format!(
            "sampleRows must be exactly 3 (got {})",
            if ops["sampleRows"].is_array() {
                sample_rows.len().to_string()
            } else {
                "non-array".into()
            }
        ));
    }
    match &raw_entries {
        Some(entries) => {
            for (i, s) in sample_rows.iter().enumerate() {
                if !is_str(&s["resourceName"]) || !["x", "y", "z"].iter().all(|k| is_finite(&s[*k]))
                {
                    fail.push(format!("sampleRows[{i}] needs resourceName + finite x,y,z"));
                    continue;
                }
                let hit = entries.iter().any(|(r, _, _, _)| {
                    r["resourceName"] == s["resourceName"]
                        && (r["x"].as_f64().unwrap_or(f64::MAX) - s["x"].as_f64().unwrap()).abs()
                            <= 0.001
                        && (r["y"].as_f64().unwrap_or(f64::MAX) - s["y"].as_f64().unwrap()).abs()
                            <= 0.001
                        && (r["z"].as_f64().unwrap_or(f64::MAX) - s["z"].as_f64().unwrap()).abs()
                            <= 0.001
                });
                if !hit {
                    fail.push(format!(
                        "sampleRows[{i}] ({}) does not resolve to any raw-entities.jsonl line within 0.001 m",
                        s["resourceName"].as_str().unwrap_or("")
                    ));
                }
            }
        }
        None if !sample_rows.is_empty() => {
            fail.push(format!(
                "cannot resolve sampleRows — raw-entities.jsonl missing: {}",
                raw_path.display()
            ));
        }
        None => {}
    }

    if is_pass("K2") {
        let real_obb = raw_entries.as_ref().is_some_and(|entries| {
            entries.iter().any(|(r, kind, _, _)| {
                kind == "building"
                    && r["halfExtentsM"]
                        .as_array()
                        .is_some_and(|a| a.len() == 3 && a.iter().all(is_finite))
            })
        });
        let kind_default = ops["probes"]["S2"]["obbDecision"] == "kind-default"
            && is_str(&ops["probes"]["S2"]["mcpEvidence"]);
        if !real_obb && !kind_default {
            fail.push("K2 pass requires a building row with numeric halfExtentsM[3] OR probes.S2.obbDecision==='kind-default' + probes.S2.mcpEvidence".into());
        }
    }

    let sat = &ops["tileFindings"]["satellite"];
    let sat_file = resolve_artifact(&sat["path"]);
    if is_pass("K3") {
        if !is_str(&sat["path"]) || !sat_file.as_deref().is_some_and(size_gt0) {
            fail.push(format!(
                "K3 pass requires tileFindings.satellite.path to be a >0-byte file (looked at {})",
                sat_file
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            ));
        }
    } else if gates["K3"] == "fail" {
        if is_str(&sat["path"]) && sat_file.as_deref().is_some_and(size_gt0) {
            fail.push("K3 fail but a satellite tile file is present — inconsistent".into());
        } else if !(sat["escalate"] == true && is_str(&sat["evidence"])) {
            fail.push("K3 fail requires no tile OR tileFindings.satellite.escalate===true with non-empty evidence".into());
        }
    }

    let map_t = &ops["tileFindings"]["map"];
    let map_file = resolve_artifact(&map_t["path"]);
    let n9 = "synthesized-cartographic required";
    let has_n9 = map_t["synthesizedCartographicRequired"] == true
        && (map_t["note"].as_str().unwrap_or("").contains(n9)
            || ops["probes"]["S4"]["note"]
                .as_str()
                .unwrap_or("")
                .contains(n9));
    if is_pass("K4") {
        let has_tile = is_str(&map_t["path"]) && map_file.as_deref().is_some_and(size_gt0);
        if !has_tile && !has_n9 {
            fail.push(format!("K4 pass requires a >0-byte map tile OR synthesizedCartographicRequired + literal \"{n9}\" note"));
        }
    } else if gates["K4"] == "fail"
        && is_str(&map_t["path"])
        && map_file.as_deref().is_some_and(size_gt0)
    {
        fail.push("K4 fail but a map tile file is present — inconsistent".into());
    }

    if !fail.is_empty() {
        eprintln!("verify-spike-ops-log: FAIL ({})", fail.len());
        for f in &fail {
            eprintln!("  {f}");
        }
        return Ok(1);
    }
    println!("verify-spike-ops-log: OK (K7 + K2/K3/K4 gate↔artifact)");
    Ok(0)
}
