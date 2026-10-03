//! The ticket models every feature reads.
//!
//! **Role:** declares `classification`, `corpus`, `palette`, `projection` and `scope`.
//! **Position:** read by every feature of the crate and by the desktop application's views.
//! **Signals & state:** none here; see each module.
//! **Invariants:** the raw lowercase status and class names are the labels everywhere; colour is an
//! accent only.

pub mod classification;
pub mod corpus;
pub mod palette;
pub mod projection;
pub mod scope;
