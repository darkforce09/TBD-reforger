//! Server Control (/admin/server) — T-270.
//!
//! Live `GET /servers` list (typed [`ServerRowDto`]) + selectable master/detail. Header Restart and
//! the RCON console / mapped Quick Actions POST `/admin/servers/{id}/rcon` with the live
//! `action` enum (`restart` | `change_map` | `kick` | `custom`).
//!
//! # T-598 — the transport is no longer pending
//!
//! T-269 filed an endpoint that answered `202 {accepted:true}` over a command nothing carried and
//! replaced it with an honest `503`; this page's success copy said "audit queued; transport
//! pending T-269" because that was true. **T-289 shipped the host control agent and T-595 shipped
//! the API client**, so a 202 is now a delivery the host confirmed by re-reading the unit — and
//! the old copy became wrong in the opposite direction. The 202 body grew `delivered` / `state` /
//! `detail` (`api/src/handlers/admin.rs:824`), and this page must *read* them: a toast that says
//! "delivered" without consulting `delivered` is the same defect T-269 found, wearing a different
//! hat. See [`rcon_accepted_message`] and [`rcon_reports_success`].
//!
//! Stop has no HTTP/RCON route → disabled with honest copy. Launch has no start endpoint → same
//! client-side honesty as Server Intel ("requires the Reforger client"). Swap Modpack / Global
//! Broadcast have no matching RCON action → disabled.
#![allow(dead_code)]
use crate::dto::{DataEnvelope, ModpackDto, ServerRowDto, ServerStatusDto};
use crate::ui::{cn, AdminGate, MaterialIcon};
use leptos::prelude::*;
use serde::Deserialize;
use serde_json::{json, Value};

/// 202 body from `POST /admin/servers/{id}/rcon` (`handlers/admin.rs::send_rcon`).
///
/// Mirrors the serializer at `apps/website/api/src/handlers/admin.rs:824-831` —
/// `{action, accepted, delivered, state, detail, audited}`. `audited` is not modelled because
/// nothing on this page renders it; the other five are the claim.
///
/// # Why every field is required
///
/// No `#[serde(default)]`, deliberately, mirroring the reasoning on the API's own
/// [`AgentReply`](../../api/src/services/game_agent.rs): a body missing `delivered` must **fail
/// the parse**, not default to `false`-that-reads-as-`true` or to an empty `state` this page
/// would then format into a sentence nobody told it. `api_post` turns a deserialize failure into
/// `Err((0, None))` (`client.rs:225`), so the page reports a failed RCON request instead of
/// inventing an outcome — fail closed, same as the API does at its own edge.
#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RconAccepted {
    /// The unit was re-read in the state the action intended. On its own this is **not**
    /// permission to report success — see [`rcon_reports_success`].
    accepted: bool,
    action: String,
    /// The host agent took the command and ran the verb. **True even when `accepted` is false**
    /// (the verb ran, the unit did not get where it was told): the API keeps those apart because
    /// collapsing them sends an operator hunting a network fault over a unit fault.
    delivered: bool,
    /// systemd `ActiveState` re-read **after** the action: `active` | `inactive` | `failed` |
    /// `activating` | `deactivating` | `reloading` | `unknown`. This is the point of T-289's
    /// design — `systemctl restart` exits 0 over a dead Reforger server
    /// (`docs/mod/STAGING-SERVER.md:246-250`), so the agent never trusts an exit status and
    /// reports what it actually observed. Throwing it away here would throw away the only
    /// evidence behind the word "delivered".
    state: String,
    /// The agent's human-readable note about that observation.
    detail: String,
}

/// Path for the admin RCON route — must track `app.rs`.
fn admin_server_rcon_path(server_id: &str) -> String {
    format!("/admin/servers/{server_id}/rcon")
}

fn rcon_body_restart() -> Value {
    json!({ "action": "restart" })
}

fn rcon_body_change_map(map: &str) -> Value {
    json!({ "action": "change_map", "map": map })
}

fn rcon_body_custom(command: &str) -> Value {
    json!({ "action": "custom", "command": command })
}

fn rcon_body_kick() -> Value {
    json!({ "action": "kick" })
}

/// What to do with a `window.prompt` answer for a required non-empty field (map name, etc.).
#[derive(Debug, PartialEq)]
enum PromptField {
    Abort,
    Reject,
    Send(String),
}

fn classify_prompt_field(answer: Option<&str>) -> PromptField {
    match answer {
        None => PromptField::Abort,
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                PromptField::Reject
            } else {
                PromptField::Send(trimmed.to_string())
            }
        }
    }
}

fn format_uptime(seconds: i64) -> String {
    let d = seconds / 86_400;
    let h = (seconds % 86_400) / 3600;
    let m = (seconds % 3600) / 60;
    if d > 0 {
        format!("{d}d {h:02}h {m:02}m")
    } else {
        format!("{h:02}h {m:02}m")
    }
}

fn format_endpoint(ip: &str, port: i64) -> String {
    format!("{ip}:{port}")
}

fn status_label(online: bool) -> &'static str {
    if online {
        "online"
    } else {
        "offline"
    }
}

fn status_meta(status: &str) -> (&'static str, &'static str, bool) {
    match status {
        "online" => ("bg-success", "Online", true),
        "starting" => ("bg-tactical-yellow", "Starting", true),
        _ => ("bg-outline", "Offline", false),
    }
}

fn modpack_label(mp: Option<&ModpackDto>) -> String {
    match mp {
        Some(m) => format!("{} v{}", m.modpack.name, m.modpack.version),
        None => "—".to_string(),
    }
}

fn pick_default_id(servers: &[ServerRowDto]) -> Option<String> {
    servers
        .iter()
        .find(|s| s.is_active)
        .or_else(|| servers.first())
        .map(|s| s.id.clone())
}

/// Does this 202 body actually report a delivered, confirmed command?
///
/// Both fields, never one. `accepted` alone is the agent's verdict about the *unit* and
/// `delivered` alone is about the *channel*; success is the conjunction, and reading either in
/// isolation is how a green toast ends up over a command that never landed. A 2xx status is not
/// consulted at all — the status got us into this arm, it is not evidence about the host.
fn rcon_reports_success(resp: &RconAccepted) -> bool {
    resp.delivered && resp.accepted
}

/// Console / toast line after a 202 — says only what the host agent reported observing.
///
/// # T-598: the same string, wrong in the opposite direction
///
/// This read `"RCON accepted action={} (audit queued; transport pending T-269)"`, which was true
/// while `send_rcon` could not deliver anything. T-289 built the host agent and T-595 wired the
/// API to it, so a 202 now means the command was carried to a host agent that re-read the unit
/// and reported [`RconAccepted::state`]. "Audit queued, transport pending" is now an
/// understatement in exactly the way "accepted" was once an overstatement — both are the page
/// describing an outcome it did not look at.
///
/// # Why four arms and not one sentence
///
/// The message must not be able to lie in a third direction, so every combination of the two
/// booleans gets its own wording and every arm interpolates the observed `state`:
///
/// | `delivered` | `accepted` | what the operator is told |
/// |---|---|---|
/// | true | true | delivered, and the host re-read the unit as `state` |
/// | true | false | delivered, and the unit did **not** reach the expected state |
/// | false | false | **not** delivered — nothing reached the host |
/// | false | true | **not** delivered, over a body that contradicts itself → treat as failed |
///
/// The last row is not reachable through today's serializer: `admin.rs` only returns 202 when
/// `delivery.accepted`, which is set solely on `AgentResult::Accepted`, which also sets
/// `delivered: true`. It is written out anyway because it is precisely the shape this page must
/// never render as success, and "the server currently cannot send it" is a property of the
/// server, not of this function. The word *accepted* never appears unqualified in either
/// `delivered == false` arm.
///
/// The 409 (delivered-not-accepted) and 503 (not delivered) answers reach the operator through
/// the `Err` arm of [`post_rcon`] today. This function still handles both, because a client that
/// only decodes the body shape its server happens to emit is trusting a status code to stand in
/// for a payload it never read.
fn rcon_accepted_message(resp: &RconAccepted) -> String {
    let (action, state, detail) = (&resp.action, &resp.state, &resp.detail);
    match (resp.delivered, resp.accepted) {
        (true, true) => format!(
            "RCON delivered action={action} — host agent re-read the unit as state={state} ({detail})"
        ),
        (true, false) => format!(
            "RCON delivered action={action} but the unit did NOT reach the expected state \
             (state={state}; {detail})"
        ),
        (false, true) => format!(
            "RCON NOT delivered action={action} — host reported acceptance of a command it never \
             carried; treat as FAILED (state={state}; {detail})"
        ),
        (false, false) => format!(
            "RCON NOT delivered action={action} — nothing reached the host agent \
             (state={state}; {detail})"
        ),
    }
}

#[component]
pub fn ServerControlPage() -> impl IntoView {
    view! {
        <AdminGate>
            <ServerControlInner />
        </AdminGate>
    }
}

#[component]
fn ServerControlInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let servers = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<DataEnvelope<ServerRowDto>>(store, "/servers")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<DataEnvelope<ServerRowDto>>
        }
    });

    view! {
        <div class="relative h-full w-full overflow-hidden">
            <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
            <div class="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
                <Suspense fallback=move || {
                    view! {
                        <p class="px-8 py-10 text-on-surface-variant">"Loading servers…"</p>
                    }
                }>
                    {move || {
                        servers.get().map(|opt| match opt {
                            Some(env) => control_board(env.data).into_any(),
                            None => {
                                view! {
                                    <p class="px-8 py-10 text-error">"Failed to load servers."</p>
                                }
                                    .into_any()
                            }
                        })
                    }}
                </Suspense>
            </div>
        </div>
    }
}

fn control_board(list: Vec<ServerRowDto>) -> impl IntoView {
    let selected_id = RwSignal::new(pick_default_id(&list).unwrap_or_default());
    let console_log = RwSignal::new(Vec::<String>::new());
    let busy = RwSignal::new(false);
    let command = RwSignal::new(String::new());
    let list_master = list.clone();
    let list_detail = list;

    view! {
        <crate::split_pane::SplitPane
            transparent=true
            master_width="17rem"
            master_header=master_header(list_master.len()).into_any()
            master=view! {
                {move || {
                    server_list(&list_master, selected_id)
                }}
            }
                .into_any()
            detail=view! {
                {move || {
                    let id = selected_id.get();
                    let Some(s) = list_detail.iter().find(|s| s.id == id) else {
                        return view! {
                            <p class="px-8 py-10 text-on-surface-variant">
                                {if list_detail.is_empty() {
                                    "No servers configured."
                                } else {
                                    "No server selected."
                                }}
                            </p>
                        }
                            .into_any();
                    };
                    server_detail(
                        s.clone(),
                        console_log,
                        busy,
                        command,
                    )
                        .into_any()
                }}
            }
                .into_any()
        />
    }
}

fn master_header(count: usize) -> impl IntoView {
    view! {
        <h1 class="w-full text-label-md font-semibold tracking-wide text-on-surface uppercase">
            "Servers"
            <span class="ml-2 font-mono text-code-md text-outline">{count as i64}</span>
        </h1>
    }
}

fn server_list(servers: &[ServerRowDto], selected_id: RwSignal<String>) -> impl IntoView {
    servers
        .iter()
        .cloned()
        .map(|s| {
            let id = s.id.clone();
            let id_click = id.clone();
            let name = s.name.clone();
            let online = s.status.as_ref().is_some_and(|st| st.is_online);
            let status = status_label(online);
            view! {
                {move || {
                    let (dot, label, pulse) = status_meta(status);
                    let active = selected_id.get() == id;
                    let btn = cn(&[
                        "flex items-center gap-3 rounded-lg border-l-4 px-3 py-3 text-left transition-all duration-200",
                        if active {
                            "border-primary bg-primary/15"
                        } else {
                            "border-transparent hover:bg-white/[0.03]"
                        },
                    ]);
                    let ping = cn(&[
                        "absolute inline-flex h-full w-full animate-ping rounded-full opacity-60",
                        dot,
                    ]);
                    let solid = cn(&["relative inline-flex size-2.5 rounded-full", dot]);
                    let name_class = if active {
                        "block truncate font-medium text-on-surface"
                    } else {
                        "block truncate font-medium text-on-surface-variant"
                    };
                    let id_click = id_click.clone();
                    let name = name.clone();
                    view! {
                        <button
                            type="button"
                            class=btn
                            on:click=move |_| selected_id.set(id_click.clone())
                        >
                            <span class="relative flex size-2.5 shrink-0">
                                {pulse.then(|| view! { <span class=ping.clone()></span> })}
                                <span class=solid></span>
                            </span>
                            <span class="min-w-0 flex-1">
                                <span class=name_class>{name.clone()}</span>
                                <span class="block font-mono text-code-md text-outline">{label}</span>
                            </span>
                        </button>
                    }
                }}
            }
        })
        .collect_view()
}

fn append_log(console_log: RwSignal<Vec<String>>, line: impl Into<String>) {
    console_log.update(|lines| {
        lines.push(line.into());
    });
}

fn post_rcon(
    store: crate::auth::AuthStore,
    server_id: String,
    body: Value,
    echo: String,
    console_log: RwSignal<Vec<String>>,
    busy: RwSignal<bool>,
    toasts: crate::toast::Toasts,
) {
    if busy.get_untracked() || server_id.is_empty() {
        return;
    }
    busy.set(true);
    append_log(console_log, echo);
    #[cfg(target_arch = "wasm32")]
    {
        leptos::task::spawn_local(async move {
            let path = admin_server_rcon_path(&server_id);
            match crate::client::api_post::<RconAccepted>(store, &path, body).await {
                Ok(resp) => {
                    let msg = rcon_accepted_message(&resp);
                    // T-598 — a 2xx is not a delivery. The console colours `RCON:` green
                    // (`rcon_console`), so routing a `delivered:false` body down this branch
                    // would paint a success over a command the host never carried. Branch on
                    // the body, not on the status that got us here.
                    if rcon_reports_success(&resp) {
                        append_log(console_log, format!("RCON: {msg}"));
                        toasts.success(msg);
                    } else {
                        append_log(console_log, format!("RCON error: {msg}"));
                        toasts.error(msg);
                    }
                }
                Err(e) => {
                    let msg = crate::client::api_error_message(&e, "RCON request failed");
                    append_log(console_log, format!("RCON error: {msg}"));
                    toasts.error(msg);
                }
            }
            busy.set(false);
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (store, body, console_log, toasts);
        busy.set(false);
    }
}

fn server_detail(
    s: ServerRowDto,
    console_log: RwSignal<Vec<String>>,
    busy: RwSignal<bool>,
    command: RwSignal<String>,
) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let toasts = crate::toast::use_toasts();
    let server_id = s.id.clone();
    let name = s.name.clone();
    let endpoint = format_endpoint(&s.ip, s.port);
    let status = s.status.clone();
    let mod_label = modpack_label(s.required_modpack.as_ref());

    let (players, max_players, uptime, fps) = match &status {
        Some(st) => (
            st.player_count,
            st.max_players,
            format_uptime(st.uptime_seconds),
            format!("{:.1} Hz", st.server_fps),
        ),
        None => (0, 0, "—".to_string(), "—".to_string()),
    };
    // Terrain / active mission are not on `ServerRowDto` / `GET /servers` — show honest em dash
    // rather than inventing mock Everon / mission titles (T-270; sibling of T-359 terrain note).
    let terrain = "—".to_string();
    let mission = status
        .as_ref()
        .and_then(|st: &ServerStatusDto| st.current_match_id.clone())
        .unwrap_or_else(|| "—".to_string());

    let restart_id = server_id.clone();
    let on_restart = {
        let console_log = console_log;
        let busy = busy;
        let toasts = toasts;
        move |_| {
            post_rcon(
                store,
                restart_id.clone(),
                rcon_body_restart(),
                "$ restart".into(),
                console_log,
                busy,
                toasts,
            );
        }
    };

    let on_launch = move |_| {
        #[cfg(target_arch = "wasm32")]
        toasts.message("Launch requires the Reforger client");
        #[cfg(not(target_arch = "wasm32"))]
        let _ = toasts;
    };

    view! {
        <div class="flex h-full min-w-0 flex-1 flex-col">
            <header class="flex flex-wrap items-center justify-between gap-4 border-b border-white/5 p-6 pb-6">
                <div class="min-w-0">
                    <h2 class="truncate text-headline-lg text-on-surface">{name}</h2>
                    <div class="mt-2 inline-flex items-center gap-2 rounded-full bg-white/5 px-3 py-1">
                        <MaterialIcon name="lan" class="text-[16px] text-on-surface-variant" />
                        <span class="font-mono text-code-md text-on-surface">{endpoint}</span>
                    </div>
                </div>
                <div class="flex items-center gap-2">
                    <button
                        type="button"
                        data-testid="server-control-restart"
                        prop:disabled=move || busy.get()
                        class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2.5 text-label-md text-on-surface transition hover:bg-white/5 disabled:opacity-50"
                        on:click=on_restart
                    >
                        <MaterialIcon name="restart_alt" class="text-[18px]" />
                        "Restart"
                    </button>
                    <button
                        type="button"
                        data-testid="server-control-stop"
                        disabled=true
                        title="No Stop HTTP or RCON endpoint — process stop is not wired"
                        class="flex items-center gap-1.5 rounded-full border border-error-alert/30 px-4 py-2.5 text-label-md text-error-alert opacity-50"
                    >
                        <MaterialIcon name="stop" class="text-[18px]" />
                        "Stop"
                    </button>
                    <button
                        type="button"
                        data-testid="server-control-launch"
                        class="flex items-center gap-2 rounded-full bg-action px-6 py-2.5 text-label-md font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
                        on:click=on_launch
                    >
                        <MaterialIcon name="rocket_launch" class="text-[18px]" />
                        "LAUNCH & CONNECT"
                    </button>
                </div>
            </header>
            <div class="grid shrink-0 grid-cols-3 divide-x divide-white/10 border-b border-white/5">
                {telemetry_col(
                    "Active Personnel",
                    &format!("{players} / {max_players}"),
                    "Uptime",
                    &uptime,
                )}
                {telemetry_col("Terrain", &terrain, "Active Mission", &mission)}
                {telemetry_col("Server FPS", &fps, "Mod Configuration", &mod_label)}
            </div>
            {rcon_console(
                server_id,
                console_log,
                busy,
                command,
            )}
        </div>
    }
}

fn telemetry_col(
    primary_label: &str,
    primary_value: &str,
    secondary_label: &str,
    secondary_value: &str,
) -> impl IntoView {
    let (pl, pv, sl, sv) = (
        primary_label.to_string(),
        primary_value.to_string(),
        secondary_label.to_string(),
        secondary_value.to_string(),
    );
    view! {
        <div class="px-6 py-6">
            <p class="font-mono text-code-md tracking-wider text-on-surface-variant/70 uppercase">
                {pl}
            </p>
            <p class="mt-1 truncate font-mono text-3xl font-bold tracking-tight text-on-surface">
                {pv}
            </p>
            <p class="mt-4 font-mono text-code-md tracking-wider text-on-surface-variant/70 uppercase">
                {sl}
            </p>
            <p class="mt-1 truncate text-label-md text-on-surface">{sv}</p>
        </div>
    }
}

fn fire_custom_command(
    store: crate::auth::AuthStore,
    server_id: String,
    command: RwSignal<String>,
    console_log: RwSignal<Vec<String>>,
    busy: RwSignal<bool>,
    toasts: crate::toast::Toasts,
) {
    let cmd = command.get_untracked();
    let trimmed = cmd.trim().to_string();
    if trimmed.is_empty() {
        toasts.error("Enter an RCON command");
        return;
    }
    command.set(String::new());
    post_rcon(
        store,
        server_id,
        rcon_body_custom(&trimmed),
        format!("$ {trimmed}"),
        console_log,
        busy,
        toasts,
    );
}

fn rcon_console(
    server_id: String,
    console_log: RwSignal<Vec<String>>,
    busy: RwSignal<bool>,
    command: RwSignal<String>,
) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let toasts = crate::toast::use_toasts();

    let change_map = {
        let server_id = server_id.clone();
        let console_log = console_log;
        let busy = busy;
        let toasts = toasts;
        move |_| {
            #[cfg(target_arch = "wasm32")]
            {
                let Some(win) = web_sys::window() else {
                    return;
                };
                let Ok(answer) = win.prompt_with_message("Map name (required for change_map):")
                else {
                    return;
                };
                match classify_prompt_field(answer.as_deref()) {
                    PromptField::Abort => {}
                    PromptField::Reject => {
                        toasts.error("Map name required");
                    }
                    PromptField::Send(map) => {
                        post_rcon(
                            store,
                            server_id.clone(),
                            rcon_body_change_map(&map),
                            format!("$ change_map {map}"),
                            console_log,
                            busy,
                            toasts,
                        );
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (&server_id, console_log, busy, toasts, store);
            }
        }
    };

    let force_restart = {
        let server_id = server_id.clone();
        let console_log = console_log;
        let busy = busy;
        let toasts = toasts;
        move |_| {
            post_rcon(
                store,
                server_id.clone(),
                rcon_body_restart(),
                "$ restart".into(),
                console_log,
                busy,
                toasts,
            );
        }
    };

    let send_id = server_id.clone();
    let send_click = {
        let console_log = console_log;
        let busy = busy;
        let command = command;
        let toasts = toasts;
        move |_| {
            fire_custom_command(store, send_id.clone(), command, console_log, busy, toasts);
        }
    };
    let enter_id = server_id;
    let send_enter = {
        let console_log = console_log;
        let busy = busy;
        let command = command;
        let toasts = toasts;
        move |ev: web_sys::KeyboardEvent| {
            if ev.key() == "Enter" {
                fire_custom_command(store, enter_id.clone(), command, console_log, busy, toasts);
            }
        }
    };

    view! {
        <section class="flex min-h-0 flex-1 flex-col bg-surface/40">
            <div class="flex flex-wrap items-center gap-3 border-b border-white/5 bg-surface-container/30 p-4">
                <span class="text-label-sm tracking-wider text-on-surface-variant uppercase">
                    "Quick Actions:"
                </span>
                <button
                    type="button"
                    data-testid="server-control-qa-change-map"
                    prop:disabled=move || busy.get()
                    class="flex items-center gap-1.5 rounded-full border border-white/10 bg-white/5 px-4 py-2 text-label-sm text-on-surface backdrop-blur-md transition hover:bg-white/10 disabled:opacity-50"
                    on:click=change_map
                >
                    <MaterialIcon name="map" class="text-[16px] text-on-surface-variant" />
                    "Change Map"
                </button>
                <button
                    type="button"
                    data-testid="server-control-qa-swap-modpack"
                    disabled=true
                    title="No RCON action for modpack swap (enum is restart|change_map|kick|custom)"
                    class="flex items-center gap-1.5 rounded-full border border-white/10 bg-white/5 px-4 py-2 text-label-sm text-on-surface opacity-50 backdrop-blur-md"
                >
                    <MaterialIcon name="extension" class="text-[16px] text-on-surface-variant" />
                    "Swap Modpack"
                </button>
                <button
                    type="button"
                    data-testid="server-control-qa-broadcast"
                    disabled=true
                    title="No RCON action for global broadcast (enum is restart|change_map|kick|custom)"
                    class="flex items-center gap-1.5 rounded-full border border-white/10 bg-white/5 px-4 py-2 text-label-sm text-on-surface opacity-50 backdrop-blur-md"
                >
                    <MaterialIcon name="campaign" class="text-[16px] text-on-surface-variant" />
                    "Global Broadcast"
                </button>
                <button
                    type="button"
                    data-testid="server-control-qa-force-restart"
                    prop:disabled=move || busy.get()
                    class="flex items-center gap-1.5 rounded-full border border-white/10 bg-white/5 px-4 py-2 text-label-sm text-on-surface backdrop-blur-md transition hover:bg-white/10 disabled:opacity-50"
                    on:click=force_restart
                >
                    <MaterialIcon name="restart_alt" class="text-[16px] text-on-surface-variant" />
                    "Force Restart"
                </button>
            </div>
            <div class="flex min-h-0 flex-1 flex-col p-6">
                <div class="mb-3 flex items-center gap-2">
                    <MaterialIcon name="terminal" class="text-[18px] text-on-surface-variant" />
                    <h3 class="text-label-md font-semibold tracking-wide text-on-surface uppercase">
                        "RCON Console"
                    </h3>
                </div>
                <div
                    class="custom-scrollbar min-h-0 flex-1 overflow-y-auto rounded-xl border border-white/5 bg-black/30 p-4 font-mono text-sm leading-relaxed text-on-surface-variant"
                    data-testid="server-control-console"
                >
                    {move || {
                        let lines = console_log.get();
                        if lines.is_empty() {
                            return view! {
                                <p class="text-on-surface-variant/50">
                                    "No RCON traffic yet. Commands POST to /admin/servers/{id}/rcon. Only restart has a host-agent verb — change_map, kick and custom answer 503."
                                </p>
                            }
                                .into_any();
                        }
                        lines
                            .into_iter()
                            .map(|line| {
                                let c = cn(&[
                                    "whitespace-pre-wrap",
                                    if line.starts_with('$') { "text-primary" } else { "" },
                                    if line.contains("RCON:") { "text-success" } else { "" },
                                    if line.contains("RCON error:") {
                                        "text-error-alert"
                                    } else {
                                        ""
                                    },
                                ]);
                                view! { <p class=c>{line}</p> }
                            })
                            .collect_view()
                            .into_any()
                    }}
                </div>
                <div class="mt-3 flex items-center gap-2 rounded-full border border-white/10 bg-white/5 py-1.5 pr-1.5 pl-5 focus-within:border-primary/40">
                    <span class="font-mono text-sm text-on-surface-variant/60">"$"</span>
                    <input
                        type="text"
                        prop:value=move || command.get()
                        prop:disabled=move || busy.get()
                        placeholder="Send RCON command…"
                        class="flex-1 bg-transparent font-mono text-sm text-on-surface placeholder:text-on-surface-variant/50 outline-none"
                        on:input=move |ev| command.set(event_target_value(&ev))
                        on:keydown=send_enter
                    />
                    <button
                        type="button"
                        data-testid="server-control-rcon-send"
                        aria-label="Send command"
                        prop:disabled=move || busy.get()
                        class="flex size-9 items-center justify-center rounded-full bg-primary text-on-primary transition hover:bg-primary/80 disabled:opacity-50"
                        on:click=send_click
                    >
                        <MaterialIcon name="arrow_upward" class="text-[20px]" />
                    </button>
                </div>
            </div>
        </section>
    }
}

#[cfg(test)]
mod t270 {
    use super::*;

    #[test]
    fn rcon_path_tracks_app_rs() {
        const APP_RS: &str =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../api/src/app.rs"));
        assert!(
            APP_RS.contains(r#""/admin/servers/{id}/rcon""#),
            "app.rs must register POST /admin/servers/{{id}}/rcon"
        );
        assert_eq!(
            admin_server_rcon_path("00000000-0000-4000-d000-000000000001"),
            "/admin/servers/00000000-0000-4000-d000-000000000001/rcon"
        );
    }

    #[test]
    fn rcon_bodies_match_live_action_enum() {
        assert_eq!(rcon_body_restart(), json!({ "action": "restart" }));
        assert_eq!(
            rcon_body_change_map("Everon"),
            json!({ "action": "change_map", "map": "Everon" })
        );
        assert_eq!(
            rcon_body_custom("#restart"),
            json!({ "action": "custom", "command": "#restart" })
        );
        assert_eq!(rcon_body_kick(), json!({ "action": "kick" }));
    }

    /// The shipped half of this page, scrubbed (T-601 — [`crate::arsenal::class_r_scrub`]).
    /// Literals survive: a route path and a `data-testid` are the contract, not a mention of it.
    fn live() -> String {
        crate::arsenal::class_r_scrub::live_source(include_str!("server_control.rs"))
    }

    /// **Cure 2** — these are pure source-shape bans and wiring seams inside `view!` macros, with
    /// no runtime signature to observe. What T-601 changed is *what source they read*: the raw
    /// `include_str!` included this test module and every comment in the file, so a banned string
    /// that survived only in prose would have failed the ban, and a needle that survived only in
    /// prose would have satisfied the positive.
    #[test]
    fn no_mock_servers_or_fabricated_console() {
        // Needles assembled so include_str cannot false-green off this test's own literals.
        // (Belt and braces since T-601: the scrubber cuts the test module out of `live()` anyway.)
        let src = live();
        let mock = format!("{}{}", "MOCK_", "SERVERS");
        let fake_listener = format!("{}{}", "RCON listener bound", " to 0.0.0.0:19999");
        let fake_init = format!("{}{}", "Server initialized on ", "Everon");
        assert!(
            !src.contains(&mock),
            "compile-time mock server table must be gone (perturbation: reintroduce it)"
        );
        assert!(
            !src.contains(&fake_listener) && !src.contains(&fake_init),
            "fabricated console log lines must be gone"
        );
    }

    #[test]
    fn loads_servers_via_typed_api() {
        let src = live();
        let get = format!("{}{}", "api_get", "::<DataEnvelope<ServerRowDto>>");
        assert!(
            src.contains(&get) && src.contains(r#""/servers""#),
            "page must GET /servers as DataEnvelope<ServerRowDto> on a live path"
        );
    }

    #[test]
    fn restart_and_console_post_rcon() {
        let src = live();
        let post = format!("{}{}", "api_post", "::<RconAccepted>");
        assert!(
            src.contains(&post) && src.contains("admin_server_rcon_path"),
            "Restart/console must POST via admin_server_rcon_path + api_post::<RconAccepted>"
        );
        assert!(
            src.contains("rcon_body_restart") && src.contains("rcon_body_custom"),
            "live body helpers must drive Restart and console send"
        );
        // Success toast must name acceptance, not a fake process result.
        let fake_ok = format!("{}{}", "Server restarted", " successfully");
        assert!(
            !src.contains(&fake_ok),
            "must not toast fabricated process success"
        );
        // T-598 Class-R — the toast copy must track the transport that actually exists.
        //
        // This replaces `SRC.contains("transport pending T-269")`, which pinned a claim that
        // T-289 + T-595 made false. It is deliberately **not** another `SRC.contains`: the page
        // now discusses T-269 in its module docs and in this very test file, so any source grep
        // for the old or new wording would go green off prose that no operator ever sees.
        // Calling the live formatter is the only assertion that provably reads the code path the
        // `Ok` arm of `post_rcon` renders.
        let toast = rcon_accepted_message(&accepted_reply());
        assert!(
            !toast.contains("transport pending"),
            "T-289 shipped the host agent and T-595 shipped the client — a 202 toast may no \
             longer call the transport pending. Got: {toast}"
        );
        assert!(
            toast.contains("delivered") && toast.contains("state=active"),
            "a 202 toast must report the delivery and the state the host actually re-read, \
             not a queued-audit placeholder. Got: {toast}"
        );
    }

    #[test]
    fn stop_and_unmapped_quick_actions_are_honestly_disabled() {
        let src = live();
        assert!(
            src.contains(r#"data-testid="server-control-stop""#)
                && src.contains("No Stop HTTP or RCON endpoint"),
            "Stop must be disabled with honest copy (no silent success)"
        );
        assert!(
            src.contains("No RCON action for modpack swap")
                && src.contains("No RCON action for global broadcast"),
            "Swap Modpack / Global Broadcast must be disabled — no matching action enum"
        );
        let mock_stop = format!("{}{}", "Server stopped", " (mock)");
        assert!(
            !src.contains(&mock_stop),
            "Stop must not toast a mock success"
        );
    }

    #[test]
    fn classify_prompt_field_trims_and_rejects_blank() {
        assert_eq!(classify_prompt_field(None), PromptField::Abort);
        assert_eq!(classify_prompt_field(Some("")), PromptField::Reject);
        assert_eq!(classify_prompt_field(Some("   ")), PromptField::Reject);
        assert_eq!(
            classify_prompt_field(Some("  Everon  ")),
            PromptField::Send("Everon".to_string())
        );
    }

    /// The exact 202 body `admin.rs` emits for a restart the host agent confirmed — the same
    /// literal `game_agent.rs::parses_the_agents_exact_replies` pins on the API side.
    fn accepted_reply() -> RconAccepted {
        RconAccepted {
            accepted: true,
            action: "restart".into(),
            delivered: true,
            state: "active".into(),
            detail: "unit active after restart".into(),
        }
    }

    #[test]
    fn rcon_accepted_message_is_honest() {
        let msg = rcon_accepted_message(&accepted_reply());
        assert!(
            msg.contains("delivered") && msg.contains("restart"),
            "got: {msg}"
        );
        // The state is the evidence; a message that omits it is asserting, not reporting.
        assert!(msg.contains("state=active"), "got: {msg}");
        assert!(msg.contains("unit active after restart"), "got: {msg}");
        assert!(!msg.to_lowercase().contains("restarted successfully"));
        assert!(
            !msg.contains("T-269") && !msg.contains("pending"),
            "the transport shipped in T-289/T-595 — this may not still call it pending: {msg}"
        );
    }

    /// T-598 — the three non-success shapes must each read as their own failure.
    ///
    /// The signature defect this guards is a tool reporting success over an input it never
    /// examined. Every assertion below is against the live formatter's output, so a message that
    /// stopped consulting `delivered` / `accepted` fails here rather than shipping a green toast.
    #[test]
    fn rcon_message_never_reads_as_success_without_delivery() {
        // Delivered, but the unit did not get there — T-289's whole reason to exist
        // (`systemctl restart` exits 0 over a dead Reforger server).
        let refused = rcon_accepted_message(&RconAccepted {
            accepted: false,
            action: "restart".into(),
            delivered: true,
            state: "failed".into(),
            detail: "unit is failed after restart; systemctl rc=0".into(),
        });
        assert!(
            refused.contains("NOT reach the expected state") && refused.contains("state=failed"),
            "delivered-but-refused must name the state it did reach: {refused}"
        );
        assert!(
            !refused.contains("re-read the unit as"),
            "must not borrow the confirmed-delivery wording: {refused}"
        );

        // Nothing reached the host.
        let undelivered = rcon_accepted_message(&RconAccepted {
            accepted: false,
            action: "restart".into(),
            delivered: false,
            state: "unknown".into(),
            detail: "unit not installed: not-found".into(),
        });
        assert!(
            undelivered.contains("NOT delivered") && undelivered.contains("state=unknown"),
            "got: {undelivered}"
        );

        // A body that contradicts itself. `admin.rs` cannot emit this today — that is a property
        // of the server, and this page does not get to assume it.
        let contradictory = rcon_accepted_message(&RconAccepted {
            accepted: true,
            action: "restart".into(),
            delivered: false,
            state: "failed".into(),
            detail: "agent said accepted over an undelivered verb".into(),
        });
        assert!(
            contradictory.contains("NOT delivered") && contradictory.contains("FAILED"),
            "`accepted` without `delivered` must never render as success: {contradictory}"
        );

        // And the shared predicate the toast branch keys on agrees with all three.
        for (label, delivered, accepted) in [
            ("delivered-not-accepted", true, false),
            ("not-delivered", false, false),
            ("contradictory", false, true),
        ] {
            let reply = RconAccepted {
                accepted,
                action: "restart".into(),
                delivered,
                state: "failed".into(),
                detail: "x".into(),
            };
            assert!(
                !rcon_reports_success(&reply),
                "{label} must not be reported as a success"
            );
        }
        assert!(rcon_reports_success(&accepted_reply()));
    }

    /// T-598 / T-601 — the `Ok` arm must branch on the body, not on the 2xx that got it there.
    ///
    /// # This is **cure 1**: the pinned code is compiled and run, not greped
    ///
    /// The branch lives inside `#[cfg(target_arch = "wasm32")]` in [`post_rcon`], so the native
    /// test binary cannot link it — which is why every generation of this pin was a source grep.
    /// It was a grep for `"if rcon_reports_success" + "(&resp) {"` plus a count of `toasts.success(`
    /// occurrences, and both are defeatable the same way every Class-R grep in this repo has been:
    /// park the guard in `if false { … }`, `#[cfg(any())]`, or after a `return;` and the needle is
    /// still in the file. The count made it worse, not better — a *second* success toast in dead
    /// code would have failed the count while changing nothing, and moving the only live toast
    /// into dead code would have passed it.
    ///
    /// So the instrument changed rather than the pattern, following `mission_title_prefer`
    /// (T-570). [`harness`] lifts `post_rcon`, `append_log`, `admin_server_rcon_path`,
    /// `rcon_reports_success` and `rcon_accepted_message` **verbatim** out of this file, resolves
    /// the two `target_arch` gates the way the shipped wasm build resolves them
    /// ([`class_r_scrub::resolve_wasm_cfg`], which refuses any gate it cannot decide), compiles the
    /// result against a recording stand-in for the toast host and the API client, and **runs** it
    /// once per 202 body shape. The assertions are on the toasts that were actually raised.
    ///
    /// Dead code raises no toast, so every wrapper — the ten in the handed-down battery and the
    /// ones nobody has invented yet — fails by construction rather than by enumeration. The
    /// shadow-copy attack fails one step earlier: `only_item` refuses two definitions of
    /// `post_rcon` rather than picking one.
    ///
    /// ## What a GREEN here does and does not claim
    ///
    /// * **Does:** for each of the four `(delivered, accepted)` bodies and for a transport error,
    ///   the source as committed raises exactly one toast, on the severity the body warrants, with
    ///   the text the real [`rcon_accepted_message`] produces — and clears `busy` either way.
    /// * **Does not:** say anything about `crate::toast::Toasts` itself, or about the colour the
    ///   console renders `RCON:` in. Those are other modules' pins.
    /// * **Residual:** the harness reads its evidence from the generated program's stdout, so
    ///   production source that printed the sentinels could forge a record. That is the
    ///   irreducible limit of running code you are also judging — it is sabotage, not a wrapper.
    #[test]
    fn ok_arm_toasts_success_only_when_the_body_says_delivered() {
        // (delivered, accepted, expected severity). The last row cannot be emitted by `admin.rs`
        // today; it is exactly the shape this page must never render as success, so it is driven
        // anyway rather than assumed away.
        let lines = harness::run();
        for (delivered, accepted, want) in [
            (true, true, "success"),
            (true, false, "error"),
            (false, false, "error"),
            (false, true, "error"),
        ] {
            let channel = format!("d{delivered}-a{accepted}");
            let toasts = harness::toasts(&lines, &channel);
            assert_eq!(
                toasts.len(),
                1,
                "T-601: post_rcon raised {} toasts for a delivered={delivered} accepted={accepted} \
                 body; exactly one is the contract. 0 means the live branch never ran — a \
                 dead-code wrapper or a deleted call; this pin executes the source, so unreachable \
                 code is invisible to it by design.\n{}",
                toasts.len(),
                lines.join("\n")
            );
            let (severity, text) = toasts[0].split_once('\u{1f}').expect("severity\u{1f}text");
            assert_eq!(
                severity, want,
                "T-601: a delivered={delivered} accepted={accepted} body was toasted as \
                 {severity:?}. A 2xx is not a delivery — the `Ok` arm must branch on the body \
                 (perturbation: `if true {{ … }}`, or gate on the status instead of \
                 rcon_reports_success).\ntoast text was: {text}"
            );
            // The text must be the live formatter's, not a sentence the branch invented.
            let want_text = rcon_accepted_message(&RconAccepted {
                accepted,
                action: harness::ACTION.into(),
                delivered,
                state: harness::STATE.into(),
                detail: harness::DETAIL.into(),
            });
            assert_eq!(
                text, want_text,
                "T-601: the toast did not carry rcon_accepted_message's output, so the operator is \
                 reading a claim this page composed rather than what the host reported"
            );
            // The console echo must agree with the toast; a green console line over a failed
            // command is the same lie in a different widget.
            let console = harness::console(&lines, &channel);
            let want_prefix = if want == "success" {
                "RCON: "
            } else {
                "RCON error: "
            };
            assert!(
                console.iter().any(|l| l.starts_with(want_prefix)),
                "T-601: console log for {channel} never carried a {want_prefix:?} line: {console:?}"
            );
            assert_eq!(
                harness::busy(&lines, &channel),
                Some(false),
                "T-601: post_rcon must clear `busy` on every path or the page locks up"
            );
        }

        // A transport failure must not reach the success branch at all.
        let err = harness::toasts(&lines, "transport-error");
        assert_eq!(err.len(), 1, "{}", lines.join("\n"));
        assert!(
            err[0].starts_with(&format!("error{}", '\u{1f}')),
            "T-601: an Err from the client must toast as an error: {}",
            err[0]
        );
        assert_eq!(harness::busy(&lines, "transport-error"), Some(false));
    }

    /// **Calibration — proof this instrument can still say NO.**
    ///
    /// Takes the same verbatim `post_rcon` and breaks it the ten documented ways plus two the
    /// handed-down list does not contain, then asserts the harness reports each break. If this
    /// ever passes vacuously, the pin above is decoration.
    ///
    /// Each `dead` wrapper must produce **no toast at all** for the success body — that is the
    /// whole point of executing rather than greping: an unreachable call contributes nothing to
    /// the recording no matter how it was made unreachable, so the list below is illustrative
    /// rather than exhaustive by construction.
    #[test]
    fn every_dead_code_wrapper_is_visible_to_this_pin() {
        for (label, wrap) in harness::ATTACKS {
            let lines = harness::run_wrapped(label, wrap);
            for channel in ["dtrue-atrue", "dtrue-afalse", "transport-error"] {
                assert!(
                    harness::toasts(&lines, channel).is_empty(),
                    "T-601: `{label}` still recorded a toast on {channel} — the harness is not \
                     observing execution, and every claim this module makes about dead code is \
                     void.\n{}",
                    lines.join("\n")
                );
            }
        }
        // …and the un-wrapped source still records, or the assertions above are vacuous.
        let lines = harness::run();
        assert!(!harness::toasts(&lines, "dtrue-atrue").is_empty());
    }

    /// **T-601 — the cure-1 harness for [`post_rcon`].**
    ///
    /// Lifts the five items the RCON reply path is made of out of this file **verbatim**, resolves
    /// their `target_arch` gates the way the shipped wasm build resolves them, compiles the result
    /// against a recording stand-in for the toast host and the API client, and runs it.
    ///
    /// Everything in [`PREAMBLE`] is scaffolding; the only code under test is the text spliced in
    /// after it. Deliberate fail-closed edges, following T-570:
    ///
    /// * No compiler → RED. A pin that cannot examine its input must not pass.
    /// * A pinned item that grows a new `crate::…` dependency stops compiling → RED until the
    ///   preamble is extended. Loud and cheap; the alternative is a pin that quietly stops
    ///   covering the thing it names.
    /// * A `cfg` the wasm build's resolution cannot decide → RED
    ///   ([`class_r_scrub::resolve_wasm_cfg`]).
    mod harness {
        use std::path::{Path, PathBuf};
        use std::process::Command;

        const SRC: &str = include_str!("server_control.rs");

        pub(super) const ACTION: &str = "T601-ACTION";
        pub(super) const STATE: &str = "T601-STATE";
        pub(super) const DETAIL: &str = "T601-DETAIL";
        const END: &str = "T601-HARNESS-END";
        /// Field separator in the generated program's stdout — cannot occur in Rust source.
        const US: char = '\u{1f}';

        /// Dead-code wrappers applied to `post_rcon`'s whole body. `@BODY@` is the verbatim body.
        ///
        /// The list is illustrative, not exhaustive, and that is the point of cure 1: an
        /// unreachable call contributes nothing to the recording *however* it was made
        /// unreachable, so a shape nobody has thought of yet fails here too. The ten from the
        /// handed-down battery are listed so a reader can see them go red, plus two the list does
        /// not contain — a constant folded through a comparison, and an `if let` over a pattern
        /// that can never match, which no textual scrubber in this repo folds.
        pub(super) const ATTACKS: &[(&str, &str)] = &[
            ("if true == false", "if true == false { @CALL@ }"),
            ("loop { break; … }", "loop { break; @CALL@ }"),
            ("#[cfg(any())]", "#[cfg(any())] { @CALL@ }"),
            ("while false", "while false { @CALL@ }"),
            ("if !true", "if !true { @CALL@ }"),
            ("if 1 > 2", "if 1 > 2 { @CALL@ }"),
            (
                "if std::hint::black_box(false)",
                "if std::hint::black_box(false) { @CALL@ }",
            ),
            (
                "const C: bool = false; if C",
                "const C: bool = false; if C { @CALL@ }",
            ),
            (
                "return; above",
                "return; #[allow(unreachable_code)] { @CALL@ }",
            ),
            (
                "match () { _ if false => … }",
                "match () { _ if false => { @CALL@ } _ => {} }",
            ),
            // ── two the handed-down list does not contain
            (
                "const NEVER: bool = 1 > 2 (folded through a comparison)",
                "const NEVER: bool = 1 > 2; if NEVER { @CALL@ }",
            ),
            (
                "if let over a pattern that cannot match",
                "if let Some(()) = Option::<()>::None { @CALL@ }",
            ),
        ];

        /// Mock toast host + API client + the crate items the pinned bodies reach for.
        const PREAMBLE: &str = r###"// GENERATED by website-frontend `server_control::t270::harness`.
// The items below the preamble are copied VERBATIM out of `server_control.rs` — do not edit.
#![allow(dead_code, unused_variables, unused_mut, unreachable_code, clippy::all)]

use std::cell::RefCell;

pub type ApiErr = (u16, Option<String>);

/// The 202 body, cut down to the five fields this page reads.
struct RconAccepted {
    accepted: bool,
    action: String,
    delivered: bool,
    state: String,
    detail: String,
}

/// `serde_json::Value` stands in as an opaque token: `post_rcon` only forwards it.
#[derive(Clone)]
struct Value;

/// Leptos signals are `Copy` handles onto shared state; this is the same shape with none of the
/// reactivity, which the pinned code does not use.
struct RwSignal<T: 'static>(&'static RefCell<T>);
impl<T: 'static> Clone for RwSignal<T> {
    fn clone(&self) -> Self {
        RwSignal(self.0)
    }
}
impl<T: 'static> Copy for RwSignal<T> {}
impl<T: 'static + Clone> RwSignal<T> {
    fn new(v: T) -> Self {
        RwSignal(Box::leak(Box::new(RefCell::new(v))))
    }
    fn get_untracked(&self) -> T {
        self.0.borrow().clone()
    }
    fn set(&self, v: T) {
        *self.0.borrow_mut() = v;
    }
    fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut self.0.borrow_mut());
    }
}

thread_local! {
    static SCRIPT: RefCell<Option<Result<RconAccepted, ApiErr>>> = RefCell::new(None);
    static CHANNEL: RefCell<String> = RefCell::new(String::new());
}

fn emit(kind: &str, payload: &str) {
    CHANNEL.with(|c| println!("{}\u{1f}{}\u{1f}{}", c.borrow(), kind, payload));
}

mod auth {
    #[derive(Clone)]
    pub struct AuthStore;
}

/// Recording stand-in: it reports which toast the pinned source ACTUALLY raised, on the path that
/// actually ran.
mod toast {
    #[derive(Clone)]
    pub struct Toasts;
    impl Toasts {
        pub fn success(&self, msg: String) {
            crate::emit("toast", &format!("success\u{1f}{}", msg));
        }
        pub fn error(&self, msg: String) {
            crate::emit("toast", &format!("error\u{1f}{}", msg));
        }
    }
}

mod client {
    pub trait Scripted: Sized {
        fn scripted() -> Result<Self, crate::ApiErr>;
    }
    impl Scripted for crate::RconAccepted {
        fn scripted() -> Result<Self, crate::ApiErr> {
            crate::SCRIPT
                .with(|s| s.borrow_mut().take())
                .expect("the harness arms exactly one scripted reply per drive")
        }
    }
    pub async fn api_post<T: Scripted>(
        _store: crate::auth::AuthStore,
        _path: &str,
        _body: crate::Value,
    ) -> Result<T, crate::ApiErr> {
        T::scripted()
    }
    pub fn api_error_message(e: &crate::ApiErr, fallback: &str) -> String {
        match &e.1 {
            Some(m) => m.clone(),
            None => fallback.to_string(),
        }
    }
}

/// The pinned code hands its work to `spawn_local`; nothing it awaits is real I/O, so one poll is
/// the whole future. A `Pending` here means the pinned path grew a real await the harness does not
/// model — RED, never a silent pass.
fn block_on<F: core::future::Future<Output = ()>>(fut: F) {
    use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    unsafe fn np(_: *const ()) {}
    unsafe fn cl(p: *const ()) -> RawWaker {
        RawWaker::new(p, &VT)
    }
    static VT: RawWakerVTable = RawWakerVTable::new(cl, np, np, np);
    let w = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VT)) };
    let mut cx = Context::from_waker(&w);
    let mut fut = Box::pin(fut);
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(()) => {}
        Poll::Pending => panic!("T601: the pinned RCON path parked on I/O the harness cannot model"),
    }
}

mod leptos {
    pub mod task {
        pub fn spawn_local<F: core::future::Future<Output = ()> + 'static>(f: F) {
            crate::block_on(f)
        }
    }
}

fn drive(channel: &str, script: Result<RconAccepted, ApiErr>) {
    CHANNEL.with(|c| *c.borrow_mut() = channel.to_string());
    SCRIPT.with(|s| *s.borrow_mut() = Some(script));
    let console: RwSignal<Vec<String>> = RwSignal::new(Vec::new());
    let busy: RwSignal<bool> = RwSignal::new(false);
    post_rcon(
        auth::AuthStore,
        "T601-SERVER".to_string(),
        Value,
        "T601-ECHO".to_string(),
        console,
        busy,
        toast::Toasts,
    );
    for line in console.get_untracked() {
        emit("console", &line);
    }
    emit("busy", if busy.get_untracked() { "true" } else { "false" });
}

fn reply(delivered: bool, accepted: bool) -> RconAccepted {
    RconAccepted {
        accepted,
        action: "@ACTION@".to_string(),
        delivered,
        state: "@STATE@".to_string(),
        detail: "@DETAIL@".to_string(),
    }
}

"###;

        fn item(sig: &str) -> String {
            let prod = crate::arsenal::class_r_scrub::live_source(SRC);
            let raw = crate::arsenal::class_r_scrub::only_item(&prod, sig);
            crate::arsenal::class_r_scrub::resolve_wasm_cfg(raw)
        }

        /// The verbatim `post_rcon`, renamed, plus the `post_rcon` the driver actually calls: a
        /// shim whose only statement is one call to the real thing, optionally parked inside
        /// `wrapper` (`@CALL@` = that call).
        ///
        /// The attack is applied to the **call site**, which is what every wave-77 decoy did to
        /// `apply_row_meta` / `set_loadout` — and it keeps the borrow checker out of the way. The
        /// loop-shaped wrappers (`loop { break; … }`, `while false { … }`) make rustc analyse the
        /// body as if it could run twice, and `post_rcon` moves five of its seven parameters; the
        /// closure clones them per call, so borrowck has nothing to say and the only variable
        /// under test stays *reachability*.
        const SHIM: &str = r###"
fn post_rcon(
    store: auth::AuthStore,
    server_id: String,
    body: Value,
    echo: String,
    console_log: RwSignal<Vec<String>>,
    busy: RwSignal<bool>,
    toasts: toast::Toasts,
) {
    let call = move || {
        post_rcon_live(
            store.clone(),
            server_id.clone(),
            body.clone(),
            echo.clone(),
            console_log,
            busy,
            toasts.clone(),
        )
    };
    @WRAPPED@
}
"###;

        fn program(wrapper: Option<&str>) -> String {
            let mut src = PREAMBLE
                .replace("@ACTION@", ACTION)
                .replace("@STATE@", STATE)
                .replace("@DETAIL@", DETAIL);
            for sig in [
                "fn admin_server_rcon_path(",
                "fn rcon_reports_success(",
                "fn rcon_accepted_message(",
                "fn append_log(",
            ] {
                src.push_str(&item(sig));
                src.push_str("\n\n");
            }
            const POST_SIG: &str = "fn post_rcon(";
            let post = item(POST_SIG);
            assert!(
                post.starts_with(POST_SIG),
                "T-601: post_rcon extraction misaligned"
            );
            src.push_str(&format!("fn post_rcon_live({}", &post[POST_SIG.len()..]));
            src.push_str("\n\n");
            src.push_str(&SHIM.replace(
                "@WRAPPED@",
                &wrapper.unwrap_or("@CALL@").replace("@CALL@", "call();"),
            ));
            src.push_str("\n\nfn main() {\n");
            for (d, a) in [(true, true), (true, false), (false, false), (false, true)] {
                src.push_str(&format!("    drive(\"d{d}-a{a}\", Ok(reply({d}, {a})));\n"));
            }
            src.push_str(
                "    drive(\"transport-error\", Err((503u16, Some(\"host agent unreachable\".to_string()))));\n",
            );
            src.push_str(&format!("    println!(\"{END}\");\n}}\n"));
            src
        }

        /// The compiler that built this test. Cargo exports `CARGO` to every crate it compiles and
        /// `rustc` is its sibling in the same toolchain; PATH is the fallback. There is
        /// deliberately no "skip if absent" branch — a pin that cannot examine its input goes RED.
        fn rustc_bin() -> PathBuf {
            if let Some(cargo) = option_env!("CARGO") {
                let sibling = Path::new(cargo).with_file_name("rustc");
                if sibling.is_file() {
                    return sibling;
                }
            }
            PathBuf::from("rustc")
        }

        fn compile_and_run(tag: &str, source: &str) -> Vec<String> {
            let dir = std::env::current_exe()
                .expect("T-601: test executable path")
                .with_file_name(format!("t601-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir)
                .unwrap_or_else(|e| panic!("T-601: cannot create {}: {e}", dir.display()));
            let rs = dir.join("t601_pin.rs");
            std::fs::write(&rs, source)
                .unwrap_or_else(|e| panic!("T-601: cannot write {}: {e}", rs.display()));
            let bin = dir.join("t601_pin");

            let rustc = rustc_bin();
            let compiled = Command::new(&rustc)
                .args(["--edition", "2021", "--crate-name", "t601_pin"])
                .args(["-C", "debug-assertions=on", "-A", "warnings", "-o"])
                .arg(&bin)
                .arg(&rs)
                .output()
                .unwrap_or_else(|e| {
                    panic!(
                        "T-601: cannot run `{}`: {e}\n\
                         This pin proves the RCON reply path by compiling and running it, so a \
                         missing compiler is a failure to verify, not a pass.",
                        rustc.display()
                    )
                });
            assert!(
                compiled.status.success(),
                "T-601: the extracted server_control items no longer compile against the pin's \
                 mocks. Usually this means a pinned item grew a dependency the preamble does not \
                 model — extend PREAMBLE. Source kept at {}\n\n{}",
                rs.display(),
                String::from_utf8_lossy(&compiled.stderr)
            );

            let ran = Command::new(&bin)
                .output()
                .unwrap_or_else(|e| panic!("T-601: cannot run {}: {e}", bin.display()));
            assert!(
                ran.status.success(),
                "T-601: the extracted RCON path aborted. Source kept at {}\n\nstdout:\n{}\nstderr:\n{}",
                rs.display(),
                String::from_utf8_lossy(&ran.stdout),
                String::from_utf8_lossy(&ran.stderr)
            );
            let stdout = String::from_utf8_lossy(&ran.stdout).into_owned();
            assert_eq!(
                stdout.lines().last(),
                Some(END),
                "T-601: the harness did not run to completion; stdout was:\n{stdout}"
            );
            let _ = std::fs::remove_dir_all(&dir);
            stdout.lines().map(str::to_string).collect()
        }

        pub(super) fn run() -> Vec<String> {
            compile_and_run("live", &program(None))
        }

        pub(super) fn run_wrapped(label: &str, wrapper: &str) -> Vec<String> {
            let tag: String = label
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect();
            compile_and_run(&tag, &program(Some(wrapper)))
        }

        fn rows(lines: &[String], channel: &str, kind: &str) -> Vec<String> {
            let prefix = format!("{channel}{US}{kind}{US}");
            lines
                .iter()
                .filter_map(|l| l.strip_prefix(&prefix).map(str::to_string))
                .collect()
        }

        /// Recorded toasts for one channel, as `severity\u{1f}text`.
        pub(super) fn toasts(lines: &[String], channel: &str) -> Vec<String> {
            rows(lines, channel, "toast")
        }

        pub(super) fn console(lines: &[String], channel: &str) -> Vec<String> {
            rows(lines, channel, "console")
        }

        pub(super) fn busy(lines: &[String], channel: &str) -> Option<bool> {
            rows(lines, channel, "busy").first().map(|v| v == "true")
        }
    }

    #[test]
    fn pick_default_prefers_active() {
        let inactive = ServerRowDto {
            id: "a".into(),
            name: "A".into(),
            ip: "1.1.1.1".into(),
            port: 1,
            required_modpack_id: None,
            is_active: false,
            status: None,
            required_modpack: None,
            terrain: None,
        };
        let mut active = inactive.clone();
        active.id = "b".into();
        active.is_active = true;
        assert_eq!(
            pick_default_id(&[inactive.clone(), active.clone()]).as_deref(),
            Some("b")
        );
        assert_eq!(pick_default_id(&[inactive]).as_deref(), Some("a"));
        assert_eq!(pick_default_id(&[]), None);
    }
}
