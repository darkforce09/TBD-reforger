//! The `/api/v1` route table for community content.
//!
//! Paths are written relative to the `/api/v1` nest applied by `core::http_router`. Auth tiers
//! are enforced per-handler by the extractor each takes (`AuthUser` for the reads, `AdminUser`
//! for the writes), so they travel with the handler rather than with the registration.

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

use crate::core::application_state::AppState;
use crate::core::middleware;

use super::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Content reads (member tier via each handler's AuthUser extractor).
        .route(
            "/announcements",
            get(handlers::announcements_public::list_announcements),
        )
        .route(
            "/announcements/{id}",
            get(handlers::announcements_public::get_announcement),
        )
        .route("/wiki", get(handlers::wiki_knowledgebase::list_wiki))
        .route(
            "/wiki/{slug}",
            get(handlers::wiki_knowledgebase::get_wiki_page)
                .put(handlers::wiki_knowledgebase::upsert_wiki_page),
        )
        .route(
            "/vehicle-database",
            get(handlers::vehicle_database::list_vehicles)
                .post(handlers::vehicle_database::create_vehicle),
        )
        // Admin writes (create / replace / delete / set-current). Auth tier is per-handler via
        // AdminUser — same pattern as /wiki/{slug} PUT and /vehicle-database POST.
        .route(
            "/modpacks",
            get(handlers::modpack_catalog::list_modpacks)
                .post(handlers::modpack_admin::create_modpack),
        )
        .route(
            "/modpacks/current",
            get(handlers::modpack_catalog::get_current_modpack),
        )
        .route(
            "/modpacks/{id}",
            axum::routing::put(handlers::modpack_admin::replace_modpack)
                .delete(handlers::modpack_admin::delete_modpack),
        )
        .route(
            "/modpacks/{id}/set-current",
            post(handlers::modpack_admin::set_current_modpack),
        )
        // CMS — announcements + uploads.
        .route(
            "/cms/announcements",
            get(handlers::announcements_admin::list_cms_announcements)
                .post(handlers::announcements_admin::create_announcement),
        )
        .route(
            "/cms/announcements/{id}",
            axum::routing::patch(handlers::announcements_admin::update_announcement)
                .delete(handlers::announcements_admin::delete_announcement),
        )
        .route(
            "/cms/announcements/{id}/push-discord",
            post(handlers::announcement_discord_push::push_announcement_discord),
        )
        .route(
            "/cms/uploads",
            post(handlers::media_upload::upload_image)
                .layer(DefaultBodyLimit::max(middleware::MAX_MULTIPART_BODY)),
        )
}
