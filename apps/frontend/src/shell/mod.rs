//! The persistent frame every standard page renders inside.
//!
//! **Role:** owns the application frame — the sidebar, the top bar, the membership status strip,
//! the classifier that decides which frame a route gets, and the fallback page shown when no route
//! matches. The navigation menu the sidebar reads lives in `foundation::route_table`.
//! **Position:** the top layer of the frontend (foundation < features < pages, workspaces <
//! shell), mounted once at the application root by `main.rs`, outside the router's outlet.
//! Navigating between standard pages swaps the outlet only; the frame stays mounted.
//! **Signals & state:** the frame provides the session store and the toast queue to everything
//! below it, and owns two local open/closed signals — the mobile drawer and the user menu.
//! **Invariants:** for a chromeless route the frame yields the whole viewport, and for the
//! sign-in pages it renders no wrapper at all. The frame components exist on wasm32 only, since
//! `start_app` is their one user; the native build keeps the pure frame classifier, the
//! active-link rule and the account badge for their tests.

pub mod layout;
// Wasm-only: reached from `AppLayout`, which only `start_app` mounts.
#[cfg(target_arch = "wasm32")]
pub mod membership_status;
// Wasm-only: the fallback of `AppRoutes`, which only `start_app` mounts through `AppLayout`.
#[cfg(target_arch = "wasm32")]
pub mod not_found;
// Wasm-only: reached from `AppLayout`, which only `start_app` mounts.
#[cfg(target_arch = "wasm32")]
pub mod sidebar;
pub mod top_nav;
