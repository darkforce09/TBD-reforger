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

/// Building architectural blueprints + line-of-sight attribution over the [`bvh`] raycaster
/// (`blueprint` implies `bvh`; standalone so the SPA links it without the `world` parser stack;
/// `world` re-exports it for backend callers).
#[cfg(feature = "blueprint")]
pub mod building_blueprint;
/// T-090.6 step 4 — per-level visibility rasters (multi-floor viewshed) over the [`bvh`]
/// sidecar; the building viewer's wash textures.
#[cfg(feature = "blueprint")]
pub mod building_viewshed;
/// T-090.6 — BVH any-hit / closest-hit raycaster over collision trimeshes + the `.bvh` sidecar
/// codec (the 3D occlusion lane that retired 2.5D LOS; zero-dep, wasm-safe).
#[cfg(feature = "bvh")]
pub mod bvh;
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

/// T-747 — bare `cargo test -p map-engine-core` is a vacuous pass: `doc` / `mission` / `world` /
/// `bvh` (and their ~500 tests) are feature-gated and never compile. This module is deliberately
/// *not* behind those features so a featureless suite still runs one test — and that test
/// fails until `--all-features` (or `--features doc,mission,world,bvh`) is supplied.
#[cfg(test)]
mod feature_gate_tripwire {
    /// Fail loudly when the feature floor is incomplete so agents cannot mistake a partial suite
    /// (~140 bare / ~502 with doc,mission) for the real ~650-test run.
    ///
    /// Wave gate (`cargo xtask platform wave` test map-engine) and Makefile use `--all-features`
    /// (sound). Ad-hoc `cargo xtask platform wave test --slice T-nnn -p map-engine-core`
    /// (T-742 private target dirs) does **not** auto-add features — pass `--all-features` (or
    /// `--features doc,mission,world,bvh`) yourself. `doc` alone does not compile the suite.
    #[test]
    #[allow(clippy::assertions_on_constants)] // intentional: cfg! is the tripwire signal
    fn map_engine_core_tests_require_doc_feature() {
        assert!(
            cfg!(feature = "doc")
                && cfg!(feature = "mission")
                && cfg!(feature = "world")
                && cfg!(feature = "bvh"),
            "map-engine-core tests must run with --all-features (or --features doc,mission,world,bvh). \
             Bare `cargo test -p map-engine-core` compiles out doc/mission/world/bvh and silently \
             skips hundreds of tests (~140 listed vs ~650 with --all-features). \
             `--features doc,mission` still skips the world and bvh suites. Wave gate + Makefile use \
             --all-features; T-742 `cargo xtask platform wave test --slice` still needs you to pass --all-features \
             for this crate (T-747)."
        );
    }
}
