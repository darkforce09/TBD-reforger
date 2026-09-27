//! The doctrine wiki: the navigation list, the article, the administrator's save and the
//! revision history.
//!
//! **Role:** the wiki handlers, one submodule per concern, and the SQL they share
//! (`page_store`).
//! **Position:** the handlers are registered route by route in
//! [`crate::community_content::routes::routes`], which `core::http_router` nests under
//! `/api/v1`; they read and write `wiki_pages` and `wiki_page_revisions` and answer the shapes of
//! [`crate::community_content::models::wiki`], with blocks built by
//! [`crate::community_content::services::wiki_markup`].
//! **Signals & state:** none; each handler takes the pool from the application state.
//! **Invariants:** reads take `AuthUser` and the save `AdminUser`, through each handler's own
//! extractor; every handler carries the `@route` tag of its registration.

mod page_store;
mod reads;
mod revisions;
mod save;

pub use reads::{get_wiki_page, list_wiki};
pub use revisions::{get_wiki_revision, list_wiki_revisions};
pub use save::save_wiki_page;
