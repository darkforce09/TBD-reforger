//! The licensed upstream reference lanes.
//!
//! **Role:** the repository-relative paths of the gitignored reference folder, the two lanes
//! both `xtask` and `developer_tools` name, and the PlayableSelector lane with the variable that
//! points it elsewhere.
//! **Position:** `xtask` fetches into the lanes, links the PlayableSelector lane into a slice
//! worktree and checks none of them reaches a shipped addon; the `enf` commands of
//! `developer_tools` index, carve and extract them.
//! **Signals & state:** none; constants.
//! **Invariants:** every lane lies under [`REFERENCES_DIR`]; a tool that writes a lane writes
//! inside that folder and refuses when it is absent.

/// The gitignored folder holding the licensed upstream reference trees beside its tracked
/// README.md. Nothing under it is committed or deployed.
pub const REFERENCES_DIR: &str = "mod/References";

/// The Coalition Reforger Framework lane (Arma Public License): read and cite, never copy.
pub const CRF_FRAMEWORK_REFERENCE: &str = "mod/References/crf_framework";

/// The vanilla lane: extracted Arma Reforger scripts and the official Script API pages (Bohemia
/// Interactive copyright).
pub const VANILLA_REFERENCE: &str = "mod/References/vanilla_reference";

/// The PlayableSelector checkout, which carries no licence: design mirror only.
pub const PLAYABLE_SELECTOR_REFERENCE: &str = "mod/References/playable_selector";

/// An environment variable naming another PlayableSelector checkout. When it is set and not
/// empty it replaces [`PLAYABLE_SELECTOR_REFERENCE`] as the lane's source.
pub const PLAYABLE_SELECTOR_OVERRIDE_ENV: &str = "TBD_PS_ORACLE";
