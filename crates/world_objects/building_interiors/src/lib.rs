//! A building's interior: the blueprint of its levels and what stands on them, the compound of
//! its shell and placed instances, and the section cuts its plan drawings are made of.
//!
//! **Role:** models the blueprint (levels, walls, doors, windows, stairs, furniture, footprint,
//! vertical profile and roof grid) read from JSON or the archive, and attributes a sight line's
//! events to its features ([`blueprint`]); assembles a shell and its placed instances, with doors
//! that open and close, into the two-level structure the interior line of sight walks
//! ([`compound`]); and cuts the occlusion mesh at a height into plan segments and rasterises its
//! surfaces into sparse height fields ([`section`]).
//! **Position:** world objects category, tier 2, depending on `spatial_indexes` (the occlusion
//! sidecar, its BVH and the flat box tree), `world_file_formats` (the archived blueprint and the
//! building element ids), `geometry_primitives` and `newtype_ids`. The map engine's interior line
//! of sight and visibility wash, the world line of sight, the Mission Creator's building benches
//! and the developer tools' blueprint pipeline read it.
//! **Signals & state:** none; plain data and pure functions, except a compound's door states,
//! which its owner sets.
//! **Invariants:** the mesh alone decides whether a sight line is blocked, the blueprint only
//! names what it met; a level band is half-open except the topmost, so a ray on a shared floor
//! belongs to one level; a section cut equals the brute-force cut of every triangle.

pub mod blueprint;
pub mod building_ids;
pub mod compound;
pub mod error;
pub mod prelude;
pub mod section;
#[cfg(any(test, feature = "test_fixtures"))]
pub mod test_fixtures;

pub use error::{Error, Result};
