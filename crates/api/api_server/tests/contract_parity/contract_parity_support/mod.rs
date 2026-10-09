//! Support for `tests/contract_parity/goldens.rs`: the frontend golden corpus, its seeded
//! reproduction, and the route contracts each golden answers to.
//!
//! **Role:** splits the suite by responsibility: [`golden_index`] reads the corpus,
//! [`seeded_capture`] replays the capture recipe, [`json_difference`] and [`event_stream_frames`]
//! compare answers, [`golden_normalisation`] applies the table in [`normalised_fields`] to the
//! server-generated fields of write answers, [`route_contracts`] with [`route_contract_table`]
//! resolve and validate contracts, [`generated_type_decoders`] decodes into the generated types,
//! and [`catalogue_row_constraints`] compares the registry row definitions with the catalogue
//! definitions they copy.
//!
//! **Position:** compiled into the `contract_parity` binary through
//! `mod contract_parity_support;`; its `goldens` and `equipment_viewer` modules use it. This
//! directory adds no binary of its own. It reads `contracts/fixtures/api_goldens/`, `crates/api/api_database/seeds/` and
//! `contracts/definitions/`, and writes only the binary's own database.
//!
//! **Signals & state:** the capture in [`seeded_capture`] is the only state: taken once per
//! process, shared read-only by the cases that need it.
//!
//! **Invariants:** no golden or schema is normalised, and an answer only at the fields the
//! normalisation table names, each after its format check; every failure names the golden file
//! and the field or frame it concerns.

pub(crate) mod catalogue_row_constraints;
pub(crate) mod event_stream_frames;
pub(crate) mod generated_type_decoders;
pub(crate) mod golden_index;
pub(crate) mod golden_normalisation;
pub(crate) mod json_difference;
pub(crate) mod normalised_fields;
pub(crate) mod route_contract_table;
pub(crate) mod route_contracts;
pub(crate) mod seeded_capture;
