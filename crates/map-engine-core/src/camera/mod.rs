//! Camera math for the map render engine (T-151): a deck.gl-parity `OrthoCamera` on top of
//! verbatim gl-matrix f64 mirrors. Pure — no wasm/web/wgpu types; shared by the render crate,
//! the wasm shim (parity exports), and any future headless consumer.

pub mod glmat4;
mod ortho;

pub use ortho::{FAR, MAX_ZOOM, MIN_ZOOM, NEAR, OrthoCamera};
