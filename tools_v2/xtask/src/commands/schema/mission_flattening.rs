use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use ticket_engine::sync::refuse_empty_write;

fn read_json(p: &Path) -> Result<Value> {
    let raw = fs::read_to_string(p).with_context(|| format!("read {}", p.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", p.display()))
}

#[cfg(test)]
thread_local! {
    /// When `Some`, stdout flatten writes here instead of `print!` so Class-R can pin
    /// `flatten_orbat_slots(..., false)` without forking the process.
    static FLATTEN_STDOUT_CAPTURE: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

/// Shared flatten transform plus the preserve/refuse rules.
///
/// Used by both `--in-place` and stdout paths so neither silently drops loadout/uid
/// or force-stamps `schemaVersion` over a deliberate prior value.
pub(super) fn apply_flatten_orbat_slots(mission: &mut Value, context: &str) -> Result<usize> {
    let prior_schema = mission.get("schemaVersion").cloned();
    let prior_slots: Vec<Value> = mission
        .get("slots")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();
    let prior_by_id: BTreeMap<String, Value> = prior_slots
        .iter()
        .filter_map(|s| {
            s.get("id")
                .and_then(|i| i.as_str())
                .map(|id| (id.to_string(), s.clone()))
        })
        .collect();
    let prior_loadout_n = prior_slots
        .iter()
        .filter(|s| s.get("loadout").is_some())
        .count();
    let prior_uid_n = prior_slots
        .iter()
        .filter(|s| s.get("uid").is_some())
        .count();

    let mut anchors: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for zone in mission["zones"].as_array().into_iter().flatten() {
        if zone["type"] == "spawn"
            && let (Some(faction), Some(x), Some(z)) = (
                zone["faction"].as_str(),
                zone["shape"]["circle"]["x"].as_f64(),
                zone["shape"]["circle"]["z"].as_f64(),
            )
        {
            anchors.insert(faction.to_string(), (x, z));
        }
    }
    anchors.entry("blufor".into()).or_insert((4831.2, 6620.8));
    anchors.entry("opfor".into()).or_insert((6010.0, 7211.5));

    let mut slots: Vec<Value> = Vec::new();
    let mut slot_index = 0usize;
    let orbat = mission["orbat"].as_object().cloned().unwrap_or_default();
    for (faction_key, faction_orbat) in &orbat {
        let anchor = anchors
            .get(faction_key)
            .copied()
            .unwrap_or((6400.0, 6400.0));
        for group in faction_orbat["groups"].as_array().into_iter().flatten() {
            let callsign = group["callsign"].as_str().unwrap_or_default();
            for role in group["roles"].as_array().into_iter().flatten() {
                let count = role["count"].as_i64().unwrap_or(0);
                for i in 0..count {
                    let ring = (slot_index / 8) as f64;
                    let pos_in_ring = (slot_index % 8) as f64;
                    let angle = pos_in_ring / 8.0 * std::f64::consts::PI * 2.0;
                    let radius = 8.0 + ring * 6.0;
                    let x = anchor.0 + angle.cos() * radius;
                    let z = anchor.1 + angle.sin() * radius;
                    let heading =
                        (((anchor.0 - x).atan2(anchor.1 - z).to_degrees()) + 360.0) % 360.0;
                    let id = format!(
                        "{faction_key}:{callsign}:{}:{i}",
                        role["slot"].as_str().unwrap_or_default()
                    );
                    let mut slot = serde_json::json!({
                        "id": id,
                        "faction": faction_key,
                        "groupCallsign": callsign,
                        "role": role["slot"],
                        "kit": role["kit"],
                        "x": (x * 10.0).round() / 10.0,
                        "z": (z * 10.0).round() / 10.0,
                        "headingDeg": heading.round(),
                    });
                    // Preserve optional schema keys from prior slots / role (loadout, uid).
                    // Prefer role-authored values; fall back to matching prior slot by id.
                    let prior = prior_by_id.get(slot["id"].as_str().unwrap_or(""));
                    if let Some(uid) = role.get("uid").filter(|v| !v.is_null()) {
                        slot["uid"] = uid.clone();
                    } else if let Some(uid) = prior.and_then(|p| p.get("uid")) {
                        slot["uid"] = uid.clone();
                    }
                    if let Some(loadout) = role.get("loadout").filter(|v| !v.is_null()) {
                        slot["loadout"] = loadout.clone();
                    } else if let Some(loadout) = prior.and_then(|p| p.get("loadout")) {
                        slot["loadout"] = loadout.clone();
                    }
                    if let Some(y) = prior.and_then(|p| p.get("y")) {
                        slot["y"] = y.clone();
                    }
                    slots.push(slot);
                    slot_index += 1;
                }
            }
        }
    }

    let n = slots.len();
    let new_loadout_n = slots.iter().filter(|s| s.get("loadout").is_some()).count();
    let new_uid_n = slots.iter().filter(|s| s.get("uid").is_some()).count();

    // Refuse empty / lossy transform — same rules for --in-place AND stdout.
    // Stdout must not silently emit a lossy preview (force-stamping 1.1 drops
    // unmatched loadout/uid without error).
    refuse_empty_write(
        context,
        n == 0 && !prior_slots.is_empty(),
        "would write empty slots[] over a non-empty committed slots array",
    )?;
    if new_loadout_n < prior_loadout_n || new_uid_n < prior_uid_n {
        bail!(
            "refusing empty write ({context}): \
             would drop loadout ({prior_loadout_n}→{new_loadout_n}) or \
             uid ({prior_uid_n}→{new_uid_n}) from committed slots"
        );
    }
    // Preserve deliberate schemaVersion (e.g. 1.0 last-stand fixture) — never force-stamp 1.1.
    if prior_schema.is_none() {
        mission["schemaVersion"] = Value::String("1.1".into());
    }
    // else: leave schemaVersion untouched

    mission["slots"] = Value::Array(slots);
    Ok(n)
}

/// CLI body shared by `--in-place` and stdout: read → apply → return mission.
///
/// No post-apply `schemaVersion` mutation lives here or in [`flatten_orbat_slots`] —
/// preserve/default stamping is solely inside [`apply_flatten_orbat_slots`].
pub(super) fn flatten_orbat_slots_mission(
    path: &str,
    in_place: bool,
) -> Result<(PathBuf, Value, usize)> {
    let file = PathBuf::from(path);
    let mut mission = read_json(&file)?;
    let context = if in_place {
        format!("flatten-orbat-slots --in-place {}", file.display())
    } else {
        format!("flatten-orbat-slots (stdout) {}", file.display())
    };
    let n = apply_flatten_orbat_slots(&mut mission, &context)?;
    Ok((file, mission, n))
}

pub fn flatten_orbat_slots(path: &str, in_place: bool) -> Result<u8> {
    let (file, mission, n) = flatten_orbat_slots_mission(path, in_place)?;
    let out = serde_json::to_string_pretty(&mission)? + "\n";
    if in_place {
        fs::write(&file, out)?;
        println!("Wrote {n} slots to {}", file.display());
    } else {
        // Tests may capture this exact stdout emission (not apply_* alone).
        #[cfg(test)]
        {
            let captured = flatten_stdout_capture_buf(|buf| {
                if let Some(b) = buf.as_mut() {
                    b.push_str(&out);
                    true
                } else {
                    false
                }
            });
            if captured {
                return Ok(0);
            }
        }
        print!("{out}");
    }
    Ok(0)
}

#[cfg(test)]
pub(super) fn flatten_stdout_capture_buf<R>(f: impl FnOnce(&mut Option<String>) -> R) -> R {
    FLATTEN_STDOUT_CAPTURE.with(|c| f(&mut c.borrow_mut()))
}

/// Run the real stdout CLI entrypoint (`in_place=false`) and parse emitted JSON.
#[cfg(test)]
pub(super) fn flatten_stdout_json(path: &str) -> Result<Value> {
    flatten_stdout_capture_buf(|b| *b = Some(String::new()));
    let run = flatten_orbat_slots(path, false);
    let buf = flatten_stdout_capture_buf(|b| b.take().unwrap_or_default());
    run?;
    Ok(serde_json::from_str(&buf)?)
}

#[cfg(test)]
#[path = "tests/mission_flattening.rs"]
mod tests;
