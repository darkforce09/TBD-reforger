//! The persistent frame every standard page renders inside.
//!
//! **Role:** owns the application frame — the sidebar, the top bar, the navigation registry they
//! read, the classifier that decides which frame a route gets, and the fallback page shown when
//! no route matches.
//! **Position:** mounted once at the application root, outside the router's outlet. Navigating
//! between standard pages swaps the outlet only; the frame stays mounted.
//! **Signals & state:** the frame provides the session store and the toast queue to everything
//! below it, and owns two local open/closed signals — the mobile drawer and the user menu.
//! **Invariants:** for a chromeless route the frame yields the whole viewport, and for the
//! sign-in pages it renders no wrapper at all.

pub mod layout;
pub mod nav_config;
pub mod not_found;
pub mod sidebar;
pub mod top_nav;

#[allow(unused_imports)]
pub use layout::AppLayout;
#[allow(unused_imports)]
pub use nav_config::{NavItem, NavSection, NAVIGATION};
#[allow(unused_imports)]
pub use not_found::NotFoundPage;
