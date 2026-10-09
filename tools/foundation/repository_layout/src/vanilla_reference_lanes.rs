//! The folders inside the vanilla reference lane.
//!
//! **Role:** the repository-relative paths of the extracted scripts, the Script API pages, the
//! source pages and the reconstructed sources inside [`crate::VANILLA_REFERENCE`].
//! **Position:** `xtask fetch` caches the Script API and source pages; the `enf` commands of
//! `developer_tools` extract the scripts, parse the pages and rebuild the sources.
//! **Signals & state:** none; constants.
//! **Invariants:** every folder lies under [`crate::VANILLA_REFERENCE`], so nothing here is
//! committed or deployed.

/// Vanilla scripts `enf extract` copies out of the pak file table by name.
pub const VANILLA_EXTRACTED_SCRIPTS: &str = "mod/References/vanilla_reference/Scripts";

/// The official Script API pages `cargo xtask fetch vanilla-api` caches and `enf apidoc` parses.
pub const VANILLA_SCRIPT_API_PAGES: &str = "mod/References/vanilla_reference/apidoc";

/// The source pages `cargo xtask fetch vanilla-source` caches and `enf source` reads.
pub const VANILLA_SOURCE_PAGES: &str = "mod/References/vanilla_reference/source_html";

/// Vanilla `.c` files `enf source` rebuilds, method bodies included, from the source pages.
pub const VANILLA_RECONSTRUCTED_SOURCE: &str = "mod/References/vanilla_reference/Source";

#[cfg(test)]
#[path = "tests/vanilla_reference_lanes_tests.rs"]
mod tests;
