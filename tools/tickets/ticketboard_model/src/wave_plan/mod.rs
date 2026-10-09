//! The recorded wave lock, its lanes as stored, and the ownership collisions between tickets.
//!
//! **Role:** reads `.ai/tickets/wave.lock` ([`services::lock_file`]) and projects its waves into
//! lanes ([`models::wave_projection`]) exactly as recorded.
//! **Position:** over [`crate::ticket_registry`] and `ticket_wave_lock`; the Waves tab of
//! `tools/tickets/ticketboard_desktop` paints [`models::view::WavePlanView`] and emits
//! [`events::WavePlanEvent`]s.
//! **Signals & state:** none; plain data rebuilt on load.
//! **Invariants:** a missing or malformed lock is a local refusal, never an empty plan; lanes are
//! never recomputed.

pub mod events;
pub mod models;
pub mod services;
