//! The repository locations only the world-export pipeline names.
//!
//! **Role:** the committed density fixtures the export re-densifies from, and the per-terrain
//! export operation log and type inventory, each as a function of a checkout root the caller
//! passes.
//! **Position:** the builders, the census and the export checks of this crate read and write these;
//! the locations more than one tool names (the contract tree, the terrain and glyph trees, the
//! export scratch, the agent artifact tree) come from the `repository_layout` crate.
//! **Signals & state:** none; pure path joins.
//! **Invariants:** every location lies under the checkout root it is given; the operation log and
//! the type inventory are committed export records beside the map fixtures.

use std::path::{Path, PathBuf};

/// Committed forest-density fixtures, read when re-densifying without a Workbench export.
pub fn density_fixtures_dir(root: &Path) -> PathBuf {
    repository_layout::map_fixtures_dir(root).join("density")
}

/// The committed export records: the operation log and the type inventory of the export that
/// produced each terrain's committed map assets, where the export verifiers read them back.
pub fn export_records_dir(root: &Path) -> PathBuf {
    repository_layout::map_fixtures_dir(root).join("export_records")
}

/// One terrain's export operation log: every stage that ran, with what it produced.
pub fn export_operations_log(root: &Path, terrain: &str) -> PathBuf {
    export_records_dir(root).join(format!("map_export_{terrain}.json"))
}

/// One terrain's object type inventory: every world-object type the export saw, and its census
/// status.
pub fn object_type_inventory(root: &Path, terrain: &str) -> PathBuf {
    export_records_dir(root).join(format!("type_inventory_{terrain}.json"))
}
