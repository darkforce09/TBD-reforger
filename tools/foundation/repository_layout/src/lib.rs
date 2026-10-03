//! The repository locations the tools share, and the one walk that finds a checkout root.
//!
//! **Role:** [`find_repository_root`] and [`find_repository_root_from`] walk up to the folder
//! holding [`ROOT_MARKER`]; the constants name the ticket registry, the agent artifact tree, the
//! documentation the tools read, the upstream reference lanes and the build output folder, each
//! once, as repository-relative paths a caller joins onto the root it found.
//! **Position:** tier 0 of `tools/foundation`, with no workspace dependency. `ticket_engine`,
//! `developer_tools`, `xtask` and `ticketboard` find their checkout and spell these shared
//! locations through it; a location only one tool names stays in that tool's own layout module.
//! **Signals & state:** none; constants and a read-only walk of the filesystem.
//! **Invariants:** a root is a folder holding the marker file, so a worktree nested under another
//! checkout resolves to itself; a walk that reaches the filesystem root is an [`Error`], never a
//! guessed folder; every location is relative and uses `/` separators.

mod agent_artifacts;
mod build_output;
pub mod documentation;
mod error;
pub mod prelude;
mod repository_root;
mod ticket_registry;
mod upstream_references;

pub use agent_artifacts::{ARTIFACTS_DIR, LAST_VERIFIED_MARKER, VERDICTS_DIR, WORKTREES_DIR};
pub use build_output::BUILD_OUTPUT_FOLDER;
pub use error::{Error, Result};
pub use repository_root::{
    ROOT_MARKER, find_repository_root, find_repository_root_from, is_repository_root,
};
pub use ticket_registry::{
    CORPUS_PINS, ESTIMATES_DIR, ESTIMATES_SCHEMA, METRICS_DIR, METRICS_SCHEMA, QUEUE_JSON, SCHEMA,
    SCOPE_VOCAB, TICKETS_DIR, WAVE_LOCK,
};
pub use upstream_references::{CRF_FRAMEWORK_REFERENCE, REFERENCES_DIR, VANILLA_REFERENCE};
