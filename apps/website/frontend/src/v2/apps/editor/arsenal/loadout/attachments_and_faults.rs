//! Attachment sets and the loadout verdict.

use super::*;

/* ─────────────────────── weapon attachment sets ─────────────────────── */

/// Compatibility edge type linking attachments to their host weapon.
pub(crate) const ATTACHMENT_EDGE: &str = "attachment_on_weapon";

/// Separator for the packed attachment set. U+001F (ASCII US) is safe **by contract, not by luck**:
/// `registry-compat.schema.json#/$defs/resourceName` pins every node to
/// `^\{[0-9A-F]{16}\}[A-Za-z0-9/_.\- ()']+$` — a pattern that admits no control character — so a
/// join can never produce a string that splits back into something else.
pub(super) const ATTACHMENT_SEP: &str = "\u{1f}";

/// The `picks` key holding `weapon_key`'s attachment set.
///
/// The set rides a **synthetic key** rather than widening `picks` to `HashMap<String, Vec<String>>`
/// because that map is the argument type of three [`crate::v2::apps::editor::arsenal::rules`] entry points
/// (`row_options`, `validate_loadout`, `loadout_weight`) and this slice does not own that module.
/// The `@` infix cannot collide with a row key, and each of those consumers iterates `LOADOUT_ROWS`
/// **by key** — so the synthetic entry is invisible to them by construction, not by convention.
pub(crate) fn attachments_key(weapon_key: &str) -> String {
    format!("attachments@{weapon_key}")
}

/// `weapon_key`'s picked attachments, in pick order.
pub(crate) fn attachments_of(picks: &HashMap<String, String>, weapon_key: &str) -> Vec<String> {
    picks
        .get(&attachments_key(weapon_key))
        .map(|packed| {
            packed
                .split(ATTACHMENT_SEP)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Pack a set back into its `picks` value. An empty set packs to `""`, which the `pick_item` path
/// treats as "remove the key" — so clearing the last attachment leaves no residue in the map.
pub(crate) fn pack_attachments(list: &[String]) -> String {
    list.join(ATTACHMENT_SEP)
}

/// Returns stranded attachment faults keyed to the weapon row.
/// Unavailable compatibility data contributes no fault.
pub(super) fn attachment_errors(
    picks: &HashMap<String, String>,
    feed: &CompatFeed,
) -> Vec<rules::RowError> {
    let Some(g) = feed.ready_graph() else {
        return Vec::new();
    };
    let mut errs = Vec::new();
    for &(key, _, _) in rules::WEAPON_SLOTS {
        let host = picks.get(key).filter(|s| !s.is_empty());
        let label = rules::row(key).map_or(key, |r| r.label);
        for rn in attachments_of(picks, key) {
            // The message names `rn` because the key cannot. `refusal_line` separates two stranded
            // *rows* by prefixing their labels, but a weapon row can strand two *attachments* at
            // once — same row, same key, same prefix — so without the resource name the author is
            // handed the identical sentence twice without identifying the attachment.
            let message = match host {
                None => format!("Attachment `{rn}` requires a {label} pick"),
                Some(h) if !g.accepts(h, &rn, ATTACHMENT_EDGE) => {
                    format!("Attachment `{rn}` not compatible with the selected {label}")
                }
                Some(_) => continue,
            };
            errs.push(rules::RowError { key, message });
        }
    }
    errs
}

/// Collects compatibility, attachment, capacity, and unworn-container findings.
/// The list drives the verdict badge; unavailable compatibility and kit evidence do not create speculative faults.
pub(crate) fn loadout_faults(
    picks: &HashMap<String, String>,
    cargo: &[rules::CargoRow],
    feed: &CompatFeed,
    idx: &HashMap<String, &RegistryItem>,
    kit_defaults: Option<&HashSet<String>>,
) -> Vec<rules::RowError> {
    let mut errs = validate_loadout(picks, feed.ready_graph(), feed.status);
    errs.extend(attachment_errors(picks, feed));
    errs.extend(rules::cargo_capacity_errors(picks, cargo, idx));
    errs.extend(rules::cargo_unworn_container_errors(
        picks,
        cargo,
        kit_defaults,
    ));
    errs
}

/// Finds items catalogued as carried by the slot character prefab.
/// Returns `None` when the feed or asset ID is unavailable, preserving the distinction from an empty known set.
pub(crate) fn kit_default_items(
    feed: &CompatFeed,
    asset_id: Option<&str>,
) -> Option<HashSet<String>> {
    let graph = feed.ready_graph()?;
    let rn = asset_id?;
    Some(
        graph
            .items_for(rn, rules::CHARACTER_DEFAULT_CARGO_EDGE)
            .into_iter()
            .collect(),
    )
}

/// The slot's `assetId` (its character prefab) straight off the live document.
///
/// Read through the existing public `editor_context::slots_json` rather than a new accessor — this
/// slice does not own `editor_ops`. Native has no hosted document, so there is no `assetId` and
/// [`kit_default_items`] answers `None`.
pub(crate) fn slot_asset_id(slot_id: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let json = crate::v2::apps::editor::bridge::host_state::editor_context::slots_json()?;
        let map: serde_json::Value = serde_json::from_str(&json).ok()?;
        map.get(slot_id)?
            .get("assetId")?
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = slot_id;
        None
    }
}
