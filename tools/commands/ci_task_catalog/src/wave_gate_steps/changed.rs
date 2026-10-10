//! The change-scoped helpers: what a slice or wave touched, and which crate owns each file.
//!
//! **Role:** names the frontend crate and the default diff range, and re-exports the changed-file
//! readers, the format check, the wasm scope and the include-input resolution of its two
//! submodules.
//! **Position:** used by `touch`, `trunk_build`, `changed_packages` and the step dispatch; the
//! bodies live in `changed/changed_files.rs` and `changed/include_inputs.rs`.
//! **Signals & state:** none; constants and re-exports.
//! **Invariants:** the default range `main...HEAD` is the slice's own diff inside a worktree and
//! empty on merged main, so the wave gate always passes an explicit `<base>..HEAD`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::step_context::{Ctx, git_porcelain_paths, git_stdout_lossy};
use super::{host, wprint, wprintln};

/// The single-page app crate, repository-relative: the root of the wasm dependency walk.
pub(crate) const FRONTEND_DIR: &str = "crates/frontend/shell/frontend_application";

/// The default diff range: the slice's own range inside a worktree.
pub(crate) const DEFAULT_BASE: &str = "main...HEAD";

mod changed_files;
pub(crate) use changed_files::changed_rs;
pub(crate) use changed_files::fmt_changed;
pub(crate) use changed_files::frontend_include_input_touched;
pub(crate) use changed_files::frontend_tests_changed;
pub(crate) use changed_files::owning_package_dir;
pub(crate) use changed_files::realpath_m;
use changed_files::rs_files_under;
pub(crate) use changed_files::wasm_scope_prefixes;
pub(crate) use changed_files::wasm_scope_touched;

mod include_inputs;
pub(crate) use include_inputs::compiled_include_input_paths;
pub(crate) use include_inputs::include_consumer_package_dirs;
pub(crate) use include_inputs::include_inputs_under;
pub(crate) use include_inputs::repository_reads_under;
pub(crate) use include_inputs::workspace_members;
