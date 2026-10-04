//! The route table items most callers name, for `use frontend_route_table::prelude::*;`.
//!
//! **Role:** re-exports the route record and the readers the frame and the route guard call.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module.

pub use crate::routes::{
    RouteDef, auth_denial_redirect, breadcrumb, chromeless, full_bleed, role_may_enter,
};
