//! The field-tools hub: the standalone tactical aids.
//!
//! **Role:** groups the calculators and inspectors that stand on their own rather than hanging off
//! a mission or an operation.
//! **Position:** the `/tools/…` routes.
//! **Signals & state:** none at this level; each page owns its own.
//! **Invariants:** nothing here is required by another hub — a page in this folder can be removed
//! without touching the rest of the tree.

pub mod mortar;
