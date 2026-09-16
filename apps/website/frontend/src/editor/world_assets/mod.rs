//! Role: connect editor preferences and owner cleanup to the graphics asset host.
//! Position: `editor/world_assets` in the frontend editor adapter.
//! Signals & state: live preference readers and the mounted engine/host pair.
//! Invariants: cleanup clears only the same pair of Rc handles it registered.

#![cfg(target_arch = "wasm32")]
use std::rc::Rc;
use website_map_engine::core::context::handles::EngineHandle;
/// Re-export `website_map_engine::streaming::host::*`.
pub use website_map_engine::streaming::host::*;

impl crate::editor::panels::validation_panel::SeamRegistration for (EngineHandle, HostHandle) {
    fn is_same_registration(&self, live: &Self) -> bool {
        Rc::ptr_eq(&self.0, &live.0) && Rc::ptr_eq(&self.1, &live.1)
    }
}

/// Register the live engine and host with identity-guarded cleanup on the current UI owner.
pub fn register_render_ctx(engine: EngineHandle, host: HostHandle) {
    crate::editor::tools::ruler_tool::install_seam(&RENDER_CTX, (engine, host));
}

/// Supply the editor's live preference readers to the graphics asset loader.
pub async fn bootstrap(
    engine: EngineHandle,
    terrain: String,
    host: HostHandle,
    dem_out: DemGridHandle,
    report: website_map_engine::streaming::bridge::progress::ProgressFn,
) {
    use website_map_engine::streaming::bridge::host_preferences::{
        HostPreferences, RenderPreferences,
    };
    let preferences = HostPreferences {
        world_layers: crate::editor::world_layer_prefs::load_prefs,
        basemap: crate::editor::world_layer_prefs::load_basemap_view,
        render: || {
            let env = crate::editor::state::operations::read_env();
            RenderPreferences {
                hillshade_opacity: env.hillshade_opacity,
                show_hillshade: env.show_hillshade,
                show_grid: env.show_grid,
            }
        },
    };
    website_map_engine::streaming::host::bootstrap(
        engine,
        terrain,
        host,
        dem_out,
        report,
        preferences,
    )
    .await;
}
