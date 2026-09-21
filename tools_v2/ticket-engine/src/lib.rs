//! Ticket storage, validation, operations, views, scheduling, and metrics.
#![deny(clippy::wildcard_enum_match_arm)]

pub mod cli;
pub mod corpus_pins;
mod encoding;
pub mod metrics;
pub mod ops;
#[cfg(test)]
#[path = "tests/proptest_roundtrip_tests.rs"]
mod proptest_roundtrip_tests;
pub mod registry;
pub mod repository;
pub mod store;
pub mod sync;
mod timestamp;
pub mod validation;
pub mod vocab;
pub mod wave_lock;
pub use encoding::{TicketFile, parse_ticket_toml, render_ticket_toml};
pub use ops::OpOutcome;
pub use store::Corpus;
pub use timestamp::{now_utc_rfc3339, validate_rfc3339_utc};
pub use vocab::ScopeVocab;
mod model;
pub use model::*;
