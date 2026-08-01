//! Business-logic services — Rust port of `internal/services`.

pub mod audit;
pub mod discord;
pub mod game_agent;
pub mod http_retry;
pub mod mission_compile;
pub mod mortar;
pub mod ratelimit_gc;
pub mod registry_import;
pub mod role_sync;
pub mod text;
pub mod token_purge;
pub mod user_stats;
pub mod webhook;

pub use audit::write_audit;
pub use discord::DiscordService;
// T-595 — the API half of T-289's host control channel.
pub use game_agent::{AgentAction, AgentReply, AgentResult};
pub use mission_compile::{
    CompileError, ModMissionDocument, ModSlot, flatten_to_mod_document,
    flatten_to_mod_document_with_catalog, mission_terrain_key,
};
// Ported to the shared crate (T-145 Phase 2); re-exported so `crate::services::…` callers are unchanged.
pub use map_engine_core::mission::orbat::{
    OrbatSlotTemplate, OrbatSquadTemplate, parse_orbat_template,
};
pub use mortar::{FireSolution, SolveError, solve_fire_mission};
// T-578 — garbage collection for the durable rate limiter's bucket table.
pub use ratelimit_gc::{RATE_LIMIT_BUCKET_TTL, RATE_LIMIT_PRUNE_INTERVAL, start_rate_limit_prune};
pub use registry_import::{ImportCounts, ImportError, ensure_modpack, import_compat, import_items};
pub use role_sync::resync_all_roles;
pub use text::{sanitize_html, snippet};
pub use token_purge::{PurgeHandle, purge_expired_refresh_tokens, start_refresh_token_purge};
// T-336 — `users.total_deployments` / `attendance_rate` have exactly one writer and two callers.
// That makes it a service; it used to be `pub(super)` inside `handlers/telemetry.rs`.
pub use user_stats::{
    recompute_user_stats, recompute_user_stats_best_effort, refresh_leaderboard_best_effort,
};
pub use webhook::WebhookService;
