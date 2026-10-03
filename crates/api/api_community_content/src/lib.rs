//! Community content: announcements, the doctrine wiki, the vehicle database, modpack
//! manifests, the media uploads and the CMS that authors them.
//!
//! **Role:** the API's community content domain: its `/api/v1` route table ([`routes()`]), the
//! handlers behind it, the modpack lookups and the wiki markup reader, and the content models.
//! **Position:** above `api_state` (the application state every handler extracts),
//! `api_http_layer` (the caller extractors, the multipart body limit), `api_discord` (the
//! announcement webhook), `api_equipment_datasets` (the equipment data viewer reads),
//! `api_audit_log`, `api_foundation` and `api_identifiers`; names no other domain. The API's
//! router merges [`routes()`]; the server infrastructure, missions and command center domains
//! read its modpack models and lookups, and the dashboard its announcement model.
//! **Signals & state:** none in memory; announcements, wiki pages and revisions, vehicles,
//! modpacks and their mods live in the database, uploads in the configured upload directory.
//! **Invariants:** every content write takes the administrator extractor, and the announcement,
//! wiki, vehicle and modpack writes leave an audit line; the equipment data viewer reads are
//! registered only when `routes` is told the configuration is a development one.

mod error;
pub mod handlers;
pub mod models;
pub mod prelude;
pub mod routes;
pub mod services;

pub use error::{Error, Result};
pub use routes::routes;
