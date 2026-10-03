//! Heavy tooling behind the `enf`, `gate`, `mcpd`, `world`, `map`, `capture`,
//! `acknowledgement-dropping-relay` and `staging-load` binaries.
//!
//! - **Role:** headless browser gates, blueprint and world-export compilation, map raster production and map-asset verification.
//! - **Position:** the library of the `developer_tools` crate; each binary in `src/bin/` calls one
//!   module's command-line entry (`enf` and `mcpd` call the `enfusion_script_index` and
//!   `enfusion_mcp_broker` crates instead, and `acknowledgement-dropping-relay` and `staging-load`
//!   the `acknowledgement_dropping_relay` and `staging_load_generator` crates), and `xtask`
//!   calls the blueprint, map-verification and layout entry points directly.
//! - **Signals & state:** none at the crate root; each module owns its own state.
//! - **Invariants:** the crate never depends on `xtask`; the contract, fixture, terrain and glyph
//!   paths it reads resolve through [`repository_layout`].

pub mod blueprint;
pub mod browser_testing;
pub mod map_pipeline_layout;
pub mod map_raster_pipeline;
pub mod map_verification;
pub mod world_export_pipeline;
