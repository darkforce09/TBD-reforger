//! Step lists of the map-lane rows that rebuild Everon's served basemap assets.
//!
//! **Role:** Holds the multi-step recipes of the `map-water-everon` and `map-cartographic-everon`
//! rows of [`super::TASKS`], each a sequence of `developer-tools` `map` subcommands.
//! **Position:** Pulled into `task_definitions.rs` by `#[path]`; read only by those two rows.
//! **Signals & state:** None; constant data.
//! **Invariants:** Every step is a shell-free subprocess line or a named task row, so the task
//! table's `cmd_lines_are_shell_free` test covers these steps as it covers inline ones.

use super::Step;

/// Everon water composite: restore the pre-water ortho, mask, composite, bundle and pyramid,
/// then verify each product.
pub(super) const MAP_WATER_EVERON_STEPS: &[Step] = &[
    // Coreutils `cp`, not `std::fs::copy`: assets_v2/scratch/ is gitignored, so a missing
    // source is the common path, and cp's own "cannot stat" diagnostic reports it.
    sh!(
        "cp assets_v2/scratch/everon/sap/everon-sap-ortho.pre-water.png assets_v2/scratch/everon/sap/everon-sap-ortho.png"
    ),
    sh!("cargo run -q -p developer-tools --bin map -- reset-water-meta --terrain everon"),
    sh!("cargo run -q -p developer-tools --bin map -- analyze-water"),
    sh!("cargo run -q -p developer-tools --bin map -- composite-water"),
    sh!(
        "cargo run -q -p developer-tools --bin map -- build-unified --input assets_v2/scratch/everon/sap/everon-sap-ortho.png --out assets_v2/terrains/everon/satellite/everon-sat.tbd-sat --terrain everon"
    ),
    sh!("cargo run -q -p developer-tools --bin map -- patch-unified-bytes --terrain everon"),
    sh!(
        "cargo run -q -p developer-tools --bin map -- build-pyramid --input assets_v2/scratch/everon/sap/everon-sap-ortho.png --out assets_v2/terrains/everon/tiles/satellite --minzoom 0 --maxzoom 6 --tilesize 256 --lossless"
    ),
    sh!("cargo run -q -p developer-tools --bin map -- verify-sap-ortho --terrain everon"),
    sh!("cargo run -q -p developer-tools --bin map -- verify-unified --terrain everon"),
    sh!(
        "cargo run -q -p developer-tools --bin map -- verify-pyramid --terrain everon --expect-lossless"
    ),
];

/// Everon cartographic Map view: staging ortho, pyramid, manifest patch, then the
/// `map-cartographic-verify` row.
pub(super) const MAP_CARTOGRAPHIC_EVERON_STEPS: &[Step] = &[
    sh!("cargo run -q -p developer-tools --bin map -- build-cartographic --terrain everon"),
    sh!(
        "cargo run -q -p developer-tools --bin map -- build-pyramid --input assets_v2/scratch/everon/map/everon-map-ortho.png --out assets_v2/terrains/everon/tiles/map --minzoom 0 --maxzoom 6 --tilesize 256"
    ),
    sh!("cargo run -q -p developer-tools --bin map -- patch-map-tiles-meta --terrain everon"),
    Step::Task("map-cartographic-verify"),
];
