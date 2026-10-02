//! `ballistics`: the game ballistics catalog and calibration fixtures, trimmed from the gameplay
//! export and the ballistics oracle's output.
//!
//! **Role:** Holds `cargo xtask ballistics trim-export`, which writes the vanilla mortar catalog to
//! `contracts/catalogs/ballistics/` and its calibration bundle, refused bundles and provenance
//! to `contracts/fixtures/ballistics/`.
//!
//! **Position:** Reads the gitignored gameplay export and oracle output under `assets/`; its
//! documents are checked by `cargo xtask schema validate` and consumed by the map engine's
//! calibration and the API's catalog upload.
//!
//! **Signals & state:** none; each run reads its inputs and writes its outputs whole.
//!
//! **Invariants:** The same inputs always produce the same bytes; every input byte is verified
//! against its recorded SHA-256 before use.
pub(crate) mod calibration_assembly;
pub(crate) mod catalog_extraction;
pub(crate) mod cli;
pub(crate) mod contract_documents;
pub(crate) mod dispatch;
pub(crate) mod engine_numbers;
pub(crate) mod game_tables;
pub(crate) mod gameplay_export;
pub(crate) mod negative_variants;
pub(crate) mod oracle_output;
pub(crate) mod provenance_readme;
pub(crate) mod record_per_line_json;
pub(crate) mod row_elevations;
pub(crate) mod trim_export;
