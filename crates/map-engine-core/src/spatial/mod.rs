//! Spatial indices over a slot SoA — the Phase 3 replacement for the JS `rbush` (slotSpatialIndex /
//! worldSpatialIndex). **Class S** (structural: the algorithm is replaced, so the contract is
//! *query-result-set equality* with rbush, not internal-layout identity). Points live as parallel
//! `Float32Array` columns (row index = the integer handle that replaces the `${chunk}:${row}`
//! string id); a uniform CSR grid answers rect + nearest queries. No external deps.

pub mod cluster;
pub mod point_index;
