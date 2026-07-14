//! map-engine-core — pure Rust compute for the TBD Reforger map engine, geometry, mission
//! compiler, and (Phase 3) document model. Compiles to native (backend + `cargo test`) and to
//! `wasm32-unknown-unknown`.
//!
//! No `wasm-bindgen` / `web-sys` here — the JS boundary lives in the `map-engine-wasm` shim.
//! The correctness contract (see the plan §4) classifies every kernel:
//!   - **R** rational (`+ - * /`, compare, `floor/min/max`, correctly-rounded `sqrt`) → f64 with
//!     the JS operation order, `as f32` at the JS store boundary → **bit-identical** to the TS.
//!   - **T** transcendental (`atan/atan2/sin/cos`, overflow-safe `hypot`) → ≤ 1 ULP pre-quantization.
//!   - **S** structural (algorithm replaced) → query-result-set equality.

mod js;

pub mod camera;
pub mod dem;
#[cfg(feature = "doc")]
pub mod doc;
/// T-154 - arsenal doll scene/camera/pick policy (pure; GPU lives in map-engine-render).
pub mod doll;
pub mod geometry;
/// T-152.1 — map labels + importance-distance declutter.
pub mod label;
#[cfg(feature = "mission")]
pub mod mission;
/// T-151.6 W6 — slot/cluster GPU pack + cluster gates (always available).
pub mod slots_gpu;
pub mod spatial;
#[cfg(feature = "world")]
pub mod world;
