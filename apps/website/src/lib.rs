//! TBD Reforger backend — Rust port of the Go API (T-145).
//!
//! Modules are populated phase-by-phase per the approved plan
//! (`~/.claude/plans/okay-so-here-s-the-mighty-babbage.md`). Each module mirrors
//! the corresponding Go package under `internal/`, preserving the wire contract
//! verified in the ground-truth census.

pub mod app;
pub mod auth;
pub mod config;
pub mod contract;
pub mod db;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod realtime;
pub mod services;
pub mod state;
