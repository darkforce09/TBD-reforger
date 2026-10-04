//! The offline items most pages name, for `use frontend_offline::prelude::*;`.
//!
//! **Role:** re-exports the pack status words and the readers of their page-wide signals.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module.

pub use crate::pack_status::{OfflineState, OfflineStatus, OptionalFiles, PackRefresh};
pub use crate::status_signals::{offline_optional_files, offline_pack_refresh, offline_status};
