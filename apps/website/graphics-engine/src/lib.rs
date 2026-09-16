//! Role: lib.
//! Position: `apps/website/graphics-engine/src` — the pure renderer.
//! Signals & state: GPU handles and geometry only.
//! Invariants: this crate never learns a map noun. It defines the frame vocabulary in
//! `frame/`; `website-map-engine` speaks it, never the reverse. It must never depend on
//! `website-map-engine` — enforced by `cargo xtask verify engine-layers` rule 1.

/// The frame vocabulary — see `frame/mod.rs` for the rule that governs it.
#[cfg(target_arch = "wasm32")]
pub mod frame;
