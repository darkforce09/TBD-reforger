//! The prefab tables one prefab load decides.
//!
//! **Role:** builds [`PrefabTables`] (the prefab map, the oversized flag and the building and
//! fence footprint lookups) from either served form of the catalogue: gzip JSON
//! ([`tables_from_json`]) or the rkyv archive ([`tables_from_catalog`]), sniffed by
//! [`tables_from_bytes`].
//! **Position:** the top of `prefab_catalog`; read by the map engine's chunk scheduler when a
//! terrain's prefab file arrives.
//! **Signals & state:** none; pure functions; the caller stores the tables.
//! **Invariants:** both lanes give equal tables for the same export; an archive whose rows share a
//! prefab id is refused, because the JSON lane would resolve it differently; a JSON catalogue with
//! a non-integral or out-of-range prefab id is [`WorldError::InvalidPrefabId`]; an empty buffer is
//! [`WorldError::EmptyPayload`].

use std::collections::HashMap;

use serde_json::Value;

use crate::error::InvalidPrefabId;
use crate::footprint_lookups::BuildingPrefabInfo;
use crate::footprint_lookups::FencePrefabInfo;
use crate::footprint_lookups::building_prefab_lookup;
use crate::footprint_lookups::fence_prefab_lookup;
use crate::prefab_rows::PrefabCatalog;
use crate::prefab_rows::PrefabEntry;
use crate::prefab_rows::PrefabRow;
use crate::prefab_rows::build_prefab_maps;
use crate::prefab_rows::catalog_from_bytes;
use crate::prefab_rows::narrow_prefab_rows;
use crate::world_payload::WorldError;
use crate::world_payload::bytes_to_json;
use world_file_formats::ids::PrefabId;

#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn prefab_u16(prefab_id: f64) -> Option<u16> {
    ((0.0..65536.0).contains(&prefab_id) && prefab_id.fract() == 0.0).then_some(prefab_id as u16)
}

#[must_use]
fn prefab_id_u16(prefab_id: PrefabId) -> Option<u16> {
    u16::try_from(prefab_id.get()).ok()
}

/// Everything one prefab load decides. The residency assigns these fields and rebuilds its glyph lookup from `by_id`; nothing else in a load is lane-dependent.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrefabTables {
    /// `prefabId.to_bits()` → render-class code + narrowed row.
    pub by_id: HashMap<u64, PrefabEntry>,

    /// §6 oversized-ring flag.
    pub has_oversized: bool,

    /// Building / pier footprints, keyed by the u16 domain the chunk lookup actually uses.
    pub building_by_u16: HashMap<u16, BuildingPrefabInfo>,

    /// Fence footprints (`kind` `prop`, `class` `fence`), keyed by the u16 instance prefab id;
    /// an id outside `0..65536` is left out.
    pub fence_by_u16: HashMap<u16, FencePrefabInfo>,

    /// Lowest `importance_zoom` among the building footprints, the zoom from which landmark
    /// glyphs can draw early; `None` when no building carries one.
    pub min_importance_zoom: Option<f64>,

    /// Every distinct building `importance_zoom`, ascending; the zooms at which the set of early
    /// landmark glyphs changes.
    pub importance_breakpoints: Vec<f64>,
}

impl PrefabTables {
    fn derive(
        by_id: HashMap<u64, PrefabEntry>,
        has_oversized: bool,
        building_by_u16: HashMap<u16, BuildingPrefabInfo>,
        fence_by_u16: HashMap<u16, FencePrefabInfo>,
    ) -> Self {
        let min_importance_zoom = building_by_u16
            .values()
            .filter_map(|b| b.importance_zoom)
            .fold(None, |acc, v| Some(acc.map_or(v, |a: f64| a.min(v))));
        let mut importance_breakpoints: Vec<f64> = building_by_u16
            .values()
            .filter_map(|b| b.importance_zoom)
            .collect();
        importance_breakpoints
            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        importance_breakpoints.dedup();
        Self {
            by_id,
            has_oversized,
            building_by_u16,
            fence_by_u16,
            min_importance_zoom,
            importance_breakpoints,
        }
    }
}

#[must_use]
fn building_lookup_from_rows<'a>(
    rows: impl Iterator<Item = &'a PrefabRow>,
) -> HashMap<u16, BuildingPrefabInfo> {
    let mut lookup = HashMap::new();
    for row in rows {
        let included = row.kind == "building"
            || (row.kind == "water" && (row.class == "pier" || row.class == "dock"));
        if !included {
            continue;
        }
        let Some(key) = prefab_id_u16(row.prefab_id) else {
            continue;
        };
        lookup.insert(
            key,
            BuildingPrefabInfo {
                building_class: row.class.clone(),
                half_x: row.half_x.filter(|&v| v > 0.0).unwrap_or(2.0),
                half_y: row.half_y.filter(|&v| v > 0.0).unwrap_or(2.0),
                importance_zoom: row.importance_zoom,
            },
        );
    }
    lookup
}

#[must_use]
fn fence_lookup_from_rows<'a>(
    rows: impl Iterator<Item = &'a PrefabRow>,
) -> HashMap<u16, FencePrefabInfo> {
    let mut lookup = HashMap::new();
    for row in rows {
        if row.kind != "prop" || row.class != "fence" {
            continue;
        }
        let Some(key) = prefab_id_u16(row.prefab_id) else {
            continue;
        };
        lookup.insert(
            key,
            FencePrefabInfo {
                half_x: row.half_x.filter(|&v| v > 0.0).unwrap_or(1.0),
                half_y: row.half_y.filter(|&v| v > 0.0).unwrap_or(0.25),
            },
        );
    }
    lookup
}

/// The JSON lane: a parsed `prefabs.json` catalogue → the tables, or [`InvalidPrefabId`] when a
/// row's numeric `prefabId` is not a whole number in `0..=u32::MAX`.
pub fn tables_from_json(raw: &Value) -> Result<PrefabTables, InvalidPrefabId> {
    let (by_id, has_oversized) = build_prefab_maps(narrow_prefab_rows(raw)?);
    let mut building_by_u16 = HashMap::new();
    for (bits, info) in building_prefab_lookup(raw) {
        if let Some(key) = prefab_u16(f64::from_bits(bits)) {
            building_by_u16.insert(key, info);
        }
    }
    Ok(PrefabTables::derive(
        by_id,
        has_oversized,
        building_by_u16,
        fence_prefab_lookup(raw),
    ))
}

/// The archive lane: a decoded `objects/prefabs.rkyv` → the same tables.
pub fn tables_from_catalog(catalog: &PrefabCatalog) -> Result<PrefabTables, WorldError> {
    let distinct = catalog.by_id.len();
    if distinct != catalog.unique_prefabs as usize {
        return Err(WorldError::Archive(format!(
            "prefab catalogue for {:?} declares {} rows but only {distinct} distinct prefabIds — \
             two rows share an id, which the JSON lane resolves differently per lookup, so the two \
             lanes would build different residencies from the same export",
            catalog.terrain_id.as_str(),
            catalog.unique_prefabs
        )));
    }
    let rows = || catalog.by_id.values().map(|e| &e.row);
    Ok(PrefabTables::derive(
        catalog.by_id.clone(),
        catalog.has_oversized,
        building_lookup_from_rows(rows()),
        fence_lookup_from_rows(rows()),
    ))
}

/// | first bytes | route | |---|---| | *none* | [`WorldError::EmptyPayload`] — an empty buffer has no format to sniff | | `1f 8b` | gzip: [`bytes_to_json`] + [`tables_from_json`], today's path unchanged | | anything else | [`catalog_from_bytes`], the validating rkyv reader |.
pub fn tables_from_bytes(bytes: &[u8], terrain: &str) -> Result<PrefabTables, WorldError> {
    if bytes.is_empty() {
        return Err(WorldError::EmptyPayload);
    }
    if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        return Ok(tables_from_json(&bytes_to_json(bytes)?)?);
    }
    let catalog =
        catalog_from_bytes(bytes, terrain).map_err(|e| WorldError::Archive(e.to_string()))?;
    tables_from_catalog(&catalog)
}

#[cfg(test)]
#[path = "tests/prefab_tables_tests.rs"]
mod tests;
