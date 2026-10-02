//! The route acceptance framework shared by the `route_acceptance_*` binaries.
//!
//! **Role:** proves every registered route against seven dimensions (authorized,
//! unauthorized, ownership, guest, ban, malformed, boundary) and its response contract, from
//! compact per-route specs; the framework derives every probe that needs no domain knowledge.
//!
//! **Position:** compiled into each binary that writes `mod route_acceptance_support;` (beside
//! `mod common;` and `mod contract_support;`, which it uses); adds no test binary. Parts:
//! [`specs`] holds every part's specs, `world/<part>.rs` its world (mounted by its binary).
//!
//! **Signals & state:** see each module; the parsed route table is the only process-wide cache.
//!
//! **Invariants:** the route table is read from source at runtime, never listed by hand; the
//! shared core ([`route_table`], [`spec`], [`actors`], [`requests`], [`world`], [`specs`]) is
//! what every part builds on, so a part changes only its own spec and world files.

// Each binary uses a different subset of this module, so an item one binary leaves unused is
// not dead code; the gate runs clippy over every target with `-D warnings`.
#![allow(dead_code)]

pub mod actors;
pub mod contracts;
pub mod derived_probes;
pub mod dimension_runner;
pub mod probe;
pub mod requests;
pub mod round_trip_comparison;
pub mod route_table;
pub mod route_tags;
pub mod rust_source_scanning;
pub mod spec;
pub mod specs;
pub mod world;
