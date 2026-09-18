//! T-935.11 — the rkyv twins of `objects/prefabs.json.gz`, `objects/type-inventory.json` and
//! `objects/forest-regions.json.gz`.
//!
//! `build-world-objects` writes each of these catalogues twice: the JSON the shipped loader still
//! fetches, and a Tier-2 archive beside it. Dual emission is deliberate and stays until T-935.13
//! flips the manifest — deleting either write before then blinds one reader.
//!
//! # Why it re-reads the JSON it just wrote instead of narrowing the builder's own model
//!
//! Same reason `binary_emit` and `roads_emit` do: the
//! JSON round trip is not the identity. `build_world_objects` holds `f64` values, writes them
//! through `js_normalize` and `js_num`, and the loader narrows *those*. Reading
//! the file that was just written puts the emitter on the loader's own chain
//! (`bytes_to_json` → `narrow_prefab_rows` / `parse_regions_payload`), so the archive equals the
//! JSON decode **by construction** rather than by coincidence — including the drops: a row the
//! loader rejects is absent from both.
//!
//! # Why the row encoding lives in `map-engine-core`, not here
//!
//! `row_to_archive` and `region_to_archive` are in `world/prefab.rs` and `world/regions.rs`,
//! next to the readers that invert them. The wire rows have no `Option`s while the parser rows
//! have eight, so "absent" needs a sentinel (NaN / `""` / alpha 0 / `u32::MAX`) and a sentinel
//! written in one crate and read in another is a contract with nobody holding both halves. This
//! module is therefore only the *file* half: which paths, read-back, refusals, sizes.
//!
//! # Three files, two of them versioned
//!
//! `prefabs.rkyv` and `forest-regions.rkyv` carry `schema_version`. `type-inventory.rkyv` does
//! not — `TypeInventory` has no such field (`binary/archives.rs`), so the standalone census is
//! the *convenience* copy and the one inside `PrefabCatalogArchive` is the checked one. Both are
//! written from a single value here, and `standalone_census_matches_the_catalogue` pins that.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use website_map_engine::io::archives::codec::access_checked;
use website_map_engine::io::archives::codec::to_bytes;
use website_map_engine::io::archives::forest::ForestRegionsArchive;
use website_map_engine::io::archives::prefabs::PrefabCatalogArchive;
use website_map_engine::io::archives::prefabs::TypeInventory;
use website_map_engine::io::archives::version::ARCHIVE_SCHEMA_VERSION;
use website_map_engine::streaming::loaders::store::bytes_to_json;
use website_map_engine::world::environment::buildings::prefab::catalog_from_bytes;
use website_map_engine::world::environment::buildings::prefab::inventory_from_bytes;
use website_map_engine::world::environment::buildings::prefab::inventory_to_archive;
use website_map_engine::world::environment::buildings::prefab::narrow_prefab_rows;
use website_map_engine::world::environment::buildings::prefab::row_to_archive;
use website_map_engine::world::environment::vegetation::regions::parse_regions_payload;
use website_map_engine::world::environment::vegetation::regions::region_to_archive;
use website_map_engine::world::environment::vegetation::regions::regions_from_bytes;

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

/// `prefabs.json.gz` + `type-inventory.json` → `PrefabCatalogArchive`.
///
/// # Errors
/// When either source is missing or undecodable, when the catalogue narrows to nothing (T-537:
/// an empty catalogue is refused rather than written over a committed one), when the two
/// documents disagree about which terrain they describe, or when a row cannot be encoded without
/// changing its meaning (see `row_to_archive`).
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
#[path = "tests/catalog_emit/tests.rs"]
mod tests;
