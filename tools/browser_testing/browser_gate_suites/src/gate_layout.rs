//! The repository locations only the browser gates name.
//!
//! **Role:** the pair of map-asset folders the gates' static server mounts under `/map-assets`,
//! the single-page app's release build folder every gate serves by default, and the editor gate
//! runbook the font-cache diagnostic names.
//! **Position:** the static server, the `gate` command line, the smokes and the doctor read these;
//! the folders themselves are spelled once, in the `repository_layout` crate.
//! **Signals & state:** none; a value type and constants.
//! **Invariants:** terrains and glyphs travel together, so no server mounts one without the other.

use std::path::{Path, PathBuf};

/// The single-page app's `trunk build --release` output folder, relative to the checkout root:
/// the `dist` folder its `Trunk.toml` names beside the app crate. Every gate that serves a build
/// defaults to it.
pub const FRONTEND_APPLICATION_DIST: &str = "crates/frontend/shell/frontend_application/dist";

/// The known wedge modes of the headless editor gate and the recipe for each, named by the
/// font-cache diagnostic when it cannot explain what it found.
pub const EDITOR_GATE_RUNBOOK: &str = "documentation/runbooks/editor_gates.md";

/// The pair of directories the map client reaches under a single `/map-assets` URL prefix.
///
/// Terrains and glyphs are separate on disk because glyphs are shared by every terrain, and they
/// are joined under one prefix by whatever is serving them. Carrying them as one value keeps a
/// caller from wiring the terrain mount and forgetting the glyph mount, which does not fail at
/// startup: the map renders, and every icon is missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapAssetMounts {
    /// The terrain datasets, served at `/map-assets`.
    pub terrains: PathBuf,
    /// The glyph atlases, served at `/map-assets/glyphs`.
    pub glyphs: PathBuf,
}

impl MapAssetMounts {
    /// Both directories as they sit in a checkout.
    pub fn from_root(root: &Path) -> Self {
        Self {
            terrains: ::repository_layout::terrain_assets_dir(root),
            glyphs: ::repository_layout::glyph_assets_dir(root),
        }
    }

    /// The pair implied by a terrain directory, wherever it has been placed.
    ///
    /// Glyphs are the terrain tree's sibling, so a deployment that relocates one relocates both.
    /// A terrain path with no parent yields a bare `glyphs`, which is the correct relative answer.
    pub fn beside_terrains(terrains: PathBuf) -> Self {
        let glyphs = terrains
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join("glyphs");
        Self { terrains, glyphs }
    }
}

#[cfg(test)]
#[path = "tests/gate_layout_tests.rs"]
mod tests;
