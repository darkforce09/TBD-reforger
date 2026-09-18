//! Services behind the server fleet: the client for the game host's control agent, and the
//! publisher that puts server status rows on the realtime hub.

pub mod game_agent;
pub mod status_broadcast;
