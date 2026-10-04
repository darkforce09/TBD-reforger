//! The fixed preferences of a terrain-and-imagery view.

use super::{HILLSHADE_OPACITY, SATELLITE_BASEMAP, terrain_and_imagery_preferences};
use map_streaming_model::host_preferences::BootstrapScope;

#[test]
fn the_scope_keeps_heights_and_skips_every_world_layer() {
    let prefs = terrain_and_imagery_preferences();
    assert_eq!(prefs.scope, BootstrapScope::TerrainAndImagery);
    assert!(!prefs.scope.loads_world_content());
    assert!(prefs.scope.keeps_full_resolution_dem());
    assert!(BootstrapScope::Full.loads_world_content());
    assert!(!BootstrapScope::Full.keeps_full_resolution_dem());
}

#[test]
fn draws_satellite_imagery_with_hillshade_and_grid() {
    let prefs = terrain_and_imagery_preferences();
    assert_eq!((prefs.basemap)(), SATELLITE_BASEMAP);
    let render = (prefs.render)();
    assert_eq!(render.hillshade_opacity, HILLSHADE_OPACITY);
    assert!(render.show_hillshade && render.show_grid);
}
