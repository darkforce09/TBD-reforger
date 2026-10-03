//! Two-dimensional indexes over map points in world metres.
//!
//! **Role:** groups the uniform point grid ([`point_index`]), the selection picks built on it
//! ([`picking`]) and the zoom-level clusters ([`cluster`]).
//! **Position:** a module of `spatial_indexes`; read by the map engine's editing picks, its cluster
//! layer and its chunk scheduler's object index.
//! **Signals & state:** none; module declarations only.
//! **Invariants:** no module here reads a map concept beyond points in world metres.

pub mod cluster;
pub mod picking;
pub mod point_index;
