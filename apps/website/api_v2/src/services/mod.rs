//! Business-logic services shared across domains.

pub mod user_stats;

pub use website_map_engine::data::scenario::orbat::{
    OrbatSlotTemplate, OrbatSquadTemplate, parse_orbat_template,
};
// `users.total_deployments` / `attendance_rate` have exactly one writer and two callers, which is
// what makes it a service rather than a handler helper.
pub use user_stats::{
    recompute_user_stats, recompute_user_stats_best_effort, refresh_leaderboard_best_effort,
};
