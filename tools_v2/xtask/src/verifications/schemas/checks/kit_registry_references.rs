use super::*;

/// Every `kit:` alias a mission document references, as `(JSON pointer, alias)`.
///
/// Both sites are the same contract one step apart: `slots[].kit` is what TBD_MissionValidator
/// resolves at boot, and `orbat[].groups[].roles[].kit` is what the flatten turns INTO `slots[].kit`.
/// Checking only the first would let a dangling alias sit in a 1.0 document until someone compiles it.
pub(super) fn mission_kit_refs(doc: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (i, s) in doc["slots"].as_array().into_iter().flatten().enumerate() {
        if let Some(k) = s.get("kit").and_then(Value::as_str) {
            out.push((format!("/slots/{i}/kit"), k.to_string()));
        }
    }
    for (fk, fv) in doc["orbat"].as_object().into_iter().flatten() {
        for (gi, g) in fv["groups"].as_array().into_iter().flatten().enumerate() {
            for (ri, r) in g["roles"].as_array().into_iter().flatten().enumerate() {
                if let Some(k) = r.get("kit").and_then(Value::as_str) {
                    out.push((
                        format!("/orbat/{fk}/groups/{gi}/roles/{ri}/kit"),
                        k.to_string(),
                    ));
                }
            }
        }
    }
    out
}

/// Every `preset:` alias a mission references, as `(JSON pointer, alias)`.
pub(super) fn mission_preset_refs(doc: &Value) -> Vec<(String, String)> {
    doc["factions"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(i, f)| {
            f.get("presetId")
                .and_then(Value::as_str)
                .map(|p| (format!("/factions/{i}/presetId"), p.to_string()))
        })
        .collect()
}

/// `kit:` references in `doc` that no registry entry defines, as `(pointer, alias)`.
pub(super) fn dangling_kits(doc: &Value, aliases: &HashSet<String>) -> Vec<(String, String)> {
    mission_kit_refs(doc)
        .into_iter()
        .filter(|(_, k)| !aliases.contains(k))
        .collect()
}

/// The alias set the game server actually resolves against.
///
/// Read straight out of the mod's `Data/registry.json` rather than a mirror, so the gate cannot
/// drift from the thing it is gating. Returns the path too, for an honest provenance line.
pub(super) fn spawn_registry_aliases(root: &Path) -> Result<(PathBuf, HashSet<String>)> {
    let p = root.join("apps/mod/tbd-framework/Data/registry.json");
    let doc = read_json(&p)?;
    let set: HashSet<String> = doc["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| e.get("alias").and_then(Value::as_str))
        .map(str::to_string)
        .collect();
    anyhow::ensure!(!set.is_empty(), "{}: no entries[].alias", p.display());
    Ok((p, set))
}
