//! The composition root of the application state: the one place the concrete services are built.
//!
//! **Role:** builds the session authority, the Discord OAuth2 and webhook clients and the
//! equipment datasets from the loaded configuration, and injects them into
//! [`api_state::AppState::new`].
//! **Position:** the top layer of the crate beside `router` and the binaries; the
//! `api` binary, the router tests and every integration suite build their state here, so they
//! all run the services production runs.
//! **Signals & state:** none of its own; the state it returns owns everything it builds.
//! **Invariants:** the session authority reads the same `Arc<Config>` the state holds; nothing
//! here performs I/O, so building a state never fails and never touches the pool.

use std::path::PathBuf;
use std::sync::Arc;

use sqlx::PgPool;

use api_caller_identity::session_authorization::DatabaseSessionAuthority;
use api_configuration::configuration::Config;
use api_discord::discord_client::DiscordService;
use api_discord::discord_webhook::WebhookService;
use api_equipment_datasets::EquipmentDatasets;
use api_state::AppState;

/// The application state of an open `pool` and a loaded `cfg`, with the production services:
/// the database session authority, the Discord client of the configured OAuth2 application and
/// guild, the announcement webhook, and the equipment datasets under the configured directories.
pub fn application_state(pool: PgPool, cfg: Config) -> AppState {
    let cfg = Arc::new(cfg);
    let session_authority = Arc::new(DatabaseSessionAuthority {
        pool: pool.clone(),
        config: cfg.clone(),
    });
    let discord = DiscordService::new(
        cfg.discord_client_id.clone(),
        cfg.discord_client_secret.clone(),
        cfg.discord_redirect_url.clone(),
        cfg.discord_guild_id.clone(),
    );
    let webhook = WebhookService::new(cfg.discord_webhook_url.clone());
    let equipment_data = EquipmentDatasets::new(
        &cfg.equipment_data_dir,
        cfg.equipment_export_source_dir.as_ref().map(PathBuf::from),
    );
    AppState::new(
        pool,
        cfg,
        session_authority,
        Arc::new(discord),
        Arc::new(webhook),
        Arc::new(equipment_data),
    )
}
