//! Business logic behind the command center surfaces: the fleet overview the dashboard reads.
//! The member statistics and the leaderboard refresh the boards read live in the
//! `api_member_activity` crate, which the API's match-results, identity-link and event
//! administration transactions call.

pub mod fleet_overview;
