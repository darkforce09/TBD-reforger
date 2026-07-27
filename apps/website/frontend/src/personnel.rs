//! Personnel Roster (/admin/personnel) — ported from pages/admin.tsx `PersonnelRosterPage` +
//! `PersonnelDossier`. `<AdminGate>` → `/admin/users` Resource → a two-pane layout: a data table
//! (70%) of users + a fixed dossier pane (30%).
//!
//! T-159.25: fully interactive — live search (`?q=`), row selection, and the dossier with the
//! LIVE role editor (PATCH /admin/users/:discordId) and ban (POST …/ban).
//!
//! T-323: the ban reason is **required**. T-317 made the backend reject a missing or
//! whitespace-only `reason` — before that fix a re-ban with no reason silently erased both the
//! previous reason and `banned_at`. This page was the stale half of that change: it advertised
//! the reason as optional and posted `{}` when the operator left it blank, which now 400s. A
//! blank answer is refused here without a request, and a server-side 400 is shown verbatim
//! instead of a flat "Ban failed".
//!
//! T-342: ban + warning reasons use a real `Dialog` (not `window.prompt`). Native prompts cannot
//! be intercepted by the browser gate (`web_sys` bypasses JS overrides; CDP blocks the
//! renderer). Confirm stays disabled until the reason is non-empty — the required-reason rule
//! is inline, not a toast after the fact. Role PATCH errors use `api_error_message` the same
//! way ban/unban/warn already do.
//!
//! T-247: Personnel is the SPA caller for `POST /admin/roles/sync` (was curl-only). The header
//! "Sync Roles" control posts that path, toasts the `updated` count, and refetches the roster.
//!
//! T-268: Issue Warning posts `POST /admin/users/:discordId/warnings` with the same required-
//! reason Dialog as ban (the prior mock success toast is gone). When `is_banned`, the dossier
//! shows **Unban** → `DELETE …/ban` instead of a dead "Personnel Banned" label. Header Sort /
//! Filter cycle client-side modes over the loaded page (roster API has no sort/filter query).
//!
//! T-448: dossier Deployments binds `AdminUserRow.total_deployments` (API `RosterRow` projects
//! `users.total_deployments`) — integer string, including `0` (no em-dash placeholder).
#![allow(dead_code)]
use crate::dto::{AdminUserRow, Paginated};
use crate::ui::{cn, AdminGate, Dialog, MaterialIcon};
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

/// Live admin route that re-applies `discord_roles` mappings (`resync_all_roles`).
/// Locked here so the Personnel button cannot drift off the Axum registration in `app.rs`.
const ADMIN_ROLES_SYNC_PATH: &str = "/admin/roles/sync";

/// Path suffix templates locked to `app.rs` registrations (T-268). Ban and warnings share the
/// `:discordId` segment; unban is DELETE on the same ban path.
fn admin_user_ban_path(discord_id: &str) -> String {
    format!("/admin/users/{discord_id}/ban")
}

fn admin_user_warnings_path(discord_id: &str) -> String {
    format!("/admin/users/{discord_id}/warnings")
}

/// Client-side roster sort — the list endpoint only offers `ORDER BY username ASC` + `?q=`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SortMode {
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

    fn next(self) -> Self {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    fn label(self) -> &'static str {
        match self {
            SortMode::NameAsc => "Sort: Name",
            SortMode::WarningsDesc => "Sort: Warnings",
            SortMode::RoleAsc => "Sort: Role",
            SortMode::BannedFirst => "Sort: Banned",
        }
    }
}

/// Client-side roster filter — Active / Banned / All over the loaded page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FilterMode {
    All,
    Active,
    Banned,
}

impl FilterMode {
    const ALL: [FilterMode; 3] = [FilterMode::All, FilterMode::Active, FilterMode::Banned];

    fn next(self) -> Self {
        let i = Self::ALL.iter().position(|m| *m == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    fn label(self) -> &'static str {
        match self {
            FilterMode::All => "Filter: All",
            FilterMode::Active => "Filter: Active",
            FilterMode::Banned => "Filter: Banned",
        }
    }
}

fn apply_roster_filter(mut users: Vec<AdminUserRow>, mode: FilterMode) -> Vec<AdminUserRow> {
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

fn apply_roster_sort(mut users: Vec<AdminUserRow>, mode: SortMode) -> Vec<AdminUserRow> {
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

/// Read `{ "updated": N }` from the roles-sync response. A missing/non-integer `updated` is an
/// error so a 2xx with `{}` cannot toast as a completed sync.
fn roles_sync_updated_count(body: &serde_json::Value) -> Result<i64, &'static str> {
    body.get("updated")
        .and_then(|v| v.as_i64())
        .ok_or("roles sync response missing updated count")
}

fn roles_sync_success_message(updated: i64) -> String {
    format!("Discord roles resynced ({updated} user(s) updated)")
}

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
    let sync_busy = RwSignal::new(false);
    let sort_mode = RwSignal::new(SortMode::NameAsc);
    let filter_mode = RwSignal::new(FilterMode::All);
    let on_sync_roles = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if sync_busy.get_untracked() {
                return;
            }
            sync_busy.set(true);
            let toasts = crate::toast::use_toasts();
            leptos::task::spawn_local(async move {
                match crate::client::api_post::<serde_json::Value>(
                    store,
                    ADMIN_ROLES_SYNC_PATH,
                    serde_json::json!({}),
                )
                .await
                {
                    Ok(body) => match roles_sync_updated_count(&body) {
                        Ok(n) => {
                            toasts.success(roles_sync_success_message(n));
                            refetch.run(());
                        }
                        Err(_) => {
                            toasts.error("Role sync returned an unexpected response");
                        }
                    },
                    Err(e) => {
                        toasts.error(crate::client::api_error_message(&e, "Role sync failed"))
                    }
                }
                sync_busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (store, sync_busy, refetch);
        }
    };
    let on_cycle_sort = move |_| sort_mode.update(|m| *m = m.next());
    let on_cycle_filter = move |_| filter_mode.update(|m| *m = m.next());
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
                                on:click=on_sync_roles
                                prop:disabled=move || sync_busy.get()
                                class="flex items-center gap-1.5 rounded-full border border-primary/40 bg-primary/10 px-4 py-2 text-label-sm text-primary transition hover:bg-primary/20 disabled:opacity-50"
                            >
                                <MaterialIcon name="sync" class="text-[18px]" />
                                {move || {
                                    if sync_busy.get() {
                                        "Syncing…"
                                    } else {
                                        "Sync Roles"
                                    }
                                }}
                            </button>
                            <button
                                type="button"
                                on:click=on_cycle_sort
                                class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-label-sm text-on-surface transition hover:bg-white/5"
                            >
                                <MaterialIcon name="swap_vert" class="text-[18px]" />
                                {move || sort_mode.get().label()}
                            </button>
                            <button
                                type="button"
                                on:click=on_cycle_filter
                                class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-label-sm text-on-surface transition hover:bg-white/5"
                            >
                                <MaterialIcon name="filter_list" class="text-[18px]" />
                                {move || filter_mode.get().label()}
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
                            let sort = sort_mode.get();
                            let filter = filter_mode.get();
                            roster
                                .get()
                                .map(|opt| match opt {
                                    Some(page) => {
                                        let users =
                                            apply_roster_sort(apply_roster_filter(page.data, filter), sort);
                                        roster_table(users, selected_id).into_any()
                                    }
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

/// What the client does with a **required reason** field (ban **or** warning — both endpoints
/// reject blank/whitespace the same way).
///
/// T-323 — the three outcomes are genuinely different and must not collapse into one. Cancel /
/// dismiss is an abandoned action; blank/whitespace is an attempted write missing its reason;
/// anything else is a reason to send. Split out as a plain function so host-side unit tests pin
/// the decision (T-342: the Dialog confirm stays disabled while empty, so the UI never toasts a
/// Reject — `classify_ban_reason` is still the shared trim/empty gate on submit).
#[derive(Debug, PartialEq, Eq)]
enum BanReason {
    /// Dialog Cancel / dismiss: the operator backed out. No request, no toast.
    Abort,
    /// Blank or whitespace-only answer. Refuse locally: posting `{}` is exactly the 400 T-317
    /// rejects, and substituting a placeholder ("No reason given") would reintroduce the
    /// unexplained ban that ticket exists to stop. The Dialog disables Confirm while empty so
    /// this branch is defensive, not the operator-facing path.
    Reject,
    /// A real reason, trimmed. The server stores `reason.trim()` and rejects whitespace-only, so
    /// trimming here keeps client and server from disagreeing about what counts as empty.
    Send(String),
}

fn classify_ban_reason(answer: Option<&str>) -> BanReason {
    match answer {
        None => BanReason::Abort,
        Some(raw) => {
            let reason = raw.trim();
            if reason.is_empty() {
                BanReason::Reject
            } else {
                BanReason::Send(reason.to_string())
            }
        }
    }
}

/// True when the Dialog Confirm may fire — non-empty after trim (mirrors `BanReason::Send`).
fn reason_confirm_enabled(reason: &str) -> bool {
    matches!(classify_ban_reason(Some(reason)), BanReason::Send(_))
}

/// The right-pane dossier (admin.tsx `PersonnelDossier`): profile header, service telemetry, the
/// inline role editor (live PATCH), Issue Warning (live POST …/warnings), Ban (live POST …/ban),
/// and Unban when banned (live DELETE …/ban). Ban/warn reasons open a `Dialog` (T-342).
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
    let warn_busy = RwSignal::new(false);
    let warnings = RwSignal::new(u.warnings);
    // T-342 — Dialog reason fields (never window.prompt).
    let ban_open = RwSignal::new(false);
    let ban_reason = RwSignal::new(String::new());
    let warn_open = RwSignal::new(false);
    let warn_reason = RwSignal::new(String::new());

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
                    Err(e) => {
                        toasts.error(crate::client::api_error_message(
                            &e,
                            "Failed to update role",
                        ));
                        role.set(prev_role.get_value());
                    }
                }
            });
        }
    };

    let on_ban = move |_| {
        if banned.get_untracked() || ban_busy.get_untracked() {
            return;
        }
        ban_reason.set(String::new());
        ban_open.set(true);
    };

    let on_confirm_ban = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if banned.get_untracked() || ban_busy.get_untracked() {
                return;
            }
            let toasts = crate::toast::use_toasts();
            let reason = match classify_ban_reason(Some(ban_reason.get_untracked().as_str())) {
                BanReason::Abort | BanReason::Reject => return,
                BanReason::Send(reason) => reason,
            };
            ban_busy.set(true);
            let path = admin_user_ban_path(&uid.get_value());
            let body = serde_json::json!({ "reason": reason });
            leptos::task::spawn_local(async move {
                match crate::client::api_post_ok(store, &path, body).await {
                    Ok(()) => {
                        toasts.success("Personnel banned");
                        banned.set(true);
                        ban_open.set(false);
                        ban_reason.set(String::new());
                        refetch.run(());
                    }
                    // The 400 body says exactly what is wrong; a flat "Ban failed" threw that
                    // away. `api_error_message` is the house helper (event_hub/missions/…).
                    Err(e) => toasts.error(crate::client::api_error_message(&e, "Ban failed")),
                }
                ban_busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (store, ban_busy, ban_open, ban_reason, banned, refetch);
        }
    };

    let on_unban = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if !banned.get_untracked() || ban_busy.get_untracked() {
                return;
            }
            let toasts = crate::toast::use_toasts();
            ban_busy.set(true);
            let path = admin_user_ban_path(&uid.get_value());
            leptos::task::spawn_local(async move {
                match crate::client::api_delete(store, &path).await {
                    Ok(()) => {
                        toasts.success("Personnel unbanned");
                        banned.set(false);
                        refetch.run(());
                    }
                    Err(e) => toasts.error(crate::client::api_error_message(&e, "Unban failed")),
                }
                ban_busy.set(false);
            });
        }
    };

    let on_warn = move |_| {
        if warn_busy.get_untracked() {
            return;
        }
        warn_reason.set(String::new());
        warn_open.set(true);
    };

    let on_confirm_warn = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if warn_busy.get_untracked() {
                return;
            }
            let toasts = crate::toast::use_toasts();
            let reason = match classify_ban_reason(Some(warn_reason.get_untracked().as_str())) {
                BanReason::Abort | BanReason::Reject => return,
                BanReason::Send(reason) => reason,
            };
            warn_busy.set(true);
            let path = admin_user_warnings_path(&uid.get_value());
            let body = serde_json::json!({ "reason": reason });
            leptos::task::spawn_local(async move {
                match crate::client::api_post_ok(store, &path, body).await {
                    Ok(()) => {
                        toasts.success("Warning issued");
                        warnings.update(|n| *n = n.saturating_add(1));
                        warn_open.set(false);
                        warn_reason.set(String::new());
                        refetch.run(());
                    }
                    Err(e) => toasts.error(crate::client::api_error_message(&e, "Warning failed")),
                }
                warn_busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (store, warn_busy, warn_open, warn_reason, warnings, refetch);
        }
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
                    {stat("Deployments", u.total_deployments.to_string())}
                    {stat_reactive("Current Rank", move || role.get().to_uppercase())}
                    {stat_reactive("Warnings", move || warnings.get().to_string())}
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
                    on:click=on_warn
                    prop:disabled=move || warn_busy.get()
                    data-testid="personnel-warn"
                    class="flex items-center justify-center gap-2 rounded-lg border border-tactical-yellow/30 py-2.5 text-label-md text-tactical-yellow transition hover:bg-tactical-yellow/10 disabled:cursor-not-allowed disabled:opacity-40"
                >
                    <MaterialIcon name="warning" class="text-[18px]" />
                    {move || if warn_busy.get() { "Issuing…" } else { "Issue Warning" }}
                </button>
                {move || {
                    if banned.get() {
                        view! {
                            <button
                                type="button"
                                on:click=on_unban
                                prop:disabled=move || ban_busy.get()
                                data-testid="personnel-unban"
                                class="flex items-center justify-center gap-2 rounded-lg border border-success/30 bg-success/10 py-2.5 text-label-md font-medium text-success transition hover:bg-success/20 disabled:cursor-not-allowed disabled:opacity-40"
                            >
                                <MaterialIcon name="lock_open" class="text-[18px]" />
                                {move || if ban_busy.get() { "Unbanning…" } else { "Unban Personnel" }}
                            </button>
                        }
                            .into_any()
                    } else {
                        view! {
                            <button
                                type="button"
                                on:click=on_ban
                                prop:disabled=move || ban_busy.get()
                                data-testid="personnel-ban"
                                class="flex items-center justify-center gap-2 rounded-lg bg-error-alert/15 py-2.5 text-label-md font-medium text-error-alert transition hover:bg-error-alert/25 disabled:cursor-not-allowed disabled:opacity-40"
                            >
                                <MaterialIcon name="gavel" class="text-[18px]" />
                                {move || if ban_busy.get() { "Banning…" } else { "Ban Personnel" }}
                            </button>
                        }
                            .into_any()
                    }
                }}
            </div>

            // T-342 — ban reason Dialog (driveable by the browser gate; no window.prompt).
            <Dialog
                open=ban_open
                title="Ban personnel?"
                description="A reason is required. This action is recorded on the roster."
            >
                <label class="mb-1 block text-label-sm text-on-surface-variant uppercase" for="personnel-ban-reason">
                    "Ban reason"
                </label>
                <textarea
                    id="personnel-ban-reason"
                    data-testid="personnel-ban-reason"
                    aria-label="Ban reason (required)"
                    prop:value=move || ban_reason.get()
                    on:input=move |ev| ban_reason.set(event_target_value(&ev))
                    placeholder="Reason (required)"
                    rows="4"
                    class=INPUT_CLASS
                ></textarea>
                <div class="mt-5 flex justify-end gap-2">
                    <button
                        type="button"
                        data-testid="personnel-ban-cancel"
                        on:click=move |_| {
                            ban_open.set(false);
                            ban_reason.set(String::new());
                        }
                        class="rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
                    >
                        "Cancel"
                    </button>
                    <button
                        type="button"
                        data-testid="personnel-ban-confirm"
                        on:click=on_confirm_ban
                        prop:disabled=move || {
                            ban_busy.get() || !reason_confirm_enabled(&ban_reason.get())
                        }
                        class="rounded-md bg-error-alert/20 px-3 py-1.5 text-label-md text-error-alert transition-colors hover:bg-error-alert/30 disabled:cursor-not-allowed disabled:opacity-40"
                    >
                        {move || if ban_busy.get() { "Banning…" } else { "Ban personnel" }}
                    </button>
                </div>
            </Dialog>

            // T-342 — warning reason Dialog (same required-reason contract as ban).
            <Dialog
                open=warn_open
                title="Issue warning?"
                description="A reason is required. The warning count on this dossier updates after a successful POST."
            >
                <label class="mb-1 block text-label-sm text-on-surface-variant uppercase" for="personnel-warn-reason">
                    "Warning reason"
                </label>
                <textarea
                    id="personnel-warn-reason"
                    data-testid="personnel-warn-reason"
                    aria-label="Warning reason (required)"
                    prop:value=move || warn_reason.get()
                    on:input=move |ev| warn_reason.set(event_target_value(&ev))
                    placeholder="Reason (required)"
                    rows="4"
                    class=INPUT_CLASS
                ></textarea>
                <div class="mt-5 flex justify-end gap-2">
                    <button
                        type="button"
                        data-testid="personnel-warn-cancel"
                        on:click=move |_| {
                            warn_open.set(false);
                            warn_reason.set(String::new());
                        }
                        class="rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
                    >
                        "Cancel"
                    </button>
                    <button
                        type="button"
                        data-testid="personnel-warn-confirm"
                        on:click=on_confirm_warn
                        prop:disabled=move || {
                            warn_busy.get() || !reason_confirm_enabled(&warn_reason.get())
                        }
                        class="rounded-md border border-tactical-yellow/40 bg-tactical-yellow/15 px-3 py-1.5 text-label-md text-tactical-yellow transition-colors hover:bg-tactical-yellow/25 disabled:cursor-not-allowed disabled:opacity-40"
                    >
                        {move || if warn_busy.get() { "Issuing…" } else { "Issue warning" }}
                    </button>
                </div>
            </Dialog>
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

#[cfg(test)]
mod tests {
    use super::{
        admin_user_ban_path, admin_user_warnings_path, apply_roster_filter, apply_roster_sort,
        classify_ban_reason, reason_confirm_enabled, roles_sync_success_message,
        roles_sync_updated_count, BanReason, FilterMode, SortMode, ADMIN_ROLES_SYNC_PATH,
    };
    use crate::dto::AdminUserRow;
    use crate::nav::Role;

    fn production_src() -> &'static str {
        include_str!("personnel.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("production source before tests module")
    }

    fn row(
        discord_id: &str,
        username: &str,
        role: Role,
        is_banned: bool,
        warnings: i64,
    ) -> AdminUserRow {
        AdminUserRow {
            discord_id: discord_id.into(),
            username: username.into(),
            discord_handle: username.into(),
            arma_id: None,
            arma_character: String::new(),
            role,
            is_banned,
            warnings,
            total_deployments: 0,
        }
    }

    #[test]
    fn admin_user_row_deserializes_total_deployments() {
        // T-448: wire field must survive the AdminUserRow decode — zero is a real count.
        let u: AdminUserRow = serde_json::from_str(
            r#"{"discord_id":"1","username":"u","discord_handle":"u","arma_id":null,"arma_character":"","role":"enlisted","is_banned":false,"warnings":0,"total_deployments":0}"#,
        )
        .expect("AdminUserRow with total_deployments=0");
        assert_eq!(u.total_deployments, 0);
        let u17: AdminUserRow = serde_json::from_str(
            r#"{"discord_id":"1","username":"u","discord_handle":"u","arma_id":null,"arma_character":"","role":"enlisted","is_banned":false,"warnings":0,"total_deployments":17}"#,
        )
        .expect("AdminUserRow with total_deployments=17");
        assert_eq!(u17.total_deployments, 17);
    }

    #[test]
    fn dossier_deployments_binds_total_deployments_not_em_dash() {
        // T-448 / T-461: dossier used to hardcode stat("Deployments", "—") while the API
        // column existed. Wave 23 adversarial: a conditional
        // `if u.total_deployments == 0 { "—" } else { u.total_deployments.to_string() }`
        // still contained the bind needle and false-greened. Require the exact live call
        // (no conditional) and forbid any em-dash between Deployments and the next sibling.
        let production = production_src();

        // Exact live bind — assembled so this test's own source cannot satisfy it.
        let exact = format!(
            "{}{}",
            r#"stat("Deployments", "#, "u.total_deployments.to_string())"
        );
        assert!(
            production.contains(&exact),
            "dossier Deployments must be exactly `{exact}` (no conditional / no em-dash)"
        );

        let start = production
            .find(r#"stat("Deployments""#)
            .expect("Deployments stat call present");
        let region = &production[start..];
        let end = region
            .find("Current Rank")
            .expect("Current Rank sibling after Deployments");
        let deployments_region = &region[..end];
        assert!(
            !deployments_region.contains('—'),
            "Deployments/stat region must not contain em-dash (zero is a real count; \
             conditional zero→— is a fail)"
        );
        assert!(
            !deployments_region.contains("total_deployments == 0"),
            "Deployments/stat region must not conditionalize on total_deployments == 0"
        );
    }

    #[test]
    fn admin_roles_sync_path_matches_live_api_route() {
        // T-247: Personnel is the only SPA caller. Open the live Axum router source so a
        // drift off `.route("/admin/roles/sync", …)` fails this test — not a const echo of
        // itself (which stayed green while never examining app.rs).
        const APP_RS: &str =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../api/src/app.rs"));
        let live_registration = format!(r#".route("{ADMIN_ROLES_SYNC_PATH}""#);
        assert!(
            APP_RS.contains(&live_registration),
            "apps/website/api/src/app.rs must register {live_registration}, …); \
             Personnel posts ADMIN_ROLES_SYNC_PATH"
        );
        assert_eq!(ADMIN_ROLES_SYNC_PATH, "/admin/roles/sync");
    }

    #[test]
    fn admin_ban_and_warnings_paths_match_live_api_routes() {
        // T-268: path helpers must track app.rs — a const echo of itself stays green forever.
        // Ban/warnings registrations are multi-line `.route(\n  "…"` — match the path string.
        const APP_RS: &str =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../api/src/app.rs"));
        assert!(
            APP_RS.contains(r#""/admin/users/{discordId}/ban""#),
            "app.rs must register ban/unban on /admin/users/{{discordId}}/ban"
        );
        assert!(
            APP_RS.contains(r#""/admin/users/{discordId}/warnings""#),
            "app.rs must register warnings on /admin/users/{{discordId}}/warnings"
        );
        assert!(
            APP_RS.contains("unban_user"),
            "app.rs ban route must wire DELETE to unban_user"
        );
        assert_eq!(admin_user_ban_path("42"), "/admin/users/42/ban");
        assert_eq!(admin_user_warnings_path("42"), "/admin/users/42/warnings");
    }

    #[test]
    fn issue_warning_is_not_a_mock_toast() {
        // T-268 defect: Issue Warning toasted a fake success while POST …/warnings worked.
        // Forbidden / required phrases assembled so include_str cannot false-green off this test.
        let production = production_src();
        let mock_toast = format!("{}{}", "Warning issued ", "(mock)");
        let real_toast = format!("{}{}", "toasts.success(", r#""Warning issued")"#);
        assert!(
            !production.contains(&mock_toast),
            "Issue Warning must not toast a mock success (perturbation: reintroduce the mock toast)"
        );
        assert!(
            production.contains("admin_user_warnings_path")
                && production.contains(&format!("{}{}", "api_post_ok", "(store, &path, body)")),
            "Issue Warning must POST via admin_user_warnings_path + api_post_ok"
        );
        assert!(
            production.contains("personnel-warn-reason")
                && production.contains("personnel-warn-confirm"),
            "warning reason must be collected via a driveable Dialog (T-342)"
        );
        assert!(
            production.contains(&real_toast),
            "success toast after a real POST must say Warning issued"
        );
    }

    #[test]
    fn ban_and_warn_use_dialog_not_window_prompt() {
        // T-342: native prompt blocks CDP; Dialog + testids are the gate-driveable path.
        // Module docs may name the retired path; pin the live call sites only.
        let production = production_src();
        assert!(
            !production.contains("prompt_with_message")
                && !production.contains("win.prompt")
                && !production.contains("Prompt::"),
            "personnel must not call web_sys prompt APIs"
        );
        for needle in [
            "data-testid=\"personnel-ban-reason\"",
            "data-testid=\"personnel-ban-confirm\"",
            "data-testid=\"personnel-warn-reason\"",
            "data-testid=\"personnel-warn-confirm\"",
            "reason_confirm_enabled",
        ] {
            assert!(
                production.contains(needle),
                "missing Dialog drive surface: {needle}"
            );
        }
        assert!(
            production.contains("<Dialog")
                && production.contains("title=\"Ban personnel?\"")
                && production.contains("title=\"Issue warning?\""),
            "ban and warn must each open a Dialog"
        );
    }

    #[test]
    fn role_patch_surfaces_api_error_message() {
        // T-342 fold-in: role editor used to discard the server body with a flat string.
        let production = production_src();
        let flat = format!("{}{}", r#"toasts.error("Failed to update role")"#, "");
        assert!(
            !production.contains(&flat),
            "role PATCH must not toast a flat Failed to update role (discarded server message)"
        );
        assert!(
            production.contains("api_error_message")
                && production.contains("Failed to update role"),
            "role PATCH Err must use api_error_message(..., \"Failed to update role\")"
        );
    }

    #[test]
    fn unban_control_deletes_ban_when_banned() {
        // T-268: bans were irreversible from the SPA. Needles assembled so include_str cannot
        // false-green off this assert's own string literals.
        let production = production_src();
        let delete_call = format!("{}{}", "api_delete", "(store, &path)");
        let unban_label = format!("{}{}", "Unban ", "Personnel");
        let unban_testid = format!("{}{}", "personnel-", "unban");
        assert!(
            production.contains(&delete_call),
            "Unban must DELETE via api_delete (perturbation: drop api_delete call)"
        );
        assert!(
            production.contains("admin_user_ban_path") && production.contains(&unban_label),
            "banned dossier must expose Unban Personnel on the ban path"
        );
        assert!(
            production.contains(&unban_testid),
            "unban control needs a stable testid"
        );
    }

    #[test]
    fn sort_and_filter_are_not_toast_stubs() {
        let production = production_src();
        // Assemble so the assert literals cannot false-green the include_str scan.
        let sort_stub = format!("{}{}", "Sort options ", "coming soon");
        let filter_stub = format!("{}{}", "Filter options ", "coming soon");
        assert!(
            !production.contains(&sort_stub) && !production.contains(&filter_stub),
            "Sort/Filter must not toast stub copy"
        );
        assert!(
            production.contains("apply_roster_sort") && production.contains("apply_roster_filter"),
            "header Sort/Filter must drive apply_roster_sort / apply_roster_filter"
        );
    }

    #[test]
    fn roles_sync_updated_count_reads_integer() {
        let body = serde_json::json!({ "updated": 12 });
        assert_eq!(roles_sync_updated_count(&body), Ok(12));
    }

    #[test]
    fn roles_sync_updated_count_rejects_missing_or_non_integer() {
        // A vacuous `{}` 2xx must not toast as success — that is how curl-only stayed invisible.
        assert!(roles_sync_updated_count(&serde_json::json!({})).is_err());
        assert!(roles_sync_updated_count(&serde_json::json!({ "updated": "12" })).is_err());
        assert!(roles_sync_updated_count(&serde_json::json!({ "updated": null })).is_err());
    }

    #[test]
    fn roles_sync_success_message_names_the_count() {
        assert_eq!(
            roles_sync_success_message(0),
            "Discord roles resynced (0 user(s) updated)"
        );
        assert_eq!(
            roles_sync_success_message(3),
            "Discord roles resynced (3 user(s) updated)"
        );
    }

    #[test]
    fn cancel_aborts_and_sends_nothing() {
        // Dialog Cancel / dismiss → None. Abort carries no toast: the operator chose not to ban.
        assert_eq!(classify_ban_reason(None), BanReason::Abort);
    }

    #[test]
    fn ok_with_blank_is_refused_before_any_request() {
        // The T-323 break: this branch used to POST `{}`, which T-317 correctly answers with
        // 400 "reason is required". Reject means the client never makes the request at all.
        // T-342: Dialog Confirm is disabled while empty (see reason_confirm_enabled).
        assert_eq!(classify_ban_reason(Some("")), BanReason::Reject);
        assert!(!reason_confirm_enabled(""));
    }

    #[test]
    fn whitespace_only_is_refused_too() {
        // The server rejects a whitespace-only reason. Agreeing here keeps the operator from
        // seeing a 400 for input that looked non-empty in the field.
        for blank in ["   ", "\t", "\n", " \t\n "] {
            assert_eq!(
                classify_ban_reason(Some(blank)),
                BanReason::Reject,
                "{blank:?}"
            );
            assert!(!reason_confirm_enabled(blank), "{blank:?}");
        }
    }

    #[test]
    fn a_real_reason_is_sent_trimmed() {
        // The server stores `reason.trim()`, so the client sends the same bytes it will store.
        assert_eq!(
            classify_ban_reason(Some("  Repeated TK after warning  ")),
            BanReason::Send("Repeated TK after warning".to_string())
        );
        assert_eq!(
            classify_ban_reason(Some("Repeated TK after warning")),
            BanReason::Send("Repeated TK after warning".to_string())
        );
        assert!(reason_confirm_enabled("  Repeated TK after warning  "));
    }

    #[test]
    fn filter_active_and_banned() {
        let users = vec![
            row("1", "Alice", Role::Enlisted, false, 0),
            row("2", "Bob", Role::Enlisted, true, 1),
            row("3", "Cara", Role::Leader, false, 0),
        ];
        let active = apply_roster_filter(users.clone(), FilterMode::Active);
        assert_eq!(
            active
                .iter()
                .map(|u| u.username.as_str())
                .collect::<Vec<_>>(),
            vec!["Alice", "Cara"]
        );
        let banned = apply_roster_filter(users, FilterMode::Banned);
        assert_eq!(
            banned
                .iter()
                .map(|u| u.username.as_str())
                .collect::<Vec<_>>(),
            vec!["Bob"]
        );
    }

    #[test]
    fn sort_warnings_desc_and_banned_first() {
        let users = vec![
            row("1", "Alice", Role::Enlisted, false, 0),
            row("2", "Bob", Role::Admin, true, 1),
            row("3", "Cara", Role::Leader, false, 5),
        ];
        let by_warn = apply_roster_sort(users.clone(), SortMode::WarningsDesc);
        assert_eq!(
            by_warn
                .iter()
                .map(|u| u.username.as_str())
                .collect::<Vec<_>>(),
            vec!["Cara", "Bob", "Alice"]
        );
        let banned_first = apply_roster_sort(users, SortMode::BannedFirst);
        assert_eq!(banned_first[0].username, "Bob");
        assert!(banned_first[0].is_banned);
    }

    #[test]
    fn sort_and_filter_modes_cycle() {
        assert_eq!(SortMode::NameAsc.next(), SortMode::WarningsDesc);
        assert_eq!(SortMode::BannedFirst.next(), SortMode::NameAsc);
        assert_eq!(FilterMode::All.next(), FilterMode::Active);
        assert_eq!(FilterMode::Banned.next(), FilterMode::All);
    }
}
