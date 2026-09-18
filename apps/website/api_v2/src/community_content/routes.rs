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
use crate::handlers;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Content reads (member tier via each handler's AuthUser extractor).
        .route(
            "/announcements",
            get(handlers::announcements::list_announcements),
        )
        .route(
            "/announcements/{id}",
            get(handlers::announcements::get_announcement),
        )
        .route("/wiki", get(handlers::wiki::list_wiki))
        .route(
            "/wiki/{slug}",
            get(handlers::wiki::get_wiki_page).put(handlers::wiki::upsert_wiki_page),
        )
        .route(
            "/vehicle-database",
            get(handlers::wiki::list_vehicles).post(handlers::wiki::create_vehicle),
        )
        // Admin writes (create / replace / delete / set-current). Auth tier is per-handler via
        // AdminUser — same pattern as /wiki/{slug} PUT and /vehicle-database POST.
        .route(
            "/modpacks",
            get(handlers::modpacks::list_modpacks).post(handlers::modpacks::create_modpack),
        )
        .route(
            "/modpacks/current",
            get(handlers::modpacks::get_current_modpack),
        )
        .route(
            "/modpacks/{id}",
            axum::routing::put(handlers::modpacks::replace_modpack)
                .delete(handlers::modpacks::delete_modpack),
        )
        .route(
            "/modpacks/{id}/set-current",
            post(handlers::modpacks::set_current_modpack),
        )
        // CMS — announcements + uploads.
        .route(
            "/cms/announcements",
            get(handlers::cms::list_cms_announcements).post(handlers::cms::create_announcement),
        )
        .route(
            "/cms/announcements/{id}",
            axum::routing::patch(handlers::cms::update_announcement)
                .delete(handlers::cms::delete_announcement),
        )
        .route(
            "/cms/announcements/{id}/push-discord",
            post(handlers::cms::push_announcement_discord),
        )
        .route(
            "/cms/uploads",
            post(handlers::cms::upload_image)
                .layer(DefaultBodyLimit::max(middleware::MAX_MULTIPART_BODY)),
        )
}
