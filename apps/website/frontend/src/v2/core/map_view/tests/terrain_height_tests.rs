//! Height reads through a shared full-resolution elevation handle.

use super::TerrainHeights;
use website_map_engine::world::terrain::dem::full_resolution::{
    FullResolutionDem, RasterFootprint, SampleEncoding,
};

#[test]
fn answers_none_until_the_raster_is_published() {
    let heights = TerrainHeights::new();
    assert_eq!(heights.height_at(1.0, 1.0), None);
}

#[test]
fn reads_what_the_boot_publishes_into_the_shared_handle() {
    let heights = TerrainHeights::new();
    let boot_side = heights.handle();
    let dem = FullResolutionDem::new(
        vec![0, 10, 20, 30],
        2,
        2,
        SampleEncoding {
            offset_m: 100.0,
            scale_m: 1.0,
        },
        RasterFootprint::from_world_bounds([0.0, 0.0, 2.0, 2.0]),
    )
    .expect("valid raster");
    *boot_side.borrow_mut() = Some(std::rc::Rc::new(dem));
    assert_eq!(heights.height_at(0.0, 0.0), Some(100.0));
    assert_eq!(heights.height_at(2.0, 2.0), Some(130.0));
    let centre = heights.height_at(1.0, 1.0).expect("inside");
    assert!((centre - 115.0).abs() < 1e-12, "{centre}");
    assert_eq!(heights.clone().height_at(3.0, 1.0), None);
}
