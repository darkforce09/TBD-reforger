//! Identity and access: Discord OAuth2 sign-in, token refresh/logout, the caller's own
//! profile (`/me`), and the game-account link handshake.

pub mod handlers;
pub mod models;
pub mod routes;
pub mod services;

pub use routes::routes;
