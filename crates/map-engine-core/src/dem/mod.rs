//! DEM (digital elevation model) math. `sample` = the `uint16 → meters` sampler; `downsample` =
//! the box-average vector grid the geometry marches over. Phase 1 adds `hillshade` + `png`.

pub mod downsample;
pub mod hillshade;
pub mod peaks;
#[cfg(feature = "png")]
pub mod png_decode;
/// T-935.4 — `dem/elevation.dem` (`TBDE`): the raw `u16` grid that replaces the 16-bit PNG on the
/// boot path. Behind `binary` because it reads that module's `TbdeHeader`; the PNG decoder above
/// stays until the manifest cuts over (T-935.13), and both must decode to the same grid.
#[cfg(feature = "binary")]
pub mod raw;
pub mod sample;

pub use downsample::DemVectorGrid;
