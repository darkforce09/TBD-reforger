//! The names a caller imports with `use repository_relocation::prelude::*;`: the three modes of
//! `cargo xtask refactor relocate`.

pub use crate::relocation_modes::{apply, dry_run, verify};
