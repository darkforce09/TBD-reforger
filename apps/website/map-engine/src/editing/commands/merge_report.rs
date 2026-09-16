//! Role: phrase a merge report and a duplicate-id report for the author.
//! Position: `editing/commands` in the map engine.
//! Signals & state: explicit inputs only; every function here is pure over what it is handed.
//! Invariants: a report that does not parse yields a line saying so, so a broken report is never silently swallowed. Only non-zero counts are named.

use super::selection_digest::count_noun;

/// T-693 (MENU-SCEN-011) — format a [`MissionDocCore::merge_mission_payload_json`] report for the
/// author: a one-line counts summary + a per-row skipped list. Class-R / ungated so native
/// `cargo test` can pin the wording without a browser (the [`compiled_export_text`] precedent).
///
/// Returns `(summary, skipped_lines)`. `summary` names only the non-zero counts (a merge that added
/// nothing says so), and pluralizes. `skipped_lines` is one `"kind id — reason"` per tolerated
/// malformed row (empty when the merge was clean). A report string that does not parse yields a
/// single skipped line naming that, so the caller never silently swallows a broken report.
pub fn format_merge_report(report_json: &str) -> (String, Vec<String>) {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(report_json) else {
        return (
            "Merge produced no readable report.".to_string(),
            vec![format!("report — could not parse: {report_json}")],
        );
    };
    let n = |k: &str| v.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
    let plural = |count: u64, noun: &str| {
        if count == 1 {
            format!("1 {noun}")
        } else {
            format!("{count} {noun}s")
        }
    };
    let mut parts: Vec<String> = Vec::new();
    let slots = n("slots_added");
    if slots > 0 {
        parts.push(plural(slots, "slot"));
    }
    // Squads / factions: report merged + created distinctly so "grew a side" vs "added a side" reads.
    let sq_created = n("squads_created");
    let sq_merged = n("squads_merged");
    if sq_created > 0 {
        parts.push(plural(sq_created, "squad"));
    }
    if sq_merged > 0 {
        parts.push(format!(
            "{} merged into existing",
            plural(sq_merged, "squad")
        ));
    }
    let fac_created = n("factions_created");
    if fac_created > 0 {
        parts.push(plural(fac_created, "faction"));
    }
    for (key, noun) in [
        ("vehicles_added", "vehicle"),
        ("entities_added", "object"),
        ("zones_added", "zone"),
        ("triggers_added", "trigger"),
        ("compositions_added", "composition"),
        ("markers_added", "marker"),
    ] {
        let c = n(key);
        if c > 0 {
            parts.push(plural(c, noun));
        }
    }

    let summary = if parts.is_empty() {
        "Merge added nothing — the source mission had no mergeable content.".to_string()
    } else {
        format!("Merged {}.", parts.join(", "))
    };

    let skipped: Vec<String> = v
        .get("skipped")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|s| {
                    let kind = s
                        .get("kind")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("row");
                    let id = s
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    let reason = s
                        .get("reason")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("malformed");
                    if id.is_empty() {
                        format!("{kind} — {reason}")
                    } else {
                        format!("{kind} {id} — {reason}")
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    (summary, skipped)
}

/// T-946.86 (.85) — render the [`crate::editor::state::operations::duplicate_slot_ids`] pairs as
/// the per-problem lines `save_now` puts in the Save dialog's `findings`, one line per duplicate,
/// each NAMING THE CALLSIGN AND THE ID. Those two strings are the whole point of the guard: "this
/// mission has duplicate slot ids" is not actionable, "squad 1-1 lists slot s1 twice" is.
///
/// Pure and at file scope — `mod imp` below is `#[cfg(target_arch = "wasm32")]`, so a formatter
/// defined inside it could not be exercised by `cargo test -p website-frontend`. The wasm caller
/// and the native pin therefore read the SAME function rather than two that must agree.
///
/// The headline is returned beside the rows because `status` is also rendered in the top strip and
/// has to stay short (the T-181.44 split the 400-handler below already uses).
pub fn duplicate_slot_id_report(dups: &[(String, String)]) -> (String, Vec<String>) {
    let rows: Vec<String> = dups
        .iter()
        .map(|(callsign, id)| {
            format!("Squad {callsign}: slot id \"{id}\" is used more than once in this squad")
        })
        .collect();
    let head = format!(
        "Save refused — {}",
        count_noun(rows.len(), "duplicate slot id", "duplicate slot ids")
    );
    (head, rows)
}

#[cfg(test)]
#[path = "tests/merge_report.rs"]
mod tests;
