//! Business-logic services — Rust port of `internal/services`.

pub mod audit;
pub mod discord;
pub mod http_retry;
pub mod mission_compile;
pub mod mission_payload;
pub mod mortar;
pub mod role_sync;
pub mod text;
pub mod token_purge;
pub mod webhook;

pub use audit::write_audit;
pub use discord::DiscordService;
pub use mission_compile::{CompileError, ModMissionDocument, flatten_to_mod_document};
pub use mission_payload::{OrbatSlotTemplate, OrbatSquadTemplate, parse_orbat_template};
pub use mortar::{FireSolution, solve_fire_mission};
pub use role_sync::resync_all_roles;
pub use text::{sanitize_html, snippet};
pub use token_purge::{PurgeHandle, purge_expired_refresh_tokens, start_refresh_token_purge};
pub use webhook::WebhookService;
