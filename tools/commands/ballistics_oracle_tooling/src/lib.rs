//! `ballistics_oracle_tooling`: the game ballistics catalog and calibration fixtures, trimmed from the gameplay
//! export and the ballistics oracle's output.
//!
//! **Role:** Holds `cargo xtask ballistics trim-export`, which writes the vanilla mortar catalog to
//! `contracts/catalogs/ballistics/` and its calibration bundle, refused bundles and provenance
//! to `contracts/fixtures/ballistics/`.
//!
//! **Position:** tier 1 of `tools/commands`; the xtask binary parses [`BallisticsCmd`] and calls
//! [`run`]. Reads the gitignored gameplay export and oracle output under `assets/`; its documents
//! are checked by `cargo xtask schema validate` and consumed by `ballistics_calibration` and the
//! API's catalog upload.
//!
//! **Signals & state:** none; each run reads its inputs and writes its outputs whole.
//!
//! **Invariants:** The same inputs always produce the same bytes; every input byte is verified
//! against its recorded SHA-256 before use.
mod calibration_assembly;
mod catalog_extraction;
mod cli;
mod contract_documents;
mod dispatch;
mod engine_numbers;
mod error;
mod game_tables;
mod gameplay_export;
mod negative_variants;
mod oracle_output;
pub mod prelude;
mod provenance_readme;
mod record_per_line_json;
mod row_elevations;
mod trim_export;

pub use cli::BallisticsCmd;
pub use dispatch::run;
pub use error::{Error, Result};
