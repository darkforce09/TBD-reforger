//! The landing dashboard and the five panels it arranges.
//!
//! **Role:** declares the route component and the panel shards it composes — the hero banner,
//! the server uplink card, the personal deployment card, the modpack card and the intelligence
//! feed — plus the accessors they share for reading untyped payload fields.
//! **Position:** the `/` route.
//! **Signals & state:** none at this level.
//! **Invariants:** a panel receives owned data and renders it; the fetch belongs to `page`.

mod deployment;
mod helpers;
mod hero_banner;
mod modpack;
mod page;
mod recent_intel;
mod server_uplink;

pub use page::DashboardPage;
