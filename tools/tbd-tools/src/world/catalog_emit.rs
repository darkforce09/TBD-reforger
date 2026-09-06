//! T-935.11 — the rkyv twins of `objects/prefabs.json.gz`, `objects/type-inventory.json` and
//! `objects/forest-regions.json.gz`.
//!
//! `build-world-objects` writes each of these catalogues twice: the JSON the shipped loader still
//! fetches, and a Tier-2 archive beside it. Dual emission is deliberate and stays until T-935.13
//! flips the manifest — deleting either write before then blinds one reader.
//!
//! # Why it re-reads the JSON it just wrote instead of narrowing the builder's own model
//!
//! Same reason [`binary_emit`](super::binary_emit) and [`roads_emit`](super::roads_emit) do: the
//! JSON round trip is not the identity. `build_world_objects` holds `f64` values, writes them
//! through [`js_normalize`](super::build) and `js_num`, and the loader narrows *those*. Reading
//! the file that was just written puts the emitter on the loader's own chain
//! (`bytes_to_json` → `narrow_prefab_rows` / `parse_regions_payload`), so the archive equals the
//! JSON decode **by construction** rather than by coincidence — including the drops: a row the
//! loader rejects is absent from both.
//!
//! # Why the row encoding lives in `map-engine-core`, not here
//!
//! [`row_to_archive`] and [`region_to_archive`] are in `world/prefab.rs` and `world/regions.rs`,
//! next to the readers that invert them. The wire rows have no `Option`s while the parser rows
//! have eight, so "absent" needs a sentinel (NaN / `""` / alpha 0 / `u32::MAX`) and a sentinel
//! written in one crate and read in another is a contract with nobody holding both halves. This
//! module is therefore only the *file* half: which paths, read-back, refusals, sizes.
//!
//! # Three files, two of them versioned
//!
//! `prefabs.rkyv` and `forest-regions.rkyv` carry `schema_version`. `type-inventory.rkyv` does
//! not — [`TypeInventory`] has no such field (`binary/archives.rs`), so the standalone census is
//! the *convenience* copy and the one inside [`PrefabCatalogArchive`] is the checked one. Both are
//! written from a single value here, and `standalone_census_matches_the_catalogue` pins that.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use map_engine_core::world::binary::archives::{
    ARCHIVE_SCHEMA_VERSION, ForestRegionsArchive, PrefabCatalogArchive, TypeInventory,
};
use map_engine_core::world::binary::{access_checked, to_bytes};
use map_engine_core::world::{
    bytes_to_json, catalog_from_bytes, inventory_from_bytes, inventory_to_archive,
    narrow_prefab_rows, parse_regions_payload, region_to_archive, regions_from_bytes,
    row_to_archive,
};

/// The gz-JSON prefab catalogue, relative to a terrain directory.
pub const PREFABS_GZ: &str = "objects/prefabs.json.gz";
/// The plain-JSON census (this one is not gzipped — it is a human-read report).
pub const TYPE_INVENTORY_JSON: &str = "objects/type-inventory.json";
/// The gz-JSON land-cover regions.
pub const FOREST_REGIONS_GZ: &str = "objects/forest-regions.json.gz";

/// The rkyv catalogue. Matches the manifest's `objects.binary.prefabs`
/// (`map_engine_core::world::ObjectsBinaryBlock`), which T-935.13 points the SPA at.
pub const PREFAB_CATALOG_RKYV: &str = "objects/prefabs.rkyv";
/// The rkyv census — manifest `objects.binary.typeInventory`.
pub const TYPE_INVENTORY_RKYV: &str = "objects/type-inventory.rkyv";
/// The rkyv land-cover regions — manifest `objects.binary.regions`.
pub const FOREST_REGIONS_RKYV: &str = "objects/forest-regions.rkyv";

/// Read a terrain-relative JSON (gzipped or not) through the loader's own decode.
fn read_doc(terrain_dir: &Path, rel: &str) -> Result<serde_json::Value> {
    let src = terrain_dir.join(rel);
    let raw = std::fs::read(&src).with_context(|| format!("read {}", src.display()))?;
    bytes_to_json(&raw).map_err(|e| anyhow::anyhow!("{} decode: {e}", src.display()))
}

/// Serialise, **re-read through the validating reader**, then write. Returns the file size.
///
/// The read-back is not ceremony: `access_checked` is the only thing the SPA will ever use on this
/// file, so an archive that cannot survive it is a broken file whether or not `to_bytes` returned
/// `Ok`, and the emitter is the last place that can say so cheaply. (Three concrete functions
/// rather than one generic: naming the serializer/validator bounds would put `rkyv` in
/// `tbd-tools`' dependency list for no gain, and `map-engine-core` deliberately owns that.)
macro_rules! write_archive {
    ($fn_name:ident, $ty:ty, $what:literal) => {
        /// Serialise `archive`, validate the bytes with `access_checked`, then write them.
        ///
        /// # Errors
        /// When serialisation, validation or the write fails.
        pub fn $fn_name(path: &Path, archive: &$ty) -> Result<usize> {
            let bytes = to_bytes(archive).map_err(|e| anyhow::anyhow!("{e}"))?;
            access_checked::<$ty>(&bytes).map_err(|e| {
                anyhow::anyhow!(concat!($what, " fails its own validating read: {}"), e)
            })?;
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
            }
            std::fs::write(path, &bytes).with_context(|| format!("write {}", path.display()))?;
            Ok(bytes.len())
        }
    };
}

write_archive!(
    write_prefab_catalog_rkyv,
    PrefabCatalogArchive,
    "emitted prefabs.rkyv"
);
write_archive!(
    write_type_inventory_rkyv,
    TypeInventory,
    "emitted type-inventory.rkyv"
);
write_archive!(
    write_forest_regions_rkyv,
    ForestRegionsArchive,
    "emitted forest-regions.rkyv"
);

/// `prefabs.json.gz` + `type-inventory.json` → [`PrefabCatalogArchive`].
///
/// # Errors
/// When either source is missing or undecodable, when the catalogue narrows to nothing (T-537:
/// an empty catalogue is refused rather than written over a committed one), when the two
/// documents disagree about which terrain they describe, or when a row cannot be encoded without
/// changing its meaning (see [`row_to_archive`]).
pub fn build_prefab_catalog_archive(terrain_dir: &Path) -> Result<PrefabCatalogArchive> {
    let prefabs_doc = read_doc(terrain_dir, PREFABS_GZ)?;
    let inventory_doc = read_doc(terrain_dir, TYPE_INVENTORY_JSON)?;
    let rows = narrow_prefab_rows(&prefabs_doc);
    super::refuse_empty_write(
        "prefabs-rkyv",
        rows.is_empty(),
        "zero narrowed prefab rows — refusing to write an empty prefabs.rkyv",
    )?;
    let type_inventory =
        inventory_to_archive(&inventory_doc).map_err(|e| anyhow::anyhow!("{e}"))?;

    // The rows and the census have to be the same export of the same terrain. `terrainId` is
    // written into both documents by the same run of `build_world_objects`, so a disagreement
    // means one of the two files is stale — and the reader's own terrain check
    // (`catalog_from_bytes`) would then be validating against the wrong name.
    let rows_terrain = prefabs_doc.get("terrainId").and_then(|v| v.as_str());
    if rows_terrain != Some(type_inventory.terrain_id.as_str()) {
        bail!(
            "prefabs.json.gz terrainId {rows_terrain:?} != type-inventory.json terrainId {:?} — \
             one of the two catalogue files is stale",
            type_inventory.terrain_id
        );
    }
    if type_inventory.unique_prefabs as usize != rows.len() {
        bail!(
            "type-inventory.json declares {} unique prefabs but prefabs.json.gz narrows to {} \
             rows — the census does not describe the catalogue it ships with",
            type_inventory.unique_prefabs,
            rows.len()
        );
    }

    let mut prefabs = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        prefabs.push(row_to_archive(i, row).map_err(|e| anyhow::anyhow!("{e}"))?);
    }
    Ok(PrefabCatalogArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        prefabs,
        type_inventory,
    })
}

/// `forest-regions.json.gz` → [`ForestRegionsArchive`].
///
/// # Errors
/// When the source is missing or undecodable, when it narrows to zero regions (refused rather
/// than written), or when a region cannot be encoded without changing its meaning.
pub fn build_forest_regions_archive(terrain_dir: &Path) -> Result<ForestRegionsArchive> {
    let doc = read_doc(terrain_dir, FOREST_REGIONS_GZ)?;
    let parsed = parse_regions_payload(&doc);
    super::refuse_empty_write(
        "forest-regions-rkyv",
        parsed.is_empty(),
        "zero narrowed land-cover regions — refusing to write an empty forest-regions.rkyv",
    )?;
    let mut regions = Vec::with_capacity(parsed.len());
    for (i, r) in parsed.iter().enumerate() {
        regions.push(region_to_archive(i, r).map_err(|e| anyhow::anyhow!("{e}"))?);
    }
    Ok(ForestRegionsArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        regions,
    })
}

/// Build and write every catalogue archive for one terrain directory, then **read each one back
/// through the loader's public entry point** and check it against the JSON it came from.
///
/// The read-back is what makes dual emission a claim rather than a hope: `to_bytes` succeeding
/// says the value serialised, not that the loader will agree about what it means. So the emitter
/// runs `catalog_from_bytes` / `regions_from_bytes` / `inventory_from_bytes` on its own output —
/// terrain check, census check, class-code check and all — and a disagreement fails the export
/// here instead of on a user's machine.
///
/// Returns `(path, bytes)` per file written, in emit order. `forest-regions.rkyv` is written only
/// when its JSON exists (a non-density `--phase` does not produce one, and T-378 forbids touching
/// the committed density tree in that case); the skip is reported to the caller as an absent row,
/// never as a silent success.
///
/// # Errors
/// From any of the builders or writers, or when a written archive does not read back as its
/// source.
pub fn emit_catalog_archives(terrain_dir: &Path) -> Result<Vec<(PathBuf, usize)>> {
    let mut out = Vec::new();

    let catalog = build_prefab_catalog_archive(terrain_dir)?;
    let terrain = catalog.type_inventory.terrain_id.clone();
    let inventory = catalog.type_inventory.clone();
    let prefab_rows = catalog.prefabs.len();

    let p = terrain_dir.join(PREFAB_CATALOG_RKYV);
    out.push((p.clone(), write_prefab_catalog_rkyv(&p, &catalog)?));
    let read_back = catalog_from_bytes(
        &std::fs::read(&p).with_context(|| format!("read back {}", p.display()))?,
        &terrain,
    )
    .map_err(|e| anyhow::anyhow!("{} does not read back: {e}", p.display()))?;
    if read_back.by_id.len() != prefab_rows {
        bail!(
            "{} read back {} prefabs, expected {prefab_rows} — duplicate join keys in the \
             catalogue",
            p.display(),
            read_back.by_id.len()
        );
    }

    let p = terrain_dir.join(TYPE_INVENTORY_RKYV);
    out.push((p.clone(), write_type_inventory_rkyv(&p, &inventory)?));
    let back = inventory_from_bytes(
        &std::fs::read(&p).with_context(|| format!("read back {}", p.display()))?,
    )
    .map_err(|e| anyhow::anyhow!("{} does not read back: {e}", p.display()))?;
    if back != inventory {
        bail!(
            "{} does not read back as the census written into prefabs.rkyv",
            p.display()
        );
    }

    if terrain_dir.join(FOREST_REGIONS_GZ).exists() {
        let regions = build_forest_regions_archive(terrain_dir)?;
        let want = regions.regions.len();
        let p = terrain_dir.join(FOREST_REGIONS_RKYV);
        out.push((p.clone(), write_forest_regions_rkyv(&p, &regions)?));
        let back = regions_from_bytes(
            &std::fs::read(&p).with_context(|| format!("read back {}", p.display()))?,
        )
        .map_err(|e| anyhow::anyhow!("{} does not read back: {e}", p.display()))?;
        if back.len() != want {
            bail!(
                "{} read back {} regions, expected {want}",
                p.display(),
                back.len()
            );
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use map_engine_core::world::parse_manifest_binary;

    use super::*;
    use crate::serve::repo_root;

    /// The committed everon export: 1623 prefabs, 36 land-cover regions, 1,216,066 instances.
    /// (`map-engine-core`'s `store.rs` census pin says the same three numbers.) Re-pin
    /// deliberately if the export changes — a silently shrinking corpus is how a parity test
    /// stops proving anything.
    const EVERON_PREFABS: usize = 1623;
    const EVERON_REGIONS: usize = 36;
    const EVERON_INSTANCES: u64 = 1_216_066;

    fn everon_dir() -> PathBuf {
        repo_root().join("packages/map-assets/everon")
    }

    fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("t935-11-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("objects")).expect("tempdir");
        d
    }

    /// A terrain directory holding copies of everon's three catalogue JSONs and nothing else.
    fn staged_everon(tag: &str) -> PathBuf {
        let d = tmp_dir(tag);
        for rel in [PREFABS_GZ, TYPE_INVENTORY_JSON, FOREST_REGIONS_GZ] {
            std::fs::copy(everon_dir().join(rel), d.join(rel))
                .unwrap_or_else(|e| panic!("copy {rel}: {e}"));
        }
        d
    }

    /// THE SLICE'S EMIT PIN. Run the emitter over the committed everon catalogue JSONs and prove
    /// all three archives land, are not JSON and not gzip, and read back through the shipped
    /// loader entry points as the very structures the JSON path yields.
    #[test]
    fn everon_catalog_archives_emit_and_read_back_as_their_json() {
        let dir = staged_everon("emit");
        assert!(!dir.join(PREFAB_CATALOG_RKYV).exists(), "precondition");

        let written = emit_catalog_archives(&dir).expect("emit");
        assert_eq!(written.len(), 3, "three archives: {written:?}");
        for (p, n) in &written {
            assert_eq!(
                std::fs::metadata(p).expect("stat").len() as usize,
                *n,
                "{} size",
                p.display()
            );
            assert!(*n > 0, "{} is empty", p.display());
            let bytes = std::fs::read(p).expect("read");
            assert!(
                bytes_to_json(&bytes).is_err(),
                "{} parses as JSON — the rkyv route did not run",
                p.display()
            );
            assert_ne!(&bytes[..2], &[0x1f, 0x8b], "{} is gzip", p.display());
        }
        assert_eq!(written[0].0, dir.join(PREFAB_CATALOG_RKYV));
        assert_eq!(written[1].0, dir.join(TYPE_INVENTORY_RKYV));
        assert_eq!(written[2].0, dir.join(FOREST_REGIONS_RKYV));

        // The catalogue: same rows, in the same ORDER, and the file is exactly the value the
        // builder produced.
        //
        // The order half is not decoration. `by_id` below is a hash map, so it is blind to a
        // reordered catalogue — measured: a `rows_from_archive` that rotates the row vector by one
        // leaves every assertion on `by_id` passing. The reader side of that is pinned in
        // `map-engine-core`'s `everon_catalogue_archive_equals_the_json_rows`; what is pinned HERE
        // is the writer side, JSON order → archive order → file bytes.
        let json_rows = narrow_prefab_rows(&read_doc(&dir, PREFABS_GZ).expect("prefabs json"));
        assert_eq!(json_rows.len(), EVERON_PREFABS);
        let rebuilt = build_prefab_catalog_archive(&dir).expect("rebuild");
        assert_eq!(rebuilt.prefabs.len(), json_rows.len());
        for (i, (a, j)) in rebuilt.prefabs.iter().zip(json_rows.iter()).enumerate() {
            assert_eq!(
                f64::from(a.prefab_id),
                j.prefab_id,
                "row {i}: the archive is not in prefabs.json.gz order"
            );
        }
        assert_eq!(
            std::fs::read(dir.join(PREFAB_CATALOG_RKYV)).expect("read"),
            to_bytes(&rebuilt).expect("serialise").as_slice(),
            "the file on disk is not the catalogue the builder produced"
        );

        let cat = catalog_from_bytes(
            &std::fs::read(dir.join(PREFAB_CATALOG_RKYV)).expect("read"),
            "everon",
        )
        .expect("catalogue");
        assert_eq!(cat.by_id.len(), EVERON_PREFABS);
        assert_eq!(cat.terrain_id, "everon");
        assert_eq!(cat.total_instances, EVERON_INSTANCES);
        for row in &json_rows {
            let entry = cat
                .by_id
                .get(&row.prefab_id.to_bits())
                .unwrap_or_else(|| panic!("prefab {} missing from the archive", row.prefab_id));
            assert_eq!(entry.row.kind, row.kind, "prefab {}", row.prefab_id);
            assert_eq!(entry.row.class, row.class, "prefab {}", row.prefab_id);
            assert_eq!(entry.row.label, row.label, "prefab {}", row.prefab_id);
        }

        // The regions: same set, same rings.
        let json_regions =
            parse_regions_payload(&read_doc(&dir, FOREST_REGIONS_GZ).expect("regions json"));
        assert_eq!(json_regions.len(), EVERON_REGIONS);
        let back = regions_from_bytes(&std::fs::read(dir.join(FOREST_REGIONS_RKYV)).expect("read"))
            .expect("regions");
        assert_eq!(back.len(), EVERON_REGIONS);
        let mut vertices = 0usize;
        for (a, j) in back.iter().zip(json_regions.iter()) {
            assert_eq!(a.id, j.id);
            assert_eq!(a.kind, j.kind);
            assert_eq!(a.tree_count, j.tree_count, "region {}", j.id);
            assert_eq!(a.polygon.len(), j.polygon.len(), "region {}", j.id);
            for (ra, rj) in a.polygon.iter().zip(j.polygon.iter()) {
                assert_eq!(ra.len(), rj.len(), "region {} ring length", j.id);
                vertices += ra.len();
            }
        }
        assert!(
            vertices >= 2_000,
            "only {vertices} ring vertices — the corpus is empty or LFS-pointered, so this test \
             proved nothing"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The census is written into two files from one value, and they agree. Without this the
    /// standalone file could drift from the copy inside the catalogue and nothing would notice —
    /// `TypeInventory` carries no `schema_version` of its own to catch it later.
    #[test]
    fn standalone_census_matches_the_catalogue() {
        let dir = staged_everon("census");
        emit_catalog_archives(&dir).expect("emit");
        let standalone =
            inventory_from_bytes(&std::fs::read(dir.join(TYPE_INVENTORY_RKYV)).expect("read"))
                .expect("standalone census");
        let embedded = build_prefab_catalog_archive(&dir)
            .expect("catalogue")
            .type_inventory;
        assert_eq!(standalone, embedded);
        assert_eq!(standalone.unique_prefabs as usize, EVERON_PREFABS);
        assert_eq!(standalone.total_instances, EVERON_INSTANCES);
        assert_eq!(
            standalone.by_kind.iter().map(|k| k.instances).sum::<u64>(),
            EVERON_INSTANCES,
            "the per-kind census must add up to the declared total"
        );
        // T-946.19 — the ORDER, on the emitter side too. A sum is permutation-blind, and
        // `by_kind` is an order contract (`INSTANCE_KINDS`, `road` last): the wave-241 verifier
        // showed `by_kind[1..8]` could be shuffled with every test in both crates still green.
        assert_eq!(
            standalone
                .by_kind
                .iter()
                .map(|k| k.kind.as_str())
                .collect::<Vec<_>>(),
            crate::world::INSTANCE_KINDS.to_vec(),
            "the emitted census must keep `INSTANCE_KINDS` order"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An empty catalogue is refused rather than written over a committed archive (T-537's rule,
    /// applied to the binary lane) — and nothing is written when it is refused.
    #[test]
    fn an_empty_catalogue_is_refused() {
        let dir = tmp_dir("empty");
        std::fs::write(dir.join(PREFABS_GZ), br#"{"terrainId":"x","prefabs":[]}"#).expect("write");
        std::fs::write(
            dir.join(TYPE_INVENTORY_JSON),
            br#"{"terrainId":"x","censusStatus":"partial","levels":{"uniquePrefabs":0,"totalInstances":0},"byKind":{}}"#,
        )
        .expect("write");
        let msg = format!(
            "{:#}",
            emit_catalog_archives(&dir).expect_err("must refuse")
        );
        assert!(msg.contains("refusing empty write (prefabs-rkyv)"), "{msg}");
        assert!(!dir.join(PREFAB_CATALOG_RKYV).exists(), "nothing written");

        std::fs::write(dir.join(FOREST_REGIONS_GZ), br#"{"regions":[]}"#).expect("write");
        let msg = format!(
            "{:#}",
            build_forest_regions_archive(&dir).expect_err("must refuse")
        );
        assert!(
            msg.contains("refusing empty write (forest-regions-rkyv)"),
            "{msg}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The two catalogue documents have to describe the same export. A stale census beside a
    /// fresh catalogue is exactly the failure a count-only check would miss when the counts
    /// happen to match.
    #[test]
    fn a_census_from_another_export_is_refused() {
        let dir = staged_everon("stale");
        let mut inv: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join(TYPE_INVENTORY_JSON)).expect("read"))
                .expect("parse");
        inv["terrainId"] = serde_json::json!("arland");
        std::fs::write(dir.join(TYPE_INVENTORY_JSON), inv.to_string()).expect("write");
        let msg = format!(
            "{:#}",
            build_prefab_catalog_archive(&dir).expect_err("must refuse")
        );
        assert!(
            msg.contains("one of the two catalogue files is stale"),
            "{msg}"
        );

        inv["terrainId"] = serde_json::json!("everon");
        inv["levels"]["uniquePrefabs"] = serde_json::json!(EVERON_PREFABS - 1);
        std::fs::write(dir.join(TYPE_INVENTORY_JSON), inv.to_string()).expect("write");
        let msg = format!(
            "{:#}",
            build_prefab_catalog_archive(&dir).expect_err("must refuse")
        );
        assert!(
            msg.contains("does not describe the catalogue it ships with"),
            "{msg}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The hop nothing else covers: the paths this emitter **writes** have to be the paths the
    /// manifest **names**, or `world_host`'s archive branches fetch a URL nothing ever wrote — a
    /// 404 that falls silently back to JSON on every boot, which is indistinguishable from "the
    /// binary lane is switched off".
    ///
    /// This is also the only place both halves are visible: the emitter constants live in
    /// `tbd-tools`, the manifest parser in `map-engine-core`.
    ///
    /// The second half is the acceptance's own words, made mechanical: the committed everon
    /// manifest must carry **no** `objects.binary` block, so both frontend archive branches stay
    /// dormant until T-935.13 writes one. If that ever stops being true by accident, this fails
    /// here rather than by the editor quietly changing which files it fetches.
    #[test]
    fn the_manifest_block_names_the_paths_this_emitter_writes() {
        let flipped = serde_json::json!({
            "objects": {
                "prefabsPath": "objects/prefabs.json.gz",
                "chunksPath": "objects/chunks",
                "binary": {
                    "schemaVersion": "1.0.0",
                    "container": "TBDC", "containerVersion": 1,
                    "pod": "ObjectInstancePod", "podBytes": 32,
                    "chunks": "objects/chunks/{cx}_{cy}.bin",
                    "prefabs": "objects/prefabs.rkyv",
                    "roads": "roads/road_network.rkyv",
                    "regions": "objects/forest-regions.rkyv",
                    "typeInventory": "objects/type-inventory.rkyv"
                }
            }
        });
        let block = parse_manifest_binary(&flipped)
            .objects
            .expect("objects.binary");
        assert_eq!(block.prefabs, PREFAB_CATALOG_RKYV);
        assert_eq!(block.regions, FOREST_REGIONS_RKYV);
        assert_eq!(block.type_inventory, TYPE_INVENTORY_RKYV);
        assert_eq!(block.roads, super::super::roads_emit::ROAD_NETWORK_RKYV);
        assert!(
            !block.prefabs.is_empty() && !block.regions.is_empty(),
            "world_host reads an empty path as \"not named\" — these must be non-empty"
        );

        let everon: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(everon_dir().join("manifest.json")).expect("everon manifest"),
        )
        .expect("parse everon manifest");
        assert!(
            parse_manifest_binary(&everon).objects.is_none(),
            "the committed everon manifest gained an objects.binary block — T-935.11's acceptance \
             is that JSON stays the default until T-935.13 writes one"
        );
    }

    /// A terrain with no `forest-regions.json.gz` (a non-density `--phase`) emits the two
    /// catalogue archives and reports two rows — it does not invent an empty regions file, and it
    /// does not fail.
    #[test]
    fn a_terrain_without_regions_emits_only_the_catalogue() {
        let dir = tmp_dir("no-regions");
        for rel in [PREFABS_GZ, TYPE_INVENTORY_JSON] {
            std::fs::copy(everon_dir().join(rel), dir.join(rel)).expect("copy");
        }
        let written = emit_catalog_archives(&dir).expect("emit");
        assert_eq!(written.len(), 2, "{written:?}");
        assert!(!dir.join(FOREST_REGIONS_RKYV).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
