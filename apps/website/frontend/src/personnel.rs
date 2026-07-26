//! Personnel Roster (/admin/personnel) — ported from pages/admin.tsx `PersonnelRosterPage` +
//! `PersonnelDossier`. `<AdminGate>` → `/admin/users` Resource → a two-pane layout: a data table
//! (70%) of users + a fixed dossier pane (30%).
//!
//! T-159.25: fully interactive — live search (`?q=`), row selection, and the dossier with the
//! LIVE role editor (PATCH /admin/users/:discordId) and ban (POST …/ban, reason via the same
//! window.prompt React uses); Sort/Filter/Issue-Warning stay the React toast stubs.
//!
//! T-323: the ban reason is **required**. T-317 made the backend reject a missing or
//! whitespace-only `reason` — before that fix a re-ban with no reason silently erased both the
//! previous reason and `banned_at`. This page was the stale half of that change: it advertised
//! the reason as optional and posted `{}` when the operator left it blank, which now 400s. The
//! prompt no longer lies, a blank answer is refused here without a request, and a server-side
//! 400 is shown verbatim instead of a flat "Ban failed".
//!
//! T-247: Personnel is the SPA caller for `POST /admin/roles/sync` (was curl-only). The header
//! "Sync Roles" control posts that path, toasts the `updated` count, and refetches the roster.
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

/// Live admin route that re-applies `discord_roles` mappings (`resync_all_roles`).
/// Locked here so the Personnel button cannot drift off the Axum registration in `app.rs`.
const ADMIN_ROLES_SYNC_PATH: &str = "/admin/roles/sync";

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

/// What the client does with whatever `window.prompt` handed back for a ban reason.
///
/// T-323 — the three outcomes are genuinely different and must not collapse into one. Cancel is
/// an abandoned action; OK-with-blank is an attempted ban that is missing its reason; anything
/// else is a ban to send. Split out as a plain function because the caller is a wasm-only
/// closure behind a `window.prompt`, which no test can drive — this way the decision itself is
/// pinned by host-side unit tests.
#[derive(Debug, PartialEq, Eq)]
enum BanReason {
    /// `prompt` → `null` (Cancel): the operator backed out. No request, no toast.
    Abort,
    /// OK with a blank or whitespace-only answer. Refuse locally: posting `{}` is exactly the
    /// 400 this ticket fixes, and substituting a placeholder ("No reason given") to satisfy the
    /// validator would reintroduce the unexplained ban T-317 exists to stop.
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
            let Some(win) = web_sys::window() else {
                return;
            };
            let toasts = crate::toast::use_toasts();
            // `Err` is a prompt the browser refused to show — same silent abort as Cancel.
            let Ok(answer) = win.prompt_with_message("Ban reason (required):") else {
                return;
            };
            let reason = match classify_ban_reason(answer.as_deref()) {
                BanReason::Abort => return,
                BanReason::Reject => {
                    toasts.error("Ban reason is required");
                    return;
                }
                BanReason::Send(reason) => reason,
            };
            ban_busy.set(true);
            let path = format!("/admin/users/{}/ban", uid.get_value());
            let body = serde_json::json!({ "reason": reason });
            leptos::task::spawn_local(async move {
                match crate::client::api_post_ok(store, &path, body).await {
                    Ok(()) => {
                        toasts.success("Personnel banned");
                        banned.set(true);
                        refetch.run(());
                    }
                    // The 400 body says exactly what is wrong; a flat "Ban failed" threw that
                    // away. `api_error_message` is the house helper (event_hub/missions/…).
                    Err(e) => toasts.error(crate::client::api_error_message(&e, "Ban failed")),
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

#[cfg(test)]
mod tests {
    use super::{
        classify_ban_reason, roles_sync_success_message, roles_sync_updated_count, BanReason,
        ADMIN_ROLES_SYNC_PATH,
    };

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
        // `window.prompt` → None. Abort carries no toast: the operator chose not to ban.
        assert_eq!(classify_ban_reason(None), BanReason::Abort);
    }

    #[test]
    fn ok_with_blank_is_refused_before_any_request() {
        // The T-323 break: this branch used to POST `{}`, which T-317 correctly answers with
        // 400 "reason is required". Reject means the client never makes the request at all.
        assert_eq!(classify_ban_reason(Some("")), BanReason::Reject);
    }

    #[test]
    fn whitespace_only_is_refused_too() {
        // The server rejects a whitespace-only reason. Agreeing here keeps the operator from
        // seeing a 400 for input that looked non-empty in the prompt.
        for blank in ["   ", "\t", "\n", " \t\n "] {
            assert_eq!(
                classify_ban_reason(Some(blank)),
                BanReason::Reject,
                "{blank:?}"
            );
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
    }
}
