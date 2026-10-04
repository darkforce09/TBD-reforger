//! The helpers most frontend tests reach for, for `use frontend_test_support::prelude::*;`.
//!
//! **Role:** re-exports the scrubbed views of a source text, the captured-response macro and the
//! repository file reads.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module.

pub use crate::class_r_scrub::{live_code, live_source, only_body, only_item};
pub use crate::golden;
pub use crate::repository_root::{repository_path, repository_text};
pub use crate::source_shards::{production_shard, production_source};
