//! DEM (digital elevation model) math. `sample` = the `uint16 → meters` sampler; `downsample` =
//! the box-average vector grid the geometry marches over. Phase 1 adds `hillshade` + `png`.

pub mod downsample;
pub mod hillshade;
pub mod peaks;
#[cfg(feature = "png")]
pub mod png_decode;
pub mod sample;

pub use downsample::DemVectorGrid;
