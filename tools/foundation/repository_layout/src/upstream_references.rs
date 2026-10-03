//! The licensed upstream reference lanes.
//!
//! **Role:** the repository-relative paths of the gitignored reference folder and the two lanes
//! both `xtask` and `developer_tools` name.
//! **Position:** `xtask` fetches into the lanes and checks none of them reaches a shipped addon;
//! the `enf` commands of `developer_tools` index, carve and extract them.
//! **Signals & state:** none; constants.
//! **Invariants:** every lane lies under [`REFERENCES_DIR`]; a tool that writes a lane writes
//! inside that folder and refuses when it is absent.

/// The gitignored folder holding the licensed upstream reference trees beside its tracked
/// README.md. Nothing under it is committed or deployed.
pub const REFERENCES_DIR: &str = "apps/mod/References";

/// The Coalition Reforger Framework lane (Arma Public License): read and cite, never copy.
pub const CRF_FRAMEWORK_REFERENCE: &str = "apps/mod/References/crf_framework";

/// The vanilla lane: extracted Arma Reforger scripts and the official Script API pages (Bohemia
/// Interactive copyright).
pub const VANILLA_REFERENCE: &str = "apps/mod/References/vanilla_reference";
