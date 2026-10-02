//! The sign-in route.
//!
//! **Role:** groups the page component behind a folder so its panels have somewhere to live.
//! **Position:** the `/login` route, rendered bare by the frame.
//! **Signals & state:** none held here.
//! **Invariants:** the route must stay reachable while signed out.

mod page;

pub use page::LoginPage;
