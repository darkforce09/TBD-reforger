//! map-engine-core — pure Rust compute for the TBD Reforger map engine, geometry, mission
//! compiler, and (Phase 3) document model. Compiles to native (backend + `cargo test`) and to
//! `wasm32-unknown-unknown`.
//!
//! No `wasm-bindgen` / `web-sys` here — this crate is pure compute and the Leptos SPA links it
//! directly. (T-590: that line used to say "the JS boundary lives in the `map-engine-wasm` shim".
//! T-418 retired `map-engine-wasm` — zero reverse deps after the T-159.29.3 React deletion, see
//! the workspace Cargo.toml header. The crate does not exist, and a T-582 brief was written asking
//! for a wasm binding in it before anyone noticed. The `js` module here is NOT a JS boundary: it
//! re-implements JS numeric primitives so ports stay bit-identical.)
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
/// T-180.7 — ORBAT Manager `format_slot_line` (always available; bare `format_slot_line` tests).
pub mod slot_line;
/// T-151.6 W6 — slot/cluster GPU pack + cluster gates (always available).
pub mod slots_gpu;
pub mod spatial;
/// T-180.4 — squad leader→member LineList geometry (always available; bare `squad_link_` tests).
pub mod squad_links;
#[cfg(feature = "world")]
pub mod world;
