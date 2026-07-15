//! Personnel Roster (/admin/personnel) — ported from pages/admin.tsx `PersonnelRosterPage`.
//! `<AdminGate>` → `/admin/users` Resource → a two-pane layout: a data table (70%) of users +
//! a fixed dossier pane (30%).
//!
//! **Gate scope (this slice):** the seeded `/admin/users` golden (1 real row: Dev Operator) with
//! nothing selected → the populated 1-row table + the "Select personnel to view dossier" placeholder
//! are byte-exact-verified (REAL data). Row selection + `PersonnelDossier` (role edit / ban) are
//! behavior (mutations + T-interaction) — a follow-up.
#![allow(dead_code)]
use crate::dto::{AdminUserRow, Paginated};
use crate::ui::{cn, AdminGate, MaterialIcon};
use leptos::prelude::*;

/// Badge variant="success" class (ui/badge.tsx cn(), text-label-sm twMerge-dropped).
const BADGE_SUCCESS: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-success/30 bg-success/15 text-success";
const BADGE_ERROR: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-error-alert/30 bg-error-alert/10 text-error-alert";

/// Initials fallback avatar (RosterRow carries no avatar URL) — mirrors admin.tsx `initials`.
fn initials(name: &str) -> String {
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

fn avatar(name: &str, class: &str) -> impl IntoView {
    let c = cn(&[
        "flex shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-primary/40 to-tertiary/30 font-semibold text-on-surface",
        class,
    ]);
    let text = initials(name);
    view! { <span class=c>{text}</span> }
}

#[component]
pub fn PersonnelRosterPage() -> impl IntoView {
    view! {
        <AdminGate>
            <PersonnelInner />
        </AdminGate>
    }
}

#[component]
fn PersonnelInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let roster = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<Paginated<AdminUserRow>>(store, "/admin/users")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Paginated<AdminUserRow>>
        }
    });
    view! {
        <div class="flex h-full w-full flex-1 overflow-hidden bg-surface-glass backdrop-blur-xl">
            // ── Left: data table (70%) ──
            <div class="flex min-w-0 flex-[7] flex-col border-r border-white/10">
                <div class="border-b border-white/5 p-6">
                    <div class="flex flex-wrap items-center justify-between gap-4">
                        <h1 class="text-headline-lg text-on-surface">"Personnel Roster"</h1>
                        <div class="flex items-center gap-2">
                            <button
                                type="button"
                                class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-label-sm text-on-surface transition hover:bg-white/5"
                            >
                                <MaterialIcon name="swap_vert" class="text-[18px]" />
                                "Sort"
                            </button>
                            <button
                                type="button"
                                class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-label-sm text-on-surface transition hover:bg-white/5"
                            >
                                <MaterialIcon name="filter_list" class="text-[18px]" />
                                "Filter"
                            </button>
                        </div>
                    </div>
                    <div class="relative mt-4">
                        <MaterialIcon
                            name="search"
                            class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-[18px] text-on-surface-variant"
                        />
                        <input
                            type="search"
                            placeholder="Search Discord ID or Arma Name…"
                            value=""
                            class="w-full max-w-md rounded-full border border-white/10 bg-black/20 py-2.5 pr-3 pl-9 text-label-md text-on-surface placeholder:text-on-surface-variant/60 outline-none focus:border-primary/50"
                        />
                    </div>
                </div>
                <div class="custom-scrollbar min-h-0 flex-1 overflow-y-auto">
                    <Suspense fallback=move || {
                        view! { <p class="text-on-surface-variant">"Loading…"</p> }
                    }>
                        {move || {
                            roster
                                .get()
                                .map(|opt| match opt {
                                    Some(page) => roster_table(page.data).into_any(),
                                    None => {
                                        view! { <p class="text-error">"Failed to load data."</p> }
                                            .into_any()
                                    }
                                })
                        }}
                    </Suspense>
                </div>
            </div>

            // ── Right: fixed dossier (30%), nothing selected → placeholder ──
            <aside class="flex min-w-0 flex-[3] flex-col bg-surface-container-lowest/40">
                <div class="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-center text-on-surface-variant">
                    <MaterialIcon name="badge" class="text-4xl opacity-50" />
                    <p class="text-label-md">"Select personnel to view dossier"</p>
                </div>
            </aside>
        </div>
    }
}

fn roster_table(users: Vec<AdminUserRow>) -> impl IntoView {
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
                {users.into_iter().map(roster_row).collect_view()}
            </tbody>
        </table>
    }
    .into_any()
}

fn roster_row(u: AdminUserRow) -> impl IntoView {
    // Nothing selected on load → not-active branches inlined.
    let name = if u.discord_handle.is_empty() {
        u.username.clone()
    } else {
        u.discord_handle.clone()
    };
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
    view! {
        <tr class="cursor-pointer transition-colors hover:bg-white/[0.03]">
            <td class="border-l-4 px-4 py-3 border-transparent">
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
