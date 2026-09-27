//! The open manual: its pane, its header, its body and the state they share.
//!
//! **Role:** declares the article pane, the header with the administrator's controls, the body
//! (rendered blocks, editor or older revision) and the per-manual state, and re-exports the pane
//! and the state for the route and the history and saving modules.
//! **Position:** the detail half of the wiki's split view, fed the slug the route resolved.
//! **Signals & state:** none at this level; [`ArticleState`] holds the manual's signals.
//! **Invariants:** a manual is fetched on its own when it opens, never read out of the page list,
//! and its state starts fresh each time.

mod article_body;
mod article_header;
mod article_pane;
mod article_state;

pub(super) use article_pane::article_pane;
pub(super) use article_state::ArticleState;
