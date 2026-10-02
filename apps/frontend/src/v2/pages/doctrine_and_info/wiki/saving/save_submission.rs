//! Sends a wiki save or restore and applies its answer to the page.
//!
//! **Role:** `PUT`s a [`WikiSaveRequest`] to `/wiki/{slug}` with the refusal-keeping verb and,
//! on success, drops the saved draft, returns the pane to reading, reloads the article, its
//! history and the page list, and raises a notice; on refusal, records the classified
//! [`SaveFailure`] for the problem view.
//! **Position:** called by the editor's save button and the revision view's restore action.
//! **Signals & state:** writes the page's `drafts` and `mode` and the article's `busy`,
//! `failure`, `viewing` and reload counters; reads the `AuthStore` for the request.
//! **Invariants:** one write at a time per article: a call while one is in flight does nothing.
//! The request runs on `wasm32` only; natively the call clears the busy flag and sends nothing.
//! The answer is applied with fallible writes, so a pane closed mid-flight is left alone.

use super::super::article::ArticleState;
use super::super::page_state::WikiPageState;
use super::save_refusal::SaveOrigin;
use crate::v2::core::api::dto::wiki::WikiSaveRequest;
use leptos::prelude::*;

/// Sends `request`, the `origin` write of the article `article` shows.
pub(in super::super) fn submit_save(
    page: WikiPageState,
    article: ArticleState,
    origin: SaveOrigin,
    request: WikiSaveRequest,
) {
    if article.busy.get_untracked() {
        return;
    }
    article.busy.set(true);
    article.failure.set(None);
    #[cfg(target_arch = "wasm32")]
    {
        use super::super::api_paths::article_path;
        use super::save_refusal::{SaveFailure, SaveProblem};
        use crate::v2::core::api::client::{api_put_keeping_refusal, ApiRefusal};
        use crate::v2::core::api::dto::wiki::WikiArticle;

        let toasts = crate::v2::core::ui::toast::use_toasts();
        let slug = article.slug.get_value();
        let base_revision = request.base_revision;
        let body = serde_json::to_value(&request);
        leptos::task::spawn_local(async move {
            let answer = match body {
                Ok(body) => {
                    api_put_keeping_refusal::<WikiArticle>(page.store, &article_path(&slug), body)
                        .await
                }
                Err(_) => Err(ApiRefusal::unreadable()),
            };
            match answer {
                Ok(saved) => {
                    match origin {
                        SaveOrigin::Draft => {
                            page.drafts.try_update(|drafts| drafts.remove(&slug));
                            toasts.success(format!("Saved as revision {}", saved.revision));
                        }
                        SaveOrigin::Restore => {
                            let restored = article.viewing.try_get_untracked().flatten();
                            article.viewing.try_set(None);
                            toasts.success(match restored {
                                Some(old) => format!(
                                    "Revision {old} restored as revision {}",
                                    saved.revision
                                ),
                                None => format!("Restored as revision {}", saved.revision),
                            });
                        }
                    }
                    page.mode.try_set(super::super::page_state::WikiMode::Read);
                    article.reload_after_write();
                    page.page_list.refetch();
                }
                Err(refusal) => {
                    article.failure.try_set(Some(SaveFailure {
                        origin,
                        problem: SaveProblem::from_refusal(&refusal, base_revision),
                    }));
                }
            }
            article.busy.try_set(false);
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (page, origin, request);
        article.busy.set(false);
    }
}
