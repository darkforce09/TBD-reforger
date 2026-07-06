//! Business-logic services — Rust port of `internal/services`.

pub mod audit;
pub mod discord;
pub mod http_retry;
pub mod mission_compile;
pub mod mortar;
pub mod role_sync;
pub mod text;
pub mod token_purge;
pub mod webhook;

pub use audit::write_audit;
pub use discord::DiscordService;
pub use mission_compile::{CompileError, ModMissionDocument, flatten_to_mod_document};
// Ported to the shared crate (T-145 Phase 2); re-exported so `crate::services::…` callers are unchanged.
pub use map_engine_core::mission::orbat::{
    OrbatSlotTemplate, OrbatSquadTemplate, parse_orbat_template,
};
pub use mortar::{FireSolution, solve_fire_mission};
pub use role_sync::resync_all_roles;
pub use text::{sanitize_html, snippet};
pub use token_purge::{PurgeHandle, purge_expired_refresh_tokens, start_refresh_token_purge};
pub use webhook::WebhookService;
