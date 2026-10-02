use super::*;

/// Validate one mission JSON file (or stdin with `-`).
/// (schema + the 1.1 ORBAT-count/slot-id checks; the deploy-staging V1 gate).
pub fn validate_file(target: &str) -> Result<u8> {
    let raw = if target == "-" {
        use std::io::Read;
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        s
    } else {
        fs::read_to_string(target).with_context(|| target.to_string())?
    };
    let Ok(data) = serde_json::from_str::<Value>(&raw) else {
        eprintln!("invalid JSON");
        return Ok(1);
    };

    let root = repo_root()?;
    let schema = read_json(&definition_path(&root, "mission.schema.json"))?;
    // Whole-document byte ceiling (mirrors TBD_MissionLoader.MISSION_FILE_MAX_BYTES).
    // Prefer the schema keyword so a drifted constant here fails closed rather than silently
    // accepting an oversized file that the mod would refuse.
    let max_bytes = schema["x-tbd-missionFileMaxBytes"]
        .as_u64()
        .map(|n| n as usize)
        .unwrap_or(8 * 1024 * 1024);
    let raw_bytes = raw.as_bytes();
    if raw_bytes.len() > max_bytes {
        eprintln!(
            "/: document exceeds MISSION_FILE_MAX_BYTES ({} B > {} B) — \
             TBD_MissionLoader.c LoadFromProfileFile would refuse this file",
            raw_bytes.len(),
            max_bytes
        );
        return Ok(1);
    }
    let validator =
        jsonschema::validator_for(&schema).map_err(|e| anyhow::anyhow!("schema compile: {e}"))?;
    let errs: Vec<String> = validator
        .iter_errors(&data)
        .map(|e| {
            let p = e.instance_path().to_string();
            format!("{} {e}", if p.is_empty() { "/".to_string() } else { p })
        })
        .collect();
    if !errs.is_empty() {
        for e in errs {
            eprintln!("{e}");
        }
        return Ok(1);
    }

    if data["schemaVersion"] == "1.1" {
        let mut expected: i64 = 0;
        for faction in data["orbat"]
            .as_object()
            .map(|m| m.values())
            .into_iter()
            .flatten()
        {
            for group in faction["groups"].as_array().into_iter().flatten() {
                for role in group["roles"].as_array().into_iter().flatten() {
                    expected += role["count"].as_i64().unwrap_or(0);
                }
            }
        }
        let slots = data["slots"].as_array().cloned().unwrap_or_default();
        if slots.len() as i64 != expected {
            eprintln!(
                "/slots ORBAT instance count mismatch: orbat expects {expected}, slots has {}",
                slots.len()
            );
            return Ok(1);
        }
        let mut ids = HashSet::new();
        for slot in &slots {
            let id = slot["id"].as_str().unwrap_or_default().to_string();
            if !ids.insert(id.clone()) {
                eprintln!("/slots duplicate slot id '{id}'");
                return Ok(1);
            }
        }
    }
    println!("ok");
    Ok(0)
}
