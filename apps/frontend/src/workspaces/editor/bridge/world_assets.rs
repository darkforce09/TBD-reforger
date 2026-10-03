//! Role: connect editor preferences and owner cleanup to the graphics asset host.
//! Position: `editor/bridge` in the frontend editor adapter. All asset fetching, residency,
//! geometry and upload work lives in the engines; what sits here is the preference feed and the
//! registration.
//! Signals & state: live preference readers and the mounted engine/host pair.
//! Invariants: cleanup clears only the same pair of Rc handles it registered, so stale cleanup
//! cannot clear a newer mount.

#![cfg(target_arch = "wasm32")]
use map_engine::frame::EngineHandle;
/// Re-export `map_engine::streaming::host::*`.
pub use map_engine::streaming::host::*;
use std::rc::Rc;

impl crate::workspaces::editor::ui::inspector::validation_panel::SeamRegistration
    for (EngineHandle, HostHandle)
{
    fn is_same_registration(&self, live: &Self) -> bool {
        Rc::ptr_eq(&self.0, &live.0) && Rc::ptr_eq(&self.1, &live.1)
    }
}

/// Register the live engine and host with identity-guarded cleanup on the current UI owner.
pub fn register_render_ctx(engine: EngineHandle, host: HostHandle) {
    crate::workspaces::editor::input::tools::ruler_tool::install_seam(&RENDER_CTX, (engine, host));
}

/// Boot every layer the Mission Creator draws, with the editor's live preference readers.
pub async fn bootstrap(
    engine: EngineHandle,
    terrain: String,
    host: HostHandle,
    dem_out: DemGridHandle,
    full_dem_out: terrain_elevation::full_resolution::FullResolutionDemHandle,
    report: map_engine::streaming::bridge::progress::ProgressFn,
) {
    use map_engine::streaming::bridge::host_preferences::{
        BootstrapScope, HostPreferences, RenderPreferences,
    };
    let preferences = HostPreferences {
        scope: BootstrapScope::Full,
        world_layers: crate::workspaces::editor::session::world_layer_prefs::load_prefs,
        basemap: crate::workspaces::editor::session::world_layer_prefs::load_basemap_view,
        render: || {
            let env = crate::workspaces::editor::bridge::host_state::editor_context::read_env();
            RenderPreferences {
                hillshade_opacity: env.hillshade_opacity,
                show_hillshade: env.show_hillshade,
                show_grid: env.show_grid,
            }
        },
    };
    map_engine::streaming::host::bootstrap(
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
