//! The wave lock reader.
//!
//! **Role:** declares `lock_file`: the tolerant read of `.ai/tickets/wave.lock` and the colliding
//! path pairs.
//! **Position:** loaded by `crate::application_state::background_loading`; projected by
//! `crate::wave_plan::models`.
//! **Signals & state:** none here; see `lock_file`.
//! **Invariants:** nothing in the viewer writes the lock.

pub mod lock_file;
