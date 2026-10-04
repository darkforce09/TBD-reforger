//! The single-page app's route table and the sidebar's navigation menu.
//!
//! **Role:** the contract of every route the app answers ([`routes`]: path, component name, layout
//! flags, access tier, and the readers over them) and the sidebar's sections and links
//! ([`navigation_menu`]).
//! **Position:** a foundation crate above `frontend_api_dtos` (the role ladder) and below the
//! session; read by the session's route guard, the app shell's frame, top bar and sidebar, and the
//! route drift gate. It names no page: the render form of the table is the app's `app_routes.rs`.
//! **Signals & state:** none; static tables and pure functions.
//! **Invariants:** compiles on every target, so the native tests read the same tables the browser
//! build does; the `ROUTES` table stays in `routes.rs`, where the route drift gate
//! reads it as source text.

pub mod navigation_menu;
pub mod prelude;
pub mod routes;

pub use routes::{
    ROUTES, RouteDef, auth_denial_redirect, breadcrumb, chromeless, full_bleed, role_may_enter,
};
