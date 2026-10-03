//! Every platform page, grouped by the hub its route belongs to.
//!
//! **Role:** hosts the route components and their panels. Each page is a folder with a
//! `page.rs` that owns layout only, plus one file per panel it renders.
//! **Position:** the pages layer, above `foundation` and `features`, beside `workspaces`; mounted
//! by the route table in `app_routes` and wrapped by the persistent frame in `shell`.
//! **Signals & state:** page-local signals live in the page folders themselves.
//! **Invariants:** a page may import from `foundation`, `features` and the map engine; it never
//! imports a workspace or another page area, and no lower layer imports from here.

pub mod account;
pub mod administration;
pub mod command_center;
pub mod doctrine_and_info;
pub mod field_tools;
pub mod mission_hub;
pub mod operations;
