//! The map-asset host's editor half: the preference feed and the engine and host registration.
//!
//! **Role:** connects the editor's live layer, basemap and render preferences and its owner
//! cleanup to the streaming host of `map_streaming_host`.
//! **Position:** the bridge's edge to the map engines; all asset fetching, residency, geometry and
//! upload work lives in the engines; what sits here is the preference feed and the registration.
//! **Signals & state:** live preference readers and the mounted engine and host pair.
//! **Invariants:** cleanup clears only the same pair of `Rc` handles it registered, so a stale
//! cleanup cannot clear a newer mount.

#![cfg(target_arch = "wasm32")]
use map_renderer::EngineHandle;
use map_streaming_host::{DemGridHandle, HostHandle, RENDER_CTX};

/// Register the live engine and host with identity-guarded cleanup on the current UI owner.
pub fn register_render_ctx(engine: EngineHandle, host: HostHandle) {
    mission_creator_state::seam_registration::install_seam(&RENDER_CTX, (engine, host));
}

/// Boot every layer the Mission Creator draws, with the editor's live preference readers.
pub async fn bootstrap(
    engine: EngineHandle,
    terrain: String,
    host: HostHandle,
    dem_out: DemGridHandle,
    full_dem_out: terrain_elevation::full_resolution::FullResolutionDemHandle,
    report: map_streaming_model::boot_progress::ProgressFn,
) {
    use map_streaming_model::host_preferences::{
        BootstrapScope, HostPreferences, RenderPreferences,
    };
    let preferences = HostPreferences {
        scope: BootstrapScope::Full,
        world_layers: mission_creator_state::world_layer_prefs::load_prefs,
        basemap: mission_creator_state::world_layer_prefs::load_basemap_view,
        render: || {
            let env = crate::bridge::host_state::editor_context::read_env();
            RenderPreferences {
                hillshade_opacity: env.hillshade_opacity,
                show_hillshade: env.show_hillshade,
                show_grid: env.show_grid,
            }
        },
    };
    map_streaming_host::bootstrap(
        engine,
        terrain,
        host,
        dem_out,
        full_dem_out,
        report,
        preferences,
    )
    .await;
}
