//! Personnel Roster (/admin/personnel) — ported from pages/admin.tsx `PersonnelRosterPage` +
//! `PersonnelDossier`. `<AdminGate>` → `/admin/users` Resource → a two-pane layout: a data table
//! (70%) of users + a fixed dossier pane (30%).
//!
//! T-159.25: fully interactive — live search (`?q=`), row selection, and the dossier with the
//! LIVE role editor (PATCH /admin/users/:discordId) and ban (POST …/ban, reason via the same
//! window.prompt React uses); Sort/Filter/Issue-Warning stay the React toast stubs.
#![allow(dead_code)]
use crate::dto::{AdminUserRow, Paginated};
use crate::ui::{cn, AdminGate, MaterialIcon};
use leptos::prelude::*;

/// Badge variant="success" class (ui/badge.tsx cn(), text-label-sm twMerge-dropped).
const BADGE_SUCCESS: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-success/30 bg-success/15 text-success";
const BADGE_ERROR: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-error-alert/30 bg-error-alert/10 text-error-alert";
const INPUT_CLASS: &str = "w-full rounded-lg border border-outline-variant/40 bg-surface px-3 py-2 text-label-md outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/40 disabled:opacity-50";
const ROLE_OPTIONS: [(&str, &str); 4] = [
    ("enlisted", "Enlisted"),
    ("leader", "Leader"),
    ("mission_maker", "Mission Maker"),
    ("admin", "Admin"),
];

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

fn display_name(u: &AdminUserRow) -> String {
    if u.discord_handle.is_empty() {
        u.username.clone()
    } else {
        u.discord_handle.clone()
    }
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
    let q = RwSignal::new(String::new());
    let selected_id = RwSignal::new(None::<String>);
    let roster = LocalResource::new(move || {
        let q = q.get();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = if q.is_empty() {
                    "/admin/users".to_string()
                } else {
                    format!(
                        "/admin/users?q={}",
                        js_sys::encode_uri_component(&q)
                            .as_string()
                            .unwrap_or_default()
                    )
                };
                crate::client::api_get::<Paginated<AdminUserRow>>(store, &path)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, q);
                None::<Paginated<AdminUserRow>>
            }
        }
    });
    let refetch = Callback::new(move |()| roster.refetch());
    let stub = move |msg: &'static str| {
        move |_| {
            #[cfg(target_arch = "wasm32")]
            crate::toast::use_toasts().success(msg);
            #[cfg(not(target_arch = "wasm32"))]
            let _ = msg;
        }
    };
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
                                on:click=stub("Sort options coming soon")
                                class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-label-sm text-on-surface transition hover:bg-white/5"
                            >
                                <MaterialIcon name="swap_vert" class="text-[18px]" />
                                "Sort"
                            </button>
                            <button
                                type="button"
                                on:click=stub("Filter options coming soon")
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
                            // value="" attribute at rest = React controlled-input parity (frozen V).
                            value=""
                            prop:value=move || q.get()
                            on:input=move |ev| q.set(event_target_value(&ev))
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
                                    Some(page) => roster_table(page.data, selected_id).into_any(),
                                    None => {
                                        view! { <p class="text-error">"Failed to load data."</p> }
                                            .into_any()
                                    }
                                })
                        }}
                    </Suspense>
                </div>
            </div>

            // ── Right: fixed dossier (30%) ──
            <aside class="flex min-w-0 flex-[3] flex-col bg-surface-container-lowest/40">
                <Suspense fallback=move || ()>
                    {move || {
                        let sel = selected_id.get();
                        let user = roster
                            .get()
                            .flatten()
                            .and_then(|page| {
                                page.data
                                    .iter()
                                    .find(|u| Some(&u.discord_id) == sel.as_ref())
                                    .cloned()
                            });
                        match user {
                            Some(u) => dossier(u, refetch).into_any(),
                            None => {
                                view! {
                                    <div class="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-center text-on-surface-variant">
                                        <MaterialIcon name="badge" class="text-4xl opacity-50" />
                                        <p class="text-label-md">
                                            "Select personnel to view dossier"
                                        </p>
                                    </div>
                                }
                                    .into_any()
                            }
                        }
                    }}
                </Suspense>
            </aside>
        </div>
    }
}

fn roster_table(users: Vec<AdminUserRow>, selected_id: RwSignal<Option<String>>) -> impl IntoView {
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

fn roster_row(u: AdminUserRow, selected_id: RwSignal<Option<String>>) -> impl IntoView {
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

/// The right-pane dossier (admin.tsx `PersonnelDossier`): profile header, service telemetry, the
/// inline role editor (live PATCH) and the docked actions (live ban; warning stays a stub).
fn dossier(u: AdminUserRow, refetch: Callback<()>) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, &refetch);
    let name = display_name(&u);
    let arma = if !u.arma_character.is_empty() {
        u.arma_character.clone()
    } else {
        u.arma_id
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unlinked Arma identity".to_string())
    };
    let uid = StoredValue::new(u.discord_id.clone());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = uid;
    let role = RwSignal::new(u.role.as_str().to_string());
    let prev_role = StoredValue::new(u.role.as_str().to_string());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = prev_role;
    let editing_role = RwSignal::new(false);
    let banned = RwSignal::new(u.is_banned);
    let ban_busy = RwSignal::new(false);

    let on_role_change = move |ev: leptos::ev::Event| {
        let next = event_target_value(&ev);
        role.set(next.clone());
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::toast::use_toasts();
            let path = format!("/admin/users/{}", uid.get_value());
            leptos::task::spawn_local(async move {
                match crate::client::api_patch::<serde_json::Value>(
                    store,
                    &path,
                    serde_json::json!({ "role": next }),
                )
                .await
                {
                    Ok(_) => {
                        toasts.success("Role updated");
                        refetch.run(());
                    }
                    Err(_) => {
                        toasts.error("Failed to update role");
                        role.set(prev_role.get_value());
                    }
                }
            });
        }
    };

    let on_ban = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if banned.get_untracked() || ban_busy.get_untracked() {
                return;
            }
            // window.prompt parity — null (Cancel) aborts; empty string sends no reason.
            let Some(win) = web_sys::window() else {
                return;
            };
            let Ok(reason) = win.prompt_with_message("Ban reason (optional):") else {
                return;
            };
            let Some(reason) = reason else {
                return; // Cancel
            };
            ban_busy.set(true);
            let toasts = crate::toast::use_toasts();
            let path = format!("/admin/users/{}/ban", uid.get_value());
            let body = if reason.is_empty() {
                serde_json::json!({})
            } else {
                serde_json::json!({ "reason": reason })
            };
            leptos::task::spawn_local(async move {
                match crate::client::api_post_ok(store, &path, body).await {
                    Ok(()) => {
                        toasts.success("Personnel banned");
                        banned.set(true);
                        refetch.run(());
                    }
                    Err(_) => toasts.error("Ban failed"),
                }
                ban_busy.set(false);
            });
        }
    };
    let warn_stub = move |_| {
        #[cfg(target_arch = "wasm32")]
        crate::toast::use_toasts().success("Warning issued (mock)");
    };

    view! {
        <div class="flex min-h-0 flex-1 flex-col">
            <div class="custom-scrollbar min-h-0 flex-1 overflow-y-auto p-6">
                <div class="flex flex-col items-center text-center">
                    {avatar(&name, "size-20 text-xl")}
                    <h2 class="mt-4 text-headline-sm text-on-surface">{name.clone()}</h2>
                    <p class="mt-1 font-mono text-code-md text-on-surface-variant">
                        {u.discord_id.clone()}
                    </p>
                    <p class="mt-2 text-label-md text-on-surface-variant">{arma}</p>
                </div>

                <div class="mt-6 grid grid-cols-2 gap-3">
                    {stat("Deployments", "—".to_string())}
                    {stat_reactive("Current Rank", move || role.get().to_uppercase())}
                    {stat("Warnings", u.warnings.to_string())}
                    {stat_reactive(
                        "Status",
                        move || if banned.get() { "Banned".into() } else { "Active".into() },
                    )}
                </div>

                {move || {
                    editing_role
                        .get()
                        .then(|| {
                            view! {
                                <div class="mt-4">
                                    <label class="mb-1 block text-label-sm text-on-surface-variant uppercase">
                                        "Role"
                                    </label>
                                    <select
                                        prop:value=move || role.get()
                                        on:change=on_role_change
                                        class=INPUT_CLASS
                                    >
                                        {ROLE_OPTIONS
                                            .iter()
                                            .map(|(v, l)| view! { <option value=*v>{*l}</option> })
                                            .collect_view()}
                                    </select>
                                </div>
                            }
                        })
                }}
            </div>

            <div class="flex flex-col gap-2 border-t border-white/10 p-6">
                <button
                    type="button"
                    on:click=move |_| editing_role.update(|v| *v = !*v)
                    class="flex items-center justify-center gap-2 rounded-lg border border-white/10 py-2.5 text-label-md text-on-surface transition hover:bg-white/5"
                >
                    <MaterialIcon name="manage_accounts" class="text-[18px]" />
                    "Edit Roles"
                </button>
                <button
                    type="button"
                    on:click=warn_stub
                    class="flex items-center justify-center gap-2 rounded-lg border border-tactical-yellow/30 py-2.5 text-label-md text-tactical-yellow transition hover:bg-tactical-yellow/10"
                >
                    <MaterialIcon name="warning" class="text-[18px]" />
                    "Issue Warning"
                </button>
                <button
                    type="button"
                    on:click=on_ban
                    prop:disabled=move || banned.get() || ban_busy.get()
                    class="flex items-center justify-center gap-2 rounded-lg bg-error-alert/15 py-2.5 text-label-md font-medium text-error-alert transition hover:bg-error-alert/25 disabled:cursor-not-allowed disabled:opacity-40"
                >
                    <MaterialIcon name="gavel" class="text-[18px]" />
                    {move || if banned.get() { "Personnel Banned" } else { "Ban Personnel" }}
                </button>
            </div>
        </div>
    }
}

fn stat(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-white/10 bg-white/[0.02] px-3 py-2.5 text-center">
            <p class="text-label-sm text-on-surface-variant uppercase">{label}</p>
            <p class="mt-0.5 truncate text-label-md font-semibold text-on-surface">{value}</p>
        </div>
    }
}
fn stat_reactive(
    label: &'static str,
    value: impl Fn() -> String + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-white/10 bg-white/[0.02] px-3 py-2.5 text-center">
            <p class="text-label-sm text-on-surface-variant uppercase">{label}</p>
            <p class="mt-0.5 truncate text-label-md font-semibold text-on-surface">{move || value()}</p>
        </div>
    }
}
