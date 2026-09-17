//! Validate and read imported loadout documents.

use super::buffered_loadout_operations::loadout_rule_refusals;
use super::*;

/* ───────── reading a loadout export document ───────── */

/// The row key every *document-level* refusal is filed under.
///
/// [`rules::RowError::key`] normally names the loadout row whose pick the author must change, and
/// the rule-derived refusals below keep doing exactly that. A malformed file has no row to blame —
/// the fault is the document — so it gets its own key rather than being pinned on an innocent row.
pub(super) const IMPORT_DOC_KEY: &str = "document";

/// Prefixes a row refusal with its label so the author can identify the pick.
/// Document-level messages already carry JSON paths and remain unprefixed.
#[must_use]
pub fn refusal_line(e: &rules::RowError) -> String {
    match rules::row(e.key) {
        Some(r) => format!("{} — {}", r.label, e.message),
        None => e.message.clone(),
    }
}

/// Validated imported picks and cargo, ready for an explicit document write.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportedLoadout {
    /// The picks map, in the same shape [`loadout_to_picks`] produces (incl. the packed
    /// `attachments@<weapon>` keys).
    pub picks: HashMap<String, String>,
    pub cargo: Vec<rules::CargoRow>,
    /// Did the document carry a `cargo` key at all?
    ///
    /// `cargo` is optional in the v2 branch and absent from v1 entirely, and key presence is the
    /// A present-and-empty cargo array means *the author cleared it* (never
    /// re-seed), absent means *nobody has said* (a later seed may still fire). A file that never
    /// mentions cargo has not authored an empty cargo list, so importing one must not claim it did.
    pub cargo_present: bool,
    /// `modpackId` off the document. Reported, never a refusal — see [`try_import`].
    pub modpack_id: String,
    /// `"1"` or `"2"` — which `oneOf` branch the document satisfied.
    pub loadout_version: String,
}

/// Reads either export schema branch into picks and cargo.
/// The v2 branch uses the persisted loadout reader; the v1 gear branch maps its limited vocabulary without inventing armor.
fn import_doc_to_picks(
    raw: &str,
    doc: &serde_json::Value,
) -> (HashMap<String, String>, Vec<rules::CargoRow>, bool) {
    let version = doc
        .get("loadoutVersion")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if version == "2" {
        let picks = loadout_to_picks(Some(raw));
        let (cargo, present) = rules::cargo_from_loadout(Some(raw));
        return (picks, cargo, present);
    }
    let gear = doc.get("gear");
    let mut picks = HashMap::new();
    for (doc_key, pick_key) in [
        ("primary", "primary"),
        ("uniform", "jacket"),
        ("vest", "vest"),
        ("helmet", "headCover"),
        ("optic", "optic"),
        ("magazine", "magazine"),
    ] {
        let value = gear
            .and_then(|g| g.get(doc_key))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());
        if let Some(v) = value {
            picks.insert(pick_key.to_string(), v.to_string());
        }
    }
    (picks, Vec::new(), false)
}

/// Parses and validates a loadout document before returning any picks.
/// JSON, shipped-schema, and loadout-rule checks run in order. A failed check returns only refusals; a modpack mismatch remains advisory.
pub fn try_import(
    raw: &str,
    items: &[RegistryItem],
    feed: &CompatFeed,
) -> Result<ImportedLoadout, Vec<rules::RowError>> {
    let doc: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(e) => {
            return Err(vec![rules::RowError {
                key: IMPORT_DOC_KEY,
                message: format!("This file is not valid JSON — {e}."),
            }])
        }
    };
    if let Err(faults) = rules::validate_against_loadout_export_schema(&doc) {
        return Err(faults
            .into_iter()
            .map(|message| rules::RowError {
                key: IMPORT_DOC_KEY,
                message,
            })
            .collect());
    }
    let (picks, cargo, cargo_present) = import_doc_to_picks(raw, &doc);

    // Import and buffered Apply share the same rule gate before any document write.
    let refusals = loadout_rule_refusals(&picks, &cargo, items, feed);
    if !refusals.is_empty() {
        return Err(refusals);
    }

    Ok(ImportedLoadout {
        picks,
        cargo,
        cargo_present,
        modpack_id: doc
            .get("modpackId")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        loadout_version: doc
            .get("loadoutVersion")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
    })
}

/// The one-line receipt an accepted import prints: what actually landed, counted off the applied
/// state rather than off the file, so it cannot claim more than was applied.
pub(crate) fn import_summary(name: &str, doc: &ImportedLoadout, catalog_modpack: &str) -> String {
    let weapons = ROWS
        .iter()
        .filter(|r| r.weapon.is_some())
        .filter(|r| doc.picks.get(r.key).is_some_and(|v| !v.is_empty()))
        .count();
    let wear = ROWS
        .iter()
        .filter(|r| r.weapon.is_none())
        .filter(|r| doc.picks.get(r.key).is_some_and(|v| !v.is_empty()))
        .count();
    let mut line = format!(
        "Imported {name} (v{}) — {weapons} weapon(s), {wear} wear row(s), {} cargo row(s). One Ctrl+Z undoes the whole import.",
        doc.loadout_version,
        doc.cargo.len(),
    );
    // Warn-only, and only when both sides actually know what they are: an empty modpackId is the
    // honest "we do not know" the export writes when the registry fetch failed, not a mismatch.
    if !doc.modpack_id.is_empty()
        && !catalog_modpack.is_empty()
        && doc.modpack_id != catalog_modpack
    {
        line.push_str(&format!(
            " Note: this file was authored against modpack {}, and this mission's catalog is {} — check the picks resolved to what you expected.",
            doc.modpack_id, catalog_modpack
        ));
    }
    line
}
