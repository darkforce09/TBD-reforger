//! Build and gate exported loadout documents.

use super::*;

/// The modpack the picks were authored against — `modpackId` on the exported file.
///
/// Every registry row is scoped to one modpack (`GET /registry` filters by it), so the catalog the
/// Arsenal was handed IS the answer; there is no second source to disagree with. An empty catalog
/// yields `""`, which the schema permits (`{"type":"string"}`, no `minLength`) and which the mod
/// reader turns into a named `modpackId … != expected` warning rather than a silent mismatch —
/// the honest outcome when the registry fetch failed and we genuinely do not know.
pub(crate) fn export_modpack_id(items: &[RegistryItem]) -> String {
    items
        .first()
        .map(|it| it.modpack_id.clone())
        .unwrap_or_default()
}

/// Exports a loadout after checking catalogued cargo capacity.
/// Capacity failures return refusals without bytes. Unworn-container findings remain warnings because a kit may supply its own garment.
pub fn try_export(
    picks: &HashMap<String, String>,
    cargo: &[rules::CargoRow],
    items: &[RegistryItem],
    modpack_id: &str,
) -> Result<String, Vec<rules::RowError>> {
    let idx = index_by_name(items);
    let refusals = rules::cargo_capacity_errors(picks, cargo, &idx);
    if !refusals.is_empty() {
        return Err(refusals);
    }
    Ok(picks_to_export(picks, cargo, modpack_id))
}
