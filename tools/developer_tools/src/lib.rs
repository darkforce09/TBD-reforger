//! Heavy tooling shared by the `enf`, `gate`, `mcpd`, `world`, `map`, `capture` and
//! `acknowledgement-dropping-relay` binaries.
//!
//! - **Role:** Enfusion archive and source indexing, headless browser gates, blueprint and
//!   world-export compilation, map raster production, map-asset verification, and the staging
//!   verification engines: the member load generator and the acknowledgement-dropping relay.
//! - **Position:** the library of the `developer_tools` crate; each binary in `src/bin/` calls one
//!   module's command-line entry, and `xtask` calls the blueprint, map-verification and layout
//!   entry points directly.
//! - **Signals & state:** none at the crate root; each module owns its own state.
//! - **Invariants:** the crate never depends on `xtask`; the contract, fixture, terrain and glyph
//!   paths it reads resolve through [`repository_layout`].

pub mod blueprint;
pub mod browser_testing;
pub mod content_digest;
pub mod enfusion_pak;
pub mod enfusion_tooling;
pub mod map_raster_pipeline;
pub mod map_verification;
pub mod repository_layout;
pub mod repository_paths;
pub mod staging_verification;
pub mod timestamp_formatting;
pub mod world_export_pipeline;
