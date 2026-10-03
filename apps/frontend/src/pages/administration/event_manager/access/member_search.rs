//! The member directory typeahead the access panel picks accounts from.
//!
//! **Role:** renders a search field and the matching members from `GET /members?q=`, and hands the
//! picked member to the caller — a managed roster adding an entry, or a named-account condition
//! taking its account id.
//! **Position:** inside a managed group's roster editor, and under a named-account condition.
//! **Signals & state:** owns the query signal and the search resource keyed on it.
//! **Invariants:** nothing is fetched while the query is blank, so opening a roster does not list
//! the whole directory. The query is percent-encoded into the path, and the directory excludes
//! banned accounts on the backend. The search is browser-only, as is the field itself.

#[cfg(target_arch = "wasm32")]
use crate::foundation::transport::dto::Member;
#[cfg(any(target_arch = "wasm32", test))]
use crate::foundation::transport::endpoints::encode_path_segment;
#[cfg(target_arch = "wasm32")]
use crate::foundation::ui::SearchBox;
#[cfg(target_arch = "wasm32")]
use crate::foundation::ui::DEFAULT_AVATAR;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The directory path for a search, or `None` for a blank query, which searches nothing.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn member_search_path(query: &str) -> Option<String> {
    let query = query.trim();
    (!query.is_empty()).then(|| format!("/members?q={}", encode_path_segment(query)))
}

/// A search field and its results; `on_pick` receives the member clicked.
#[cfg(target_arch = "wasm32")]
#[component]
pub(super) fn MemberSearch(
    /// Receives the picked member.
    on_pick: Callback<Member>,
    /// Accessible name of the field.
    label: &'static str,
) -> impl IntoView {
    let store = expect_context::<crate::foundation::auth::AuthStore>();
    let query = RwSignal::new(String::new());
    let results = LocalResource::new(move || {
        let path = member_search_path(&query.get());
        async move {
            {
                match path {
                    Some(path) => crate::foundation::transport::client::api_get::<
                        crate::foundation::transport::dto::DataEnvelope<Member>,
                    >(store, &path)
                    .await
                    .ok()
                    .map(|found| found.data),
                    None => None,
                }
            }
        }
    });
    view! {
        <div class="rounded-lg border border-white/10 bg-surface-container p-2">
            <SearchBox
                label=label
                placeholder="Search members by name"
                value=query
                on_input=Callback::new(move |text: String| query.set(text))
            />
            <ul class="mt-1 max-h-40 overflow-y-auto">
                {move || {
                    results
                        .get()
                        .flatten()
                        .map(|found| {
                            if found.is_empty() {
                                view! {
                                    <li class="px-2 py-1 text-xs text-on-surface-variant">
                                        "No matching members."
                                    </li>
                                }
                                    .into_any()
                            } else {
                                found
                                    .into_iter()
                                    .map(|member| {
                                        let avatar = member
                                            .avatar_url
                                            .as_deref()
                                            .map(crate::foundation::utils::safe_avatar_url)
                                            .unwrap_or_else(|| DEFAULT_AVATAR.to_string());
                                        let name = member.username.clone();
                                        let id = member.discord_id.clone();
                                        view! {
                                            <li>
                                                <button
                                                    type="button"
                                                    on:click=move |_| on_pick.run(member.clone())
                                                    class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-sm hover:bg-white/5"
                                                >
                                                    <img src=avatar alt="" class="h-5 w-5 rounded-full" />
                                                    <span class="text-on-surface">{name}</span>
                                                    <span class="font-mono text-xs text-on-surface-variant">{id}</span>
                                                </button>
                                            </li>
                                        }
                                    })
                                    .collect_view()
                                    .into_any()
                            }
                        })
                }}
            </ul>
        </div>
    }
}
