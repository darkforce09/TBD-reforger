//! The change-scoped helpers: what a slice or wave touched, and which crate owns each file.
//!
//! **Role:** names the frontend crate and the default diff base, and re-exports the changed-file
//! readers, the format and wasm checks, the wasm scope and the include-input resolution of its two
//! submodules.
//!
//! **Position:** called by both gates (`gate`), `touch` and `test_cmd`; the bodies live in
//! `changed_rs.rs` and `include_consumer_package_dirs.rs`.
//!
//! **Signals & state:** none; constants and re-exports.
//!
//! **Invariants:** the default base `main...HEAD` is the slice's own diff inside a worktree and
//! empty on merged main, so the wave gate always passes an explicit `<base>..HEAD` and the slice
//! gate takes the default.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{Ctx, git_stdout_lossy, host, ledger};

/// The SPA crate, repo-relative — the root of the wasm dependency walk.
pub(crate) const FRONTEND_DIR: &str = "crates/frontend/shell/frontend_application";
use crate::wave_execution::{wprint, wprintln};

/// The default diff base — the slice's own range inside a worktree.
pub(crate) const DEFAULT_BASE: &str = "main...HEAD";

mod changed_rs;
pub(crate) use changed_rs::changed_rs;
pub(crate) use changed_rs::fmt_changed;
pub(crate) use changed_rs::frontend_tests_changed;
pub(crate) use changed_rs::owning_package_dir;
pub(crate) use changed_rs::realpath_m;
use changed_rs::rs_files_under;
pub(crate) use changed_rs::wasm_scope_prefixes;
pub(crate) use changed_rs::wasm_scope_touched;

mod include_consumer_package_dirs;
pub(crate) use include_consumer_package_dirs::compiled_include_input_paths;
pub(crate) use include_consumer_package_dirs::include_consumer_package_dirs;
pub(crate) use include_consumer_package_dirs::include_inputs_under;
pub(crate) use include_consumer_package_dirs::repository_reads_under;
pub(crate) use include_consumer_package_dirs::workspace_members;
