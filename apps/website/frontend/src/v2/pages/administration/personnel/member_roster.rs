//! The roster table: who is on it, how it is ordered, and what each row shows.
//!
//! **Role:** the sort and filter modes the header cycles through, the two pure passes that apply
//! them, the table and its rows, and the initials avatar a roster row falls back to.
//! **Position:** the left pane of the personnel screen, filling most of its width.
//! **Signals & state:** writes `selected_id` when a row is clicked; everything else is plain data
//! handed in by the page.
//! **Invariants:** sorting and filtering happen **on the loaded page**, not on the server: the
//! roster endpoint offers a search term and nothing else, so cycling a mode reorders what is
//! already here rather than refetching. Every ordering breaks ties on a second key so the table
//! cannot reshuffle rows that compare equal. The roster carries no avatar image, so a row shows
//! initials built from the name it does carry.
#![allow(dead_code)]

use crate::v2::core::api::dto::AdminUserRow;
use crate::v2::core::ui::cn;
use leptos::prelude::*;

/// Badge shown against a member who is not banned.
pub(super) const BADGE_SUCCESS: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-success/30 bg-success/15 text-success";

/// Badge shown against a banned member.
pub(super) const BADGE_ERROR: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-error-alert/30 bg-error-alert/10 text-error-alert";

/// The orders the header's sort control cycles through.
///
/// Client-side, because the list endpoint sorts by name and offers nothing else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SortMode {
    NameAsc,
    WarningsDesc,
    RoleAsc,
    BannedFirst,
}

impl SortMode {
    const ALL: [SortMode; 4] = [
        SortMode::NameAsc,
        SortMode::WarningsDesc,
        SortMode::RoleAsc,
        SortMode::BannedFirst,
    ];

    /// The next mode in the cycle, wrapping at the end.
    pub(super) fn next(self) -> Self {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// The caption the control shows while this mode is active.
    pub(super) fn label(self) -> &'static str {
        match self {
            SortMode::NameAsc => "Sort: Name",
            SortMode::WarningsDesc => "Sort: Warnings",
            SortMode::RoleAsc => "Sort: Role",
            SortMode::BannedFirst => "Sort: Banned",
        }
    }
}

/// The subsets the header's filter control cycles through.
///
/// Client-side too, over the loaded page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FilterMode {
    All,
    Active,
    Banned,
}

impl FilterMode {
    const ALL: [FilterMode; 3] = [FilterMode::All, FilterMode::Active, FilterMode::Banned];

    /// The next mode in the cycle, wrapping at the end.
    pub(super) fn next(self) -> Self {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// The caption the control shows while this mode is active.
    pub(super) fn label(self) -> &'static str {
        match self {
            FilterMode::All => "Filter: All",
            FilterMode::Active => "Filter: Active",
            FilterMode::Banned => "Filter: Banned",
        }
    }
}

/// Keep only the members the chosen subset admits.
pub(super) fn apply_roster_filter(
    mut users: Vec<AdminUserRow>,
    mode: FilterMode,
) -> Vec<AdminUserRow> {
    match mode {
        FilterMode::All => users,
        FilterMode::Active => {
            users.retain(|u| !u.is_banned);
            users
        }
        FilterMode::Banned => {
            users.retain(|u| u.is_banned);
            users
        }
    }
}

/// Order the loaded members by the chosen mode.
///
/// Every ordering falls back to a second key, so rows that compare equal keep a stable order
/// instead of shuffling when the list is re-rendered.
pub(super) fn apply_roster_sort(mut users: Vec<AdminUserRow>, mode: SortMode) -> Vec<AdminUserRow> {
    match mode {
        SortMode::NameAsc => {
            users.sort_by(|a, b| {
                display_name(a)
                    .to_ascii_lowercase()
                    .cmp(&display_name(b).to_ascii_lowercase())
                    .then_with(|| a.discord_id.cmp(&b.discord_id))
            });
        }
        SortMode::WarningsDesc => {
            users.sort_by(|a, b| {
                b.warnings
                    .cmp(&a.warnings)
                    .then_with(|| display_name(a).cmp(&display_name(b)))
            });
        }
        SortMode::RoleAsc => {
            users.sort_by(|a, b| {
                a.role
                    .as_str()
                    .cmp(b.role.as_str())
                    .then_with(|| display_name(a).cmp(&display_name(b)))
            });
        }
        SortMode::BannedFirst => {
            users.sort_by(|a, b| {
                b.is_banned
                    .cmp(&a.is_banned)
                    .then_with(|| display_name(a).cmp(&display_name(b)))
            });
        }
    }
    users
}

/// Up to two initials from a name, or a pair of question marks when there are none.
pub(super) fn initials(name: &str) -> String {
    let s: String = name
        .split(|c| c == ' ' || c == '_' || c == '.' || c == '-')
        .filter(|w| !w.is_empty())
        .take(2)
        .filter_map(|w| w.chars().next())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if s.is_empty() {
        "??".to_string()
    } else {
        s
    }
}

/// A round initials badge standing in for the avatar the roster does not carry.
pub(super) fn avatar(name: &str, class: &str) -> impl IntoView {
    let c = cn(&[
        "flex shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-primary/40 to-tertiary/30 font-semibold text-on-surface",
        class,
    ]);
    let text = initials(name);
    view! { <span class=c>{text}</span> }
}

/// The name a member is shown under: their handle where there is one, else their username.
pub(super) fn display_name(u: &AdminUserRow) -> String {
    if u.discord_handle.is_empty() {
        u.username.clone()
    } else {
        u.discord_handle.clone()
    }
}

/// The roster table, or a line saying there is nobody to show.
pub(super) fn roster_table(
    users: Vec<AdminUserRow>,
    selected_id: RwSignal<Option<String>>,
) -> impl IntoView {
    if users.is_empty() {
        return view! { <p class="p-6 text-on-surface-variant">"No users found."</p> }.into_any();
    }
    view! {
        <table class="w-full text-label-md">
            <thead class="sticky top-0 z-10 bg-surface-container-high/80 text-label-sm text-on-surface-variant uppercase backdrop-blur-md">
                <tr>
                    <th class="px-4 py-3 text-left font-medium">"User"</th>
                    <th class="px-4 py-3 text-left font-medium">"Arma Character"</th>
                    <th class="px-4 py-3 text-left font-medium">"Rank"</th>
                    <th class="px-4 py-3 text-right font-medium">"Warnings"</th>
                    <th class="px-4 py-3 text-right font-medium">"Status"</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-white/5">
                {users
                    .into_iter()
                    .map(|u| roster_row(u, selected_id))
                    .collect_view()}
            </tbody>
        </table>
    }
    .into_any()
}

/// One roster row: avatar and name, identifiers, role, warning count and status.
pub(super) fn roster_row(u: AdminUserRow, selected_id: RwSignal<Option<String>>) -> impl IntoView {
    let name = display_name(&u);
    let arma = if !u.arma_character.is_empty() {
        u.arma_character.clone()
    } else {
        u.arma_id
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unlinked".to_string())
    };
    let warn_class = if u.warnings > 0 {
        "px-4 py-3 text-right font-mono text-tactical-yellow"
    } else {
        "px-4 py-3 text-right font-mono text-on-surface-variant"
    };
    let status = if u.is_banned {
        view! { <span class=BADGE_ERROR>"Banned"</span> }.into_any()
    } else {
        view! { <span class=BADGE_SUCCESS>"Active"</span> }.into_any()
    };
    let uid = u.discord_id.clone();
    let uid_active = StoredValue::new(u.discord_id.clone());
    let is_active = move || selected_id.get() == Some(uid_active.get_value());
    view! {
        <tr
            on:click=move |_| selected_id.set(Some(uid.clone()))
            class=move || {
                cn(
                    &[
                        "cursor-pointer transition-colors",
                        if is_active() { "bg-primary/15" } else { "hover:bg-white/[0.03]" },
                    ],
                )
            }
        >
            <td class=move || {
                cn(
                    &[
                        "border-l-4 px-4 py-3",
                        if is_active() { "border-primary" } else { "border-transparent" },
                    ],
                )
            }>
                <div class="flex items-center gap-3">
                    {avatar(&name, "size-8 text-xs")}
                    <span class="truncate text-on-surface">{name.clone()}</span>
                </div>
            </td>
            <td class="px-4 py-3 text-on-surface-variant">{arma}</td>
            <td class="px-4 py-3">
                <span class="text-label-sm text-on-surface-variant uppercase">
                    {u.role.as_str()}
                </span>
            </td>
            <td class=warn_class>{u.warnings}</td>
            <td class="px-4 py-3 text-right">{status}</td>
        </tr>
    }
}
