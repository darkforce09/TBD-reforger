//! The change-scoped helpers: what did this slice touch, and which crate owns it.
//!
//! The default base is `main...HEAD`, which is the slice's own diff inside a WORKTREE and EMPTY on
//! merged main — so without an explicit base these silently check nothing exactly where it matters
//! most. Every caller in the wave gate passes `$base..HEAD`; the slice gate takes the default.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{Ctx, git_stdout_lossy, host, ledger};

/// The SPA crate, repo-relative — the root of the wasm dependency walk.
pub const FRONTEND_DIR: &str = "apps/website/frontend";
use crate::{wprint, wprintln};

/// The default diff base — the slice's own range inside a worktree.
pub const DEFAULT_BASE: &str = "main...HEAD";

#[cfg(test)]
#[path = "tests/changed/tests.rs"]
mod tests;

mod changed_rs;
pub use changed_rs::changed_rs;
pub use changed_rs::fmt_changed;
pub use changed_rs::frontend_tests_changed;
pub use changed_rs::owning_package_dir;
pub use changed_rs::realpath_m;
use changed_rs::rs_files_under;
pub use changed_rs::wasm_changed;
pub use changed_rs::wasm_scope_prefixes;
pub use changed_rs::wasm_scope_touched;

mod include_consumer_package_dirs;
pub use include_consumer_package_dirs::compiled_include_input_paths;
pub use include_consumer_package_dirs::include_consumer_package_dirs;
pub use include_consumer_package_dirs::include_inputs_under;
pub use include_consumer_package_dirs::workspace_members;

#[cfg(test)]
use changed_rs::{file_edition, frontend_include_input_touched, frontend_include_inputs, join_rel};
