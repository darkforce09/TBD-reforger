//! The roster pager: previous and next, the page position, the page size and the member count.
//!
//! **Role:** the page arithmetic of one served roster page ([`PagerPosition`]: how many pages the
//! total fills, which neighbours a page has, whether it lies past the end) and the pager bar under
//! the roster table that shows it.
//! **Position:** the foot of the personnel screen's left pane. `page.rs` hands it the position of
//! the page the API served, the address parsed from the URL and the callback that navigates to a
//! new address.
//! **Signals & state:** none of its own; reads the caller's position and address memos and
//! calls the caller's navigation callback.
//! **Invariants:** a roster always fills at least one page, so the position never reads
//! "Page 1 of 0", even when nobody matches. Previous is disabled on the first page and next on the
//! last, and both stay disabled until a page has been served. The numbers shown are the served
//! ones, and a move keeps the address's search and page size.

#[cfg(target_arch = "wasm32")]
use super::roster_query::PER_PAGE_OPTIONS;
#[cfg(any(target_arch = "wasm32", test))]
use super::roster_query::{RosterQuery, FIRST_PAGE};
#[cfg(any(target_arch = "wasm32", test))]
use crate::foundation::transport::dto::administration::PersonnelPage;
#[cfg(target_arch = "wasm32")]
use crate::foundation::ui::{MaterialIcon, Select};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// Where one served roster page sits in the whole roster.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PagerPosition {
    /// The 1-based page served.
    pub page: i64,
    /// The page size served.
    pub per_page: i64,
    /// Every member the search matches, on any page.
    pub total: i64,
}

#[cfg(any(target_arch = "wasm32", test))]
impl PagerPosition {
    /// The position of a page the API served.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn served(page: &PersonnelPage) -> Self {
        Self {
            page: page.page,
            per_page: page.per_page,
            total: page.total,
        }
    }

    /// How many pages the total fills; one when nobody matches.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn page_count(self) -> i64 {
        if self.per_page < 1 || self.total <= 0 {
            return FIRST_PAGE;
        }
        let full = self.total / self.per_page;
        if self.total % self.per_page == 0 {
            full
        } else {
            full + 1
        }
    }

    /// The page before this one, if this is not the first.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn previous_page(self) -> Option<i64> {
        (self.page > FIRST_PAGE).then(|| self.page - 1)
    }

    /// The page after this one, if this is not the last.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn next_page(self) -> Option<i64> {
        (self.page < self.page_count()).then(|| self.page + 1)
    }

    /// True when the page lies beyond the last page the total fills, so it holds nobody.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn is_past_the_end(self) -> bool {
        self.page > self.page_count()
    }

    /// The position caption: `Page 2 of 5`.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn caption(self) -> String {
        format!("Page {} of {}", self.page, self.page_count())
    }

    /// The address to show instead of `current` when this served page lies past the end, which
    /// is the first page of the same search; `None` when the page is in range or was served for
    /// another address.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn fallback_for(self, current: &RosterQuery) -> Option<RosterQuery> {
        let served_for_current = self.page == current.page && self.per_page == current.per_page;
        (served_for_current && self.is_past_the_end()).then(|| current.with_page(FIRST_PAGE))
    }
}

/// The member count caption: `1 member`, `6 members`.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn member_count_caption(total: i64) -> String {
    if total == 1 {
        "1 member".to_string()
    } else {
        format!("{total} members")
    }
}

/// The pager bar: the member count, previous, the page position, next and the page size.
///
/// `position` is `None` until a page has been served, which leaves both moves disabled and the
/// captions empty; the page size control reads the address, so it shows before the first answer.
#[cfg(target_arch = "wasm32")]
pub(super) fn roster_pager(
    position: Memo<Option<PagerPosition>>,
    query: Memo<RosterQuery>,
    show_query: Callback<RosterQuery>,
) -> impl IntoView {
    let go_to = move |pick: fn(PagerPosition) -> Option<i64>| {
        if let Some(page) = position.get_untracked().and_then(pick) {
            show_query.run(query.get_untracked().with_page(page));
        }
    };
    let on_per_page =
        Callback::new(move |raw: String| show_query.run(query.get_untracked().with_per_page(&raw)));
    view! {
        <nav
            aria-label="Roster pages"
            data-testid="personnel-pager"
            class="flex flex-wrap items-center justify-between gap-3 border-t border-white/5 px-6 py-3 text-label-sm text-on-surface-variant"
        >
            <span data-testid="personnel-member-count">
                {move || position.get().map(|p| member_count_caption(p.total)).unwrap_or_default()}
            </span>
            <div class="flex items-center gap-2">
                <button
                    type="button"
                    data-testid="personnel-page-previous"
                    on:click=move |_| go_to(PagerPosition::previous_page)
                    prop:disabled=move || {
                        position.get().and_then(PagerPosition::previous_page).is_none()
                    }
                    class="flex items-center gap-1 rounded-full border border-white/10 px-3 py-1.5 text-on-surface transition hover:bg-white/5 disabled:opacity-40"
                >
                    <MaterialIcon name="chevron_left" class="text-[18px]" />
                    "Previous"
                </button>
                <span data-testid="personnel-page-position" class="min-w-24 text-center">
                    {move || position.get().map(PagerPosition::caption).unwrap_or_default()}
                </span>
                <button
                    type="button"
                    data-testid="personnel-page-next"
                    on:click=move |_| go_to(PagerPosition::next_page)
                    prop:disabled=move || position.get().and_then(PagerPosition::next_page).is_none()
                    class="flex items-center gap-1 rounded-full border border-white/10 px-3 py-1.5 text-on-surface transition hover:bg-white/5 disabled:opacity-40"
                >
                    "Next"
                    <MaterialIcon name="chevron_right" class="text-[18px]" />
                </button>
            </div>
            <div class="flex items-center gap-2">
                <span>"Per page"</span>
                <Select
                    label="Members per page"
                    options=PER_PAGE_OPTIONS
                    value={Signal::derive(move || query.get().per_page.to_string())}
                    on_change=on_per_page
                />
            </div>
        </nav>
    }
}

#[cfg(test)]
#[path = "tests/roster_pager.rs"]
mod tests;
