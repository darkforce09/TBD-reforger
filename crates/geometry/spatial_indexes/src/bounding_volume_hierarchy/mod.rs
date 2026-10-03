//! Box trees over triangles and boxes, and the segment tests their walks run.
//!
//! **Role:** groups the shared flat-tree build core ([`flat_tree_build`]), the triangle tree and
//! its queries ([`triangle_tree`]), its sidecar file format ([`sidecar`]) and that format's
//! refusals ([`sidecar_parse_error`]), the triangle surface kinds ([`surface_kind`]), the
//! segment-triangle test ([`segment_triangle`]) and the segment-box slab test
//! ([`segment_box_window`]).
//! **Position:** a module of `spatial_indexes`; the developer tools build and emit trees, the map
//! engine parses and queries them and builds its world box tree on the same core.
//! **Signals & state:** none; module declarations only.
//! **Invariants:** every flat tree of the workspace is built by [`flat_tree_build`].

pub mod flat_tree_build;
pub mod segment_box_window;
pub mod segment_triangle;
pub mod sidecar;
pub mod sidecar_parse_error;
pub mod surface_kind;
pub mod triangle_tree;
