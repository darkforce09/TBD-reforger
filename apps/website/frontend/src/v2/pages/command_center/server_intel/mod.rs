//! The live game-server panel and the three sections it is built from.
//!
//! **Role:** declares the route component, the server picker and frosted panel shell, the
//! connect header and the telemetry grid.
//! **Position:** the `/server-intel` route.
//! **Signals & state:** none at this level.
//! **Invariants:** the panel reads one server — the default pick — and the stream that feeds it
//! is opened and torn down by `page`.

mod direct_connect;
mod page;
mod player_census;
mod server_list;

pub use page::ServerIntelPage;
