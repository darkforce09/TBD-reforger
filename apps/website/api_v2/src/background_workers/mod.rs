//! Long-running tasks the API binary spawns at boot and never awaits: each one polls or
//! recomputes something on an interval so a quiet request path cannot leave shared state stale.

pub mod leaderboard_refresher;
pub mod server_status_publisher;
