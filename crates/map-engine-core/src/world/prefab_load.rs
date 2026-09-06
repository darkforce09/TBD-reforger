//! T-935.14 — the residency's prefab tables, built from **either** lane.
//!
//! T-935.11 shipped `objects/prefabs.rkyv` and [`catalog_from_bytes`], and deliberately shipped no
//! caller: every prefab consumer in the SPA went through `WorldResidency::load_prefabs_gz`, whose
//! four downstream structures are all derived from a [`serde_json::Value`]. Handing the archive's
//! bytes to that method would have been a fast path that could not fire. This file is where the
//! archive lands instead.
//!
//! [`PrefabTables`] is everything a prefab load decides:
//!
//! | field | JSON lane | archive lane |
//! |---|---|---|
//! | `by_id` / `has_oversized` | [`build_prefab_maps`] over [`narrow_prefab_rows`] | [`PrefabCatalog::by_id`] (same pair, computed by the reader) |
//! | `building_by_u16` | [`building_prefab_lookup`] over the `Value`, narrowed to the u16 key domain | [`building_lookup_from_rows`] over the decoded rows |
//! | `fence_by_u16` | [`fence_prefab_lookup`] over the `Value` | [`fence_lookup_from_rows`] over the decoded rows |
//! | `min_importance_zoom` / `importance_breakpoints` | derived from `building_by_u16` | *(identical code — see below)* |
//!
//! The two derived fields are computed **once**, in [`PrefabTables::derive`], for both lanes. That
//! is not a shortcut past the parity pin: they are a pure function of `building_by_u16`, which the
//! pin compares whole, so a lane that got the lookup right cannot then get the breakpoints wrong.
//! The lookups themselves are two independent walks and that is the point — the pin compares
//! derivations, not one derivation against itself.
//!
//! # Why the file exists at all
//!
//! `residency.rs` is a SIZE-3 allowlisted file, so per the file-length rule it takes only the call
//! site and the logic lives beside the reader it calls — the same split T-935.3 used for
//! `chunk_bin.rs`. The JSON derivation moved here with it so that both lanes end at one `apply`,
//! rather than the archive lane growing a second copy of the tail that could drift from it.
//!
//! # The f32 projection, and why "identical" needs stating
//!
//! The wire row is the compact f32 twin of [`PrefabRow`] (`prefab.rs` §"The absent-optional
//! encoding"), so a JSON `2.1` comes back as `f64::from(2.1_f32)`. The archive lane is therefore
//! the f32 projection of the JSON lane, not a copy of it, and the everon pin below says so
//! explicitly rather than comparing `==` and hoping. Absent is `NaN`, **not** zero — zero is a
//! legal half-extent, and [`tables_agree_on_a_real_zero_half_extent`] constructs one because
//! everon has none to catch it with.
//!
//! [`tables_agree_on_a_real_zero_half_extent`]: tests::tables_agree_on_a_real_zero_half_extent

use std::collections::HashMap;

use serde_json::Value;

use super::obb::{
    BuildingPrefabInfo, FencePrefabInfo, building_prefab_lookup, fence_prefab_lookup,
};
use super::prefab::{
    PrefabCatalog, PrefabEntry, PrefabRow, build_prefab_maps, catalog_from_bytes,
    narrow_prefab_rows,
};
use super::store::{WorldError, bytes_to_json};

/// The effective key domain of the JS `buildingInfo.get(prefabIdx[k])` lookup: `prefabIdx` is a
/// `Uint16Array`, so a building is found iff `prefabId ===` the stored `u16`. `None` for anything
/// that is not a whole number in `[0, 65536)` — the same test `residency.rs` applied inline before
/// T-935.14 and the same one `fence_prefab_lookup` applies.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn prefab_u16(prefab_id: f64) -> Option<u16> {
    ((0.0..65536.0).contains(&prefab_id) && prefab_id.fract() == 0.0).then_some(prefab_id as u16)
}

/// Everything one prefab load decides. The residency assigns these fields and rebuilds its glyph
/// lookup from `by_id`; nothing else in a load is lane-dependent.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrefabTables {
    /// `prefabId.to_bits()` → render-class code + narrowed row.
    pub by_id: HashMap<u64, PrefabEntry>,
    /// §6 oversized-ring flag.
    pub has_oversized: bool,
    /// Building / pier footprints, keyed by the u16 domain the chunk lookup actually uses.
    pub building_by_u16: HashMap<u16, BuildingPrefabInfo>,
    /// Fence-prop half-extents (T-152.4).
    pub fence_by_u16: HashMap<u16, FencePrefabInfo>,
    /// T-152.21 — smallest `importanceZoom` across building prefabs (the badge lane's O(1) guard).
    pub min_importance_zoom: Option<f64>,
    /// T-175 A5 — sorted-dedup distinct `importanceZoom` breakpoints (the glyph memo's activation
    /// index). Order is part of the value: `partition_point` reads it.
    pub importance_breakpoints: Vec<f64>,
}

impl PrefabTables {
    /// Fill in the two fields that are a pure function of `building_by_u16`. Byte-for-byte the
    /// derivation `load_prefabs_gz` did inline before this file existed.
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

/// `buildingPrefabLookup` over decoded rows instead of a `Value` — the archive twin of
/// [`building_prefab_lookup`], including its `2.0` fallback for an absent or non-positive
/// half-extent and its `building` / `water`-pier-or-dock include test.
///
/// Rows arrive already collapsed by prefab id (see [`tables_from_catalog`]), so unlike the JSON
/// walk this cannot depend on iteration order: every id is inserted exactly once.
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
        let Some(key) = prefab_u16(row.prefab_id) else {
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

/// `fencePrefabLookup` over decoded rows — the archive twin of [`fence_prefab_lookup`], including
/// its asymmetric `1.0` / `0.25` fallbacks (a fence is long and thin, and a missing half-extent
/// must not make it square).
#[must_use]
fn fence_lookup_from_rows<'a>(
    rows: impl Iterator<Item = &'a PrefabRow>,
) -> HashMap<u16, FencePrefabInfo> {
    let mut lookup = HashMap::new();
    for row in rows {
        if row.kind != "prop" || row.class != "fence" {
            continue;
        }
        let Some(key) = prefab_u16(row.prefab_id) else {
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

/// The JSON lane: `prefabs.json.gz` (already inflated to a `Value`) → the tables. Every call here
/// is the one `load_prefabs_gz` made inline before T-935.14, in the same order.
#[must_use]
pub fn tables_from_json(raw: &Value) -> PrefabTables {
    let (by_id, has_oversized) = build_prefab_maps(narrow_prefab_rows(raw));
    let mut building_by_u16 = HashMap::new();
    for (bits, info) in building_prefab_lookup(raw) {
        if let Some(key) = prefab_u16(f64::from_bits(bits)) {
            building_by_u16.insert(key, info);
        }
    }
    PrefabTables::derive(
        by_id,
        has_oversized,
        building_by_u16,
        fence_prefab_lookup(raw),
    )
}

/// The archive lane: a decoded `objects/prefabs.rkyv` → the same tables.
///
/// # The collision refusal, and why it is an error rather than a narrowing
///
/// The JSON lane resolves a duplicate `prefabId` **per lookup**: `build_prefab_maps` keeps the last
/// such row, while `building_prefab_lookup` keeps the last *included* one, so an id carried by both
/// a `building` row and a later `tree` row wins the building table and loses the class table. The
/// archive lane cannot reproduce that, because [`PrefabCatalog`] hands over rows already collapsed
/// by id — order is gone by the time this function sees them.
///
/// Rather than let the two lanes quietly build different residencies for such a file, the collision
/// is refused: `type_inventory.unique_prefabs` counts *rows* (checked against `prefabs.len()` by
/// [`catalog_from_bytes`]), so a catalogue with a duplicate id has strictly fewer distinct ids than
/// it declares, and that inequality is exactly the condition below. Everon has none — the pin says
/// so — and if an export ever grows one, this errors instead of drawing the wrong buildings.
///
/// # Errors
/// [`WorldError::Archive`] when the catalogue's distinct-id count disagrees with its census.
pub fn tables_from_catalog(catalog: &PrefabCatalog) -> Result<PrefabTables, WorldError> {
    let distinct = catalog.by_id.len();
    if distinct != catalog.unique_prefabs as usize {
        return Err(WorldError::Archive(format!(
            "prefab catalogue for {:?} declares {} rows but only {distinct} distinct prefabIds — \
             two rows share an id, which the JSON lane resolves differently per lookup, so the two \
             lanes would build different residencies from the same export",
            catalog.terrain_id, catalog.unique_prefabs
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

/// T-935.14 — the prefab table from **either** source, sniffing which one arrived. Same shape, same
/// reasoning and the same two magic bytes as `WorldStore::load_roads` (T-935.6):
///
/// | first bytes | route |
/// |---|---|
/// | *none* | [`WorldError::EmptyPayload`] — an empty buffer has no format to sniff |
/// | `1f 8b` | gzip: [`bytes_to_json`] + [`tables_from_json`], today's path unchanged |
/// | anything else | [`catalog_from_bytes`], the validating rkyv reader |
///
/// The `else` arm is rkyv rather than "try JSON too" for the reason `store.rs` spells out: a
/// fallback chain answers every failure with the *second* parser's error, and a JSON payload that
/// happened to survive rkyv validation would be a wild read. A head-truncated gzip has lost its
/// magic and is refused by the archive reader; a tail-truncated one keeps it and is refused by the
/// gunzip. `load_prefabs_gz` stays public for the callers that know they hold JSON.
///
/// `terrain` is not decoration — see [`catalog_from_bytes`]: the catalogue records the terrain it
/// was built for, and loading everon's table for arland would resolve every prefab id against the
/// wrong catalogue *without failing to find one*. The caller states the world it thinks it is
/// loading and a disagreement is refused.
///
/// # Errors
/// [`WorldError::EmptyPayload`] on a zero-length buffer; [`WorldError::Gzip`]/[`WorldError::Json`]
/// from the JSON route; [`WorldError::Archive`] from the rkyv route (invalid buffer, wrong schema
/// version, wrong terrain, a census that does not describe the rows, a class code written against a
/// different table, or the duplicate-id collision above).
pub fn tables_from_bytes(bytes: &[u8], terrain: &str) -> Result<PrefabTables, WorldError> {
    if bytes.is_empty() {
        return Err(WorldError::EmptyPayload);
    }
    if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        return Ok(tables_from_json(&bytes_to_json(bytes)?));
    }
    let catalog =
        catalog_from_bytes(bytes, terrain).map_err(|e| WorldError::Archive(e.to_string()))?;
    tables_from_catalog(&catalog)
}

#[cfg(test)]
mod tests {
    use super::super::binary::archives::{
        ARCHIVE_SCHEMA_VERSION, PrefabCatalogArchive, TypeInventory,
    };
    use super::super::binary::to_bytes;
    use super::super::prefab::{inventory_to_archive, row_to_archive};
    use super::super::residency::WorldResidency;
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;
    use std::path::PathBuf;

    /// The committed everon catalogue (`prefab.rs` pins the same two numbers). Re-pin deliberately
    /// if the export changes — a silently shrinking corpus is how a parity test stops proving
    /// anything.
    const EVERON_PREFABS: usize = 1623;
    /// The fixture chunk the T-152.3 landmark tests drive; it has trees, props and buildings.
    const FIXTURE_CHUNK: &str = "2_12";

    fn map_assets() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/map-assets")
    }

    fn everon() -> PathBuf {
        map_assets().join("everon")
    }

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(bytes).unwrap();
        enc.finish().unwrap()
    }

    fn everon_prefabs_gz() -> Vec<u8> {
        std::fs::read(everon().join("objects/prefabs.json.gz")).expect("prefabs.json.gz")
    }

    fn everon_prefabs_json() -> Value {
        bytes_to_json(&everon_prefabs_gz()).expect("prefabs decode")
    }

    /// The archive's actual contract for a JSON number: the wire row is the f32 twin of the parser
    /// struct, so the round trip is `f64::from(v as f32)`, not the identity. Written longhand
    /// rather than calling the production `num_to_wire`/`wire_to_num` — an oracle that reuses the
    /// helpers it is checking cannot catch a bug in them.
    #[allow(clippy::cast_possible_truncation)]
    fn f32_narrow(v: f64) -> f64 {
        f64::from(v as f32)
    }

    /// Everon's `prefabs.json.gz` with every number the wire row carries pushed through f32, so the
    /// JSON lane can be compared to the archive lane by **exact equality** instead of a tolerance.
    /// Only the six fields `PrefabEntryArchive` stores as `f32` move; `prefabId` is a `u32` join key
    /// on both sides and must not be touched.
    fn everon_prefabs_json_f32() -> Value {
        let mut raw = everon_prefabs_json();
        let narrow_at = |v: &mut Value, path: &[&str]| {
            let mut cur = v;
            for key in path {
                match cur.get_mut(*key) {
                    Some(next) => cur = next,
                    None => return,
                }
            }
            if let Some(n) = cur.as_f64() {
                *cur = Value::from(f32_narrow(n));
            }
        };
        let rows = raw
            .get_mut("prefabs")
            .and_then(Value::as_array_mut)
            .expect("prefabs array");
        for row in rows.iter_mut() {
            for path in [
                &["spatial", "halfExtentsM", "x"][..],
                &["spatial", "halfExtentsM", "y"][..],
                &["spatial", "halfExtentsM", "z"][..],
                &["spatial", "heightM"][..],
                &["render", "baseSizePx"][..],
                &["render", "importanceZoom"][..],
            ] {
                narrow_at(row, path);
            }
        }
        raw
    }

    /// The everon catalogue exactly as `build-world-objects` writes it: rows from the committed
    /// JSON through the shipped [`row_to_archive`], census from the committed `type-inventory.json`.
    fn everon_archive_bytes() -> Vec<u8> {
        let rows = narrow_prefab_rows(&everon_prefabs_json());
        let inventory_doc = std::fs::read_to_string(everon().join("objects/type-inventory.json"))
            .expect("type-inventory.json");
        let archive = PrefabCatalogArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            prefabs: rows
                .iter()
                .enumerate()
                .map(|(i, r)| row_to_archive(i, r).expect("row → archive"))
                .collect(),
            type_inventory: inventory_to_archive(
                &serde_json::from_str(&inventory_doc).expect("inventory parse"),
            )
            .expect("inventory → archive"),
        };
        to_bytes(&archive).expect("serialise").to_vec()
    }

    /* ───────────────────────── the whole-residency parity pin ───────────────────────── */

    fn glyph_keys() -> Vec<String> {
        let raw = std::fs::read_to_string(map_assets().join("glyphs/manifest.json"))
            .expect("glyphs manifest");
        let v: Value = serde_json::from_str(&raw).unwrap();
        let mut keys: Vec<String> = v["glyphs"]
            .as_object()
            .expect("glyphs object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    }

    /// A live everon residency driven to a settled draw state over one real chunk, with the prefab
    /// catalogue loaded from `bytes` through the shipped sniffing entry point. The glyph key map is
    /// registered FIRST so `load_prefabs`' own glyph rebuild is what fills the table — registering
    /// it afterwards would have `set_glyph_key_map` build the table and hide a lane difference.
    fn everon_residency(bytes: &[u8]) -> WorldResidency {
        let mut r = WorldResidency::new();
        r.load_manifest_json(
            &std::fs::read_to_string(everon().join("manifest.json")).expect("everon manifest"),
        )
        .expect("manifest");
        r.load_chunk_index_json(
            &std::fs::read_to_string(everon().join("objects/chunks/manifest.json"))
                .expect("chunk index"),
        )
        .expect("chunk index");
        r.set_glyph_key_map(&glyph_keys());
        assert_eq!(
            r.load_prefabs(bytes, "everon").expect("prefab load"),
            EVERON_PREFABS,
            "everon prefab corpus changed; re-pin EVERON_PREFABS deliberately"
        );
        r.set_glyph_toggles(true, true, true);
        // z = 3.5 clears every world LOD gate at once (tree 0.0, badge 1.0, rockLarge 1.0,
        // vegetation 1.5, fence 1.5, prop 3.0), so all three glyph lanes and both strip lanes
        // compose — a zoom that hid one of them would compare two empty buffers and pass.
        let mut parts = FIXTURE_CHUNK.split('_');
        let cx: f64 = parts.next().unwrap().parse().unwrap();
        let cy: f64 = parts.next().unwrap().parse().unwrap();
        let (min_x, min_y) = (cx * 512.0, cy * 512.0);
        let missing = r.set_viewport(min_x, min_y, min_x + 512.0, min_y + 512.0, 3.5);
        assert!(!missing.is_empty(), "the viewport must request chunks");
        for id in &missing {
            let path = everon()
                .join("objects/chunks")
                .join(format!("{id}.json.gz"));
            r.ingest_chunk_gz(
                id,
                &std::fs::read(&path).unwrap_or_else(|e| panic!("{path:?}: {e}")),
            )
            .expect("ingest");
        }
        r.end_apply_frame(0.0);
        r
    }

    /// THE SLICE'S PIN. The archive lane and the JSON lane build the same residency for the
    /// committed everon corpus — the whole tables, not a spot check, and then the whole composed
    /// output of the four structures a prefab load feeds.
    ///
    /// The JSON side is the f32-narrowed twin of the committed export, because that projection is
    /// the archive's stated contract (`prefab.rs` §"The absent-optional encoding") and not a
    /// tolerance: with it stated, everything below is exact equality.
    #[test]
    fn everon_archive_lane_builds_the_same_residency_as_the_json_lane() {
        let json_f32 = gzip(
            serde_json::to_string(&everon_prefabs_json_f32())
                .unwrap()
                .as_bytes(),
        );
        let rkyv = everon_archive_bytes();
        assert!(
            bytes_to_json(&rkyv).is_err(),
            "the archive bytes must not parse as JSON, or the rkyv route has not been proven to run"
        );
        assert_ne!(
            &rkyv[..2],
            &[0x1f, 0x8b],
            "the archive must not carry gzip magic"
        );

        let from_json = tables_from_bytes(&json_f32, "everon").expect("json lane");
        let from_rkyv = tables_from_bytes(&rkyv, "everon").expect("archive lane");

        // The corpus has to actually exercise all four structures, or "equal" is "equally empty".
        assert_eq!(from_rkyv.by_id.len(), EVERON_PREFABS, "distinct prefab ids");
        assert!(from_rkyv.building_by_u16.len() > 100, "buildings");
        assert!(!from_rkyv.fence_by_u16.is_empty(), "fences");
        assert!(!from_rkyv.importance_breakpoints.is_empty(), "breakpoints");
        // Measured, not assumed: everon's largest classified half-extent is under the 64 m
        // oversized threshold, so the flag is `false` on BOTH sides and the equality below cannot
        // claim credit for it. `prefab.rs`' `build_maps_codes_and_oversized` covers the true arm.
        assert!(
            !from_rkyv.has_oversized,
            "everon carries no oversized prefab"
        );

        assert_eq!(from_rkyv.by_id, from_json.by_id, "prefab row table");
        assert_eq!(
            from_rkyv.building_by_u16, from_json.building_by_u16,
            "building lookup"
        );
        assert_eq!(
            from_rkyv.fence_by_u16, from_json.fence_by_u16,
            "fence lookup"
        );
        assert_eq!(
            from_rkyv, from_json,
            "every table, including the derived two"
        );

        // Row ORDER. `prefab.rs`' own pin already compares the decoded row vector against
        // `narrow_prefab_rows` field-by-field for all 1623 rows in file order, so it is not
        // restated here. What this lane needs on top of it is that order cannot *matter*: order
        // only decides which row wins a shared prefabId, and `tables_from_catalog` refuses a
        // catalogue that has one. Measured rather than asserted by hand — every row survives into
        // the id-keyed table, so no collapse happened on either side.
        let ordered_json = narrow_prefab_rows(&everon_prefabs_json_f32());
        assert_eq!(ordered_json.len(), EVERON_PREFABS, "rows in the export");
        assert_eq!(
            from_json.by_id.len(),
            ordered_json.len(),
            "a duplicate prefabId would make file order decide the table — everon has none"
        );

        // …and the same residency, end to end: the composed footprint, strip and glyph buffers are
        // the only consumers of `building_by_u16`, `fence_by_u16` and the glyph table.
        let a = everon_residency(&rkyv);
        let j = everon_residency(&json_f32);
        // Every lane the prefab tables feed must actually have composed something, or the equality
        // below is two empty buffers agreeing.
        assert!(!a.world_building_fill().is_empty(), "fill must compose");
        assert!(
            !a.world_building_outline().is_empty(),
            "outline must compose"
        );
        assert!(!a.world_fence_strips().is_empty(), "strips must compose");
        assert!(a.badge_glyph_count() > 0, "badges must compose");
        assert!(a.tree_glyph_count() > 0, "tree glyphs must compose");
        assert!(a.prop_glyph_count() > 0, "prop glyphs must compose");
        assert_eq!(a.draw_ids(), j.draw_ids(), "draw set");
        assert_eq!(a.world_building_fill(), j.world_building_fill(), "fill");
        assert_eq!(
            a.world_building_outline(),
            j.world_building_outline(),
            "outline"
        );
        assert_eq!(a.world_fence_strips(), j.world_fence_strips(), "strips");
        assert_eq!(
            (
                a.fence_strip_segment_count(),
                a.pier_strip_segment_count(),
                a.bridge_rail_strip_count()
            ),
            (
                j.fence_strip_segment_count(),
                j.pier_strip_segment_count(),
                j.bridge_rail_strip_count()
            ),
            "strip lane counts"
        );
        assert_eq!(a.world_tree_glyphs(), j.world_tree_glyphs(), "tree glyphs");
        assert_eq!(a.world_prop_glyphs(), j.world_prop_glyphs(), "prop glyphs");
        assert_eq!(
            a.world_badge_glyphs(),
            j.world_badge_glyphs(),
            "badge glyphs"
        );
        for group in 0..=2u8 {
            assert_eq!(
                a.glyph_lookup_len_for_group(group),
                j.glyph_lookup_len_for_group(group),
                "glyph table, group {group}"
            );
        }
        assert_eq!(a.stats_json(), j.stats_json(), "residency stats");
    }

    /// The f32 projection stated above is load-bearing, not paperwork: the RAW committed JSON and
    /// the archive genuinely disagree, on real everon rows. Without this the pin could be passing
    /// because everon happened to be f32-exact, and nobody would know which.
    #[test]
    fn the_f32_projection_is_not_a_no_op_on_everon() {
        let raw = tables_from_json(&everon_prefabs_json());
        let narrowed = tables_from_json(&everon_prefabs_json_f32());
        let differing = raw
            .building_by_u16
            .iter()
            .filter(|(k, v)| narrowed.building_by_u16.get(k) != Some(v))
            .count();
        assert!(
            differing > 0,
            "no building footprint changed under f32 — the everon export is now f32-exact and the \
             parity pin's projection needs re-stating rather than silently narrowing nothing"
        );
        assert_ne!(raw.by_id, narrowed.by_id, "row table under f32");
    }

    /* ─────────────────────────── the encoding corner everon lacks ─────────────────────── */

    /// A **real** `0.0` half-extent survives both lanes as `Some(0.0)`, not as absent. The wire's
    /// absent sentinel is `NaN` and zero is a legal value; `archives.rs`' doc said zero until
    /// T-935.14 and everon has no zero half-extent to catch the difference, so the corpus is built
    /// here. Every branch of both lookup twins is taken: a zero half-extent building (falls back to
    /// 2.0), a zero-half fence (falls back to 1.0/0.25), an absent-spatial building, a pier, and a
    /// row outside the u16 key domain.
    #[test]
    fn tables_agree_on_a_real_zero_half_extent() {
        let doc = serde_json::json!({ "prefabs": [
            { "prefabId": 4, "kind": "building", "class": "civic", "label": "Zero",
              "spatial": { "halfExtentsM": { "x": 0, "y": 0, "z": 0 }, "heightM": 0 },
              "render": { "iconKey": "building-civic", "baseSizePx": 18, "importanceZoom": -4 } },
            { "prefabId": 5, "kind": "prop", "class": "fence",
              "spatial": { "halfExtentsM": { "x": 0, "y": 0 } } },
            { "prefabId": 6, "kind": "building", "class": "hut" },
            { "prefabId": 7, "kind": "water", "class": "pier",
              "spatial": { "halfExtentsM": { "x": 10, "y": 1.5 } } },
            { "prefabId": 70000, "kind": "building", "class": "outside-u16",
              "spatial": { "halfExtentsM": { "x": 3, "y": 3 } } }
        ]});
        let rows = narrow_prefab_rows(&doc);
        assert_eq!(
            rows[0].half_x,
            Some(0.0),
            "the fixture must carry a real 0.0"
        );
        assert_eq!(rows[2].half_x, None, "…and a genuinely absent one");

        let archive = PrefabCatalogArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            prefabs: rows
                .iter()
                .enumerate()
                .map(|(i, r)| row_to_archive(i, r).expect("encode"))
                .collect(),
            type_inventory: TypeInventory {
                terrain_id: "probe".into(),
                census_status: "partial".into(),
                unique_prefabs: 5,
                total_instances: 0,
                by_kind: Vec::new(),
            },
        };
        let bytes = to_bytes(&archive).expect("serialise");

        let from_json = tables_from_json(&doc);
        let from_rkyv = tables_from_bytes(&bytes, "probe").expect("archive lane");
        assert_eq!(from_rkyv, from_json, "zero is a value, not an absence");

        // …and it is a value the two lanes still tell apart from absent.
        assert_eq!(from_rkyv.by_id[&4.0_f64.to_bits()].row.half_x, Some(0.0));
        assert_eq!(from_rkyv.by_id[&6.0_f64.to_bits()].row.half_x, None);
        // Both spellings fall back the same way in the lookups (`> 0.0`, not `is_some`).
        let zero = &from_rkyv.building_by_u16[&4];
        assert_eq!((zero.half_x, zero.half_y), (2.0, 2.0));
        assert_eq!(from_rkyv.building_by_u16[&6].half_x, 2.0);
        assert_eq!(
            from_rkyv.fence_by_u16[&5],
            FencePrefabInfo {
                half_x: 1.0,
                half_y: 0.25
            }
        );
        assert_eq!(from_rkyv.building_by_u16[&7].building_class, "pier");
        assert_eq!(
            from_rkyv.building_by_u16.len(),
            3,
            "70000 is outside the u16 lookup domain"
        );
        assert_eq!(from_rkyv.min_importance_zoom, Some(-4.0));
        assert_eq!(from_rkyv.importance_breakpoints, vec![-4.0]);
    }

    /* ─────────────────────────────────── the sniff ────────────────────────────────────── */

    /// Both formats reach the same tables through one entry point, and the archive bytes are
    /// provably not readable as JSON, so the archive side can only have come from the rkyv route.
    #[test]
    fn tables_from_bytes_sniffs_gzip_versus_rkyv() {
        let json_f32 = gzip(
            serde_json::to_string(&everon_prefabs_json_f32())
                .unwrap()
                .as_bytes(),
        );
        assert_eq!(&json_f32[..2], &[0x1f, 0x8b], "the JSON side must be gzip");
        let rkyv = everon_archive_bytes();
        assert_eq!(
            tables_from_bytes(&rkyv, "everon").expect("rkyv"),
            tables_from_bytes(&json_f32, "everon").expect("gz"),
            "both routes, one table"
        );
    }

    /// An empty buffer is refused before the two magic bytes are read — there are none to read, and
    /// "json parse failed: EOF" is a misleading thing to say about a file that never arrived.
    #[test]
    fn an_empty_payload_is_refused_before_the_sniff() {
        assert!(matches!(
            tables_from_bytes(&[], "everon"),
            Err(WorldError::EmptyPayload)
        ));
    }

    /// Truncation on either end lands in an error, never in the other parser — the head-truncated
    /// gzip is the case the sniff could plausibly get wrong.
    #[test]
    fn truncated_payloads_error_rather_than_reaching_the_wrong_parser() {
        let gz = gzip(br#"{"prefabs":[{"prefabId":9,"kind":"building","class":"hut"}]}"#);
        let rk = everon_archive_bytes();
        assert!(
            matches!(
                tables_from_bytes(&gz[..gz.len() / 2], "everon"),
                Err(WorldError::Gzip(_))
            ),
            "tail-truncated gzip keeps its magic and must fail as gzip"
        );
        assert!(
            matches!(
                tables_from_bytes(&gz[4..], "everon"),
                Err(WorldError::Archive(_))
            ),
            "head-truncated gzip has no magic and must be refused by the archive reader"
        );
        assert!(matches!(
            tables_from_bytes(&rk[..rk.len() - 8], "everon"),
            Err(WorldError::Archive(_))
        ));
        // Non-empty but too short to be gzip: must not index past its end.
        assert!(matches!(
            tables_from_bytes(&[0x1f], "everon"),
            Err(WorldError::Archive(_))
        ));
        // Bare (ungzipped) JSON has no magic, so it reaches the archive reader and is refused —
        // `load_prefabs_gz` remains the entry point for callers that know they hold JSON.
        assert!(matches!(
            tables_from_bytes(br#"{"prefabs":[]}"#, "everon"),
            Err(WorldError::Archive(_))
        ));
    }

    /// Rule 18 through this entry point: a catalogue built for another terrain is refused rather
    /// than served a table in which every id still resolves.
    #[test]
    fn a_catalogue_for_another_terrain_is_refused_by_the_sniff() {
        let rkyv = everon_archive_bytes();
        assert!(tables_from_bytes(&rkyv, "everon").is_ok(), "control");
        let msg = tables_from_bytes(&rkyv, "arland")
            .expect_err("must refuse")
            .to_string();
        assert!(msg.contains("everon") && msg.contains("arland"), "{msg}");
    }

    /// Two rows sharing a `prefabId` are refused rather than collapsed, because the JSON lane
    /// resolves the collision per lookup and the archive lane cannot see the order that decides it.
    #[test]
    fn a_catalogue_with_a_duplicate_prefab_id_is_refused() {
        let doc = serde_json::json!({ "prefabs": [
            { "prefabId": 3, "kind": "building", "class": "hut",
              "spatial": { "halfExtentsM": { "x": 4, "y": 4 } } },
            { "prefabId": 3, "kind": "tree", "class": "conifer",
              "spatial": { "halfExtentsM": { "x": 2, "y": 2 } } }
        ]});
        let rows = narrow_prefab_rows(&doc);
        // The divergence this refusal exists for, stated as a measurement rather than a claim:
        // the JSON lane keeps the TREE in the class table and the BUILDING in the footprint table.
        let json = tables_from_json(&doc);
        assert_eq!(json.by_id[&3.0_f64.to_bits()].row.kind, "tree");
        assert_eq!(json.building_by_u16[&3].building_class, "hut");

        let archive = PrefabCatalogArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            prefabs: rows
                .iter()
                .enumerate()
                .map(|(i, r)| row_to_archive(i, r).expect("encode"))
                .collect(),
            type_inventory: TypeInventory {
                terrain_id: "probe".into(),
                census_status: "partial".into(),
                unique_prefabs: 2,
                total_instances: 0,
                by_kind: Vec::new(),
            },
        };
        let bytes = to_bytes(&archive).expect("serialise");
        let msg = tables_from_bytes(&bytes, "probe")
            .expect_err("must refuse")
            .to_string();
        assert!(msg.contains("distinct prefabIds"), "{msg}");
    }
}
