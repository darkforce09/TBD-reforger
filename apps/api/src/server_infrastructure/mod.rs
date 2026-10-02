//! Server infrastructure: the dedicated-server catalog, its live status feed, and the RCON
//! console.

pub mod handlers;
pub mod models;
pub mod routes;
pub mod services;

pub use routes::routes;
