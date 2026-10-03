//! The two reads of a manual's history: one page of its revisions, and one revision.
//!
//! **Role:** creates the resource for the page of the history on show and the resource for the
//! revision on view, each refetched when the article's state says so.
//! **Position:** created by the article pane when a manual opens; read by the revision list and
//! the revision view.
//! **Signals & state:** the list resource tracks the article's `revisions_page` and
//! `revisions_generation`; the revision resource tracks `viewing` and resolves to `None` while no
//! revision is on view.
//! **Invariants:** a failed request settles as a failure, so the panes render their failure
//! text rather than waiting.

use super::super::api_paths::{revision_list_path, revision_path};
use super::super::article::ArticleState;
use crate::foundation::auth::AuthStore;
use crate::foundation::transport::client::Fetched;
use crate::foundation::transport::dto::wiki::{WikiRevision, WikiRevisionPage};
use leptos::prelude::*;

/// One page of the history, as fetched.
pub(in super::super) type RevisionListResource = LocalResource<Fetched<WikiRevisionPage>>;

/// The revision on view, as fetched; `None` while none is on view.
pub(in super::super) type RevisionResource = LocalResource<Option<Fetched<WikiRevision>>>;

/// `GET path` as a settled fetch.
async fn fetch<T>(store: AuthStore, path: String) -> Fetched<T>
where
    T: serde::de::DeserializeOwned + Clone + 'static,
{
    crate::foundation::transport::client::api_get::<T>(store, &path)
        .await
        .into()
}

/// The resource for the page of the history on show.
pub(in super::super) fn revision_list_resource(
    store: AuthStore,
    article: ArticleState,
) -> RevisionListResource {
    LocalResource::new(move || {
        article.revisions_generation.track();
        let path = revision_list_path(&article.slug.get_value(), article.revisions_page.get());
        fetch::<WikiRevisionPage>(store, path)
    })
}

/// The resource for the revision on view.
pub(in super::super) fn revision_resource(
    store: AuthStore,
    article: ArticleState,
) -> RevisionResource {
    LocalResource::new(move || {
        let path = article
            .viewing
            .get()
            .map(|revision| revision_path(&article.slug.get_value(), revision));
        async move {
            match path {
                Some(path) => Some(fetch::<WikiRevision>(store, path).await),
                None => None,
            }
        }
    })
}
