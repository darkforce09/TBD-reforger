//! The content manager: the unit's posts, and the editor that writes them.
//!
//! **Role:** declares the route component, the post list, the editor form, the hero upload behind
//! it, and the shape a post is edited as.
//! **Position:** the `/admin/content` route, in the administration hub.
//! **Signals & state:** none at this level; the page owns the fetch and the working set.
//! **Invariants:** the list and the form read the same working set, so the two panes can never
//! disagree about a post's title or its state.

mod article_table;
mod doc;
mod editor_form;
mod hero_upload;
pub mod page;

#[cfg(target_arch = "wasm32")]
pub use page::ContentManagerPage;

#[cfg(test)]
use doc::{category_tag, date_ymd, doc_from_announcement, is_server_id, tag_category};
#[cfg(test)]
use editor_form::apply_md_tool;

#[cfg(test)]
#[path = "tests/content.rs"]
mod tests;
