//! The one walk that finds the checkout root.
//!
//! **Role:** [`find_repository_root`] and [`find_repository_root_from`] walk up to the nearest
//! folder holding [`ROOT_MARKER`]; [`is_repository_root`] confirms a folder a caller already holds.
//! **Position:** foundation tier 0, depending on no workspace crate. The tools, the API's tests and
//! the frontend crates' tests find the checkout through it, then join their own relative locations
//! (`repository_layout` names the ones the tools share) onto the answer.
//! **Signals & state:** none; a read-only walk of the filesystem.
//! **Invariants:** a root is a folder holding the marker file, so a worktree nested under another
//! checkout resolves to itself; a walk that reaches the filesystem root is an [`Error`], never a
//! guessed folder.

mod error;
pub mod prelude;
mod root_marker_walk;

pub use error::{Error, Result};
pub use root_marker_walk::{
    ROOT_MARKER, find_repository_root, find_repository_root_from, is_repository_root,
};
