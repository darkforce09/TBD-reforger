//! Vector geometry marched over the DEM grid — contour isolines, hypsometric sea band, and forest
//! mass hulls. All **Class R** (bit-identical to the TS `worldmap/{contours,seaBand,forestMass}.ts`),
//! emitting the same deck.gl wire buffers (interleaved `[x0,y0,x1,y1]` segments, `_normalize:false`
//! closed rings, per-vertex RGBA).

pub mod contours;
pub mod forest_mass;
pub mod sea_band;
pub mod tbdd;
