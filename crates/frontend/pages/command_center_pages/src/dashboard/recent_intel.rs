//! The intelligence feed at the foot of the dashboard.
//!
//! **Role:** renders the most recent announcements as a scrolling list of linked rows.
//! **Position:** the last child of the dashboard column, below the card grid, and the only one
//! that grows to fill the remaining height.
//! **Signals & state:** none — the typed announcement list arrives owned and is read once.
//! **Invariants:** an empty list renders one line of empty-state text under the heading rather
//! than an empty box. A row without an identifier links to the board itself instead of to a
//! post that cannot be addressed.

#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::Announcement;
#[cfg(target_arch = "wasm32")]
use frontend_ui::MaterialIcon;
#[cfg(target_arch = "wasm32")]
use frontend_ui::datefmt::format_short_date;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The intelligence feed for `announcements`, newest first as the payload ordered them.
#[cfg(target_arch = "wasm32")]
pub(super) fn recent_intel(announcements: Vec<Announcement>) -> impl IntoView {
    view! {
        <div class="relative flex flex-col overflow-hidden rounded-xl p-6 glass flex-1 gap-4">
            <h3 class="flex items-center gap-2 border-b border-border-subtle pb-3 text-label-sm text-on-surface-variant uppercase">
                <MaterialIcon name="list_alt" class="text-[18px]" />
                "Recent Intelligence"
            </h3>
            <div class="custom-scrollbar flex flex-col gap-2 overflow-y-auto pr-2">
                {if announcements.is_empty() {
                    view! {
                        <p class="text-label-md text-on-surface-variant">"No announcements yet."</p>
                    }
                        .into_any()
                } else {
                    announcements.into_iter().map(intel_row).collect_view().into_any()
                }}
            </div>
        </div>
    }
}

/// One feed row: a date pill, the headline with its pin marker, and the preview line.
///
/// Reads `id`, `title`, `is_pinned`, `published_at`, `snippet` and `body` from the
/// announcement. The whole row is the link to the post.
#[cfg(target_arch = "wasm32")]
fn intel_row(a: Announcement) -> impl IntoView {
    let href = if a.id.as_str().is_empty() {
        "/announcements".to_string()
    } else {
        format!("/announcements/{}", a.id)
    };
    let title = if a.title.is_empty() {
        "Untitled Post".to_string()
    } else {
        a.title
    };
    let pinned = a.is_pinned;
    let date = format_short_date(a.published_at.as_deref().unwrap_or_default());
    // `snippet` is absent on the wire when the author wrote none; fall back to the body's opening
    // paragraph so a post published without one is not a bare headline.
    let snippet = if a.snippet.is_empty() {
        a.body.split("\n\n").next().unwrap_or_default().to_string()
    } else {
        a.snippet
    };
    view! {
        <a
            href=href
            class="group/row flex items-start gap-3 rounded-lg border border-transparent p-3 transition-all hover:border-outline-variant/30 hover:bg-surface-variant/40"
        >
            <span class="mt-0.5 shrink-0 rounded border border-border-subtle bg-surface-container-lowest px-2 py-0.5 font-mono text-[10px] tracking-widest text-on-surface-variant uppercase">
                {date}
            </span>
            <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2">
                    {pinned
                        .then(|| {
                            view! { <MaterialIcon name="push_pin" class="text-[14px] text-tactical-yellow" /> }
                        })}
                    <h4 class="truncate text-label-md font-semibold text-on-surface-variant group-hover/row:text-on-surface">
                        {title}
                    </h4>
                </div>
                {(!snippet.is_empty())
                    .then(|| {
                        view! {
                            <p class="mt-1 line-clamp-2 text-label-sm text-outline">{snippet}</p>
                        }
                    })}
            </div>
            <MaterialIcon
                name="chevron_right"
                class="mt-0.5 shrink-0 text-[18px] text-outline transition-transform group-hover/row:translate-x-0.5"
            />
        </a>
    }
}
