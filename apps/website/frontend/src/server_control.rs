//! Server Control (/admin/server) — T-270.
//!
//! Live `GET /servers` list (typed [`ServerRowDto`]) + selectable master/detail. Header Restart and
//! the RCON console / mapped Quick Actions POST `/admin/servers/{id}/rcon` with the live
//! `action` enum (`restart` | `change_map` | `kick` | `custom`). Transport is still a no-op until
//! T-269 — the UI shows the 202 `{accepted, action}` response honestly and never fabricates a
//! process success or a fake console log.
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
#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RconAccepted {
    accepted: bool,
    action: String,
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

/// Console / toast line after a 202 — honest about audit-accept, not process success.
fn rcon_accepted_message(resp: &RconAccepted) -> String {
    format!(
        "RCON accepted action={} (audit queued; transport pending T-269)",
        resp.action
    )
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
                    append_log(console_log, format!("RCON: {msg}"));
                    toasts.success(msg);
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
                                    "No RCON traffic yet. Commands POST to /admin/servers/{id}/rcon (audit-only until T-269)."
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

    #[test]
    fn no_mock_servers_or_fabricated_console() {
        // Needles assembled so include_str cannot false-green off this test's own literals.
        const SRC: &str = include_str!("server_control.rs");
        let mock = format!("{}{}", "MOCK_", "SERVERS");
        let fake_listener = format!("{}{}", "RCON listener bound", " to 0.0.0.0:19999");
        let fake_init = format!("{}{}", "Server initialized on ", "Everon");
        assert!(
            !SRC.contains(&mock),
            "compile-time mock server table must be gone (perturbation: reintroduce it)"
        );
        assert!(
            !SRC.contains(&fake_listener) && !SRC.contains(&fake_init),
            "fabricated console log lines must be gone"
        );
    }

    #[test]
    fn loads_servers_via_typed_api() {
        const SRC: &str = include_str!("server_control.rs");
        let get = format!("{}{}", "api_get", "::<DataEnvelope<ServerRowDto>>");
        assert!(
            SRC.contains(&get) && SRC.contains(r#""/servers""#),
            "page must GET /servers as DataEnvelope<ServerRowDto>"
        );
    }

    #[test]
    fn restart_and_console_post_rcon() {
        const SRC: &str = include_str!("server_control.rs");
        let post = format!("{}{}", "api_post", "::<RconAccepted>");
        assert!(
            SRC.contains(&post) && SRC.contains("admin_server_rcon_path"),
            "Restart/console must POST via admin_server_rcon_path + api_post::<RconAccepted>"
        );
        assert!(
            SRC.contains("rcon_body_restart") && SRC.contains("rcon_body_custom"),
            "live body helpers must drive Restart and console send"
        );
        // Success toast must name acceptance, not a fake process result.
        let fake_ok = format!("{}{}", "Server restarted", " successfully");
        assert!(
            !SRC.contains(&fake_ok),
            "must not toast fabricated process success"
        );
        assert!(
            SRC.contains("transport pending T-269"),
            "accepted toast/console must admit transport is still pending"
        );
    }

    #[test]
    fn stop_and_unmapped_quick_actions_are_honestly_disabled() {
        const SRC: &str = include_str!("server_control.rs");
        assert!(
            SRC.contains(r#"data-testid="server-control-stop""#)
                && SRC.contains("No Stop HTTP or RCON endpoint"),
            "Stop must be disabled with honest copy (no silent success)"
        );
        assert!(
            SRC.contains("No RCON action for modpack swap")
                && SRC.contains("No RCON action for global broadcast"),
            "Swap Modpack / Global Broadcast must be disabled — no matching action enum"
        );
        let mock_stop = format!("{}{}", "Server stopped", " (mock)");
        assert!(
            !SRC.contains(&mock_stop),
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

    #[test]
    fn rcon_accepted_message_is_honest() {
        let msg = rcon_accepted_message(&RconAccepted {
            accepted: true,
            action: "restart".into(),
        });
        assert!(msg.contains("accepted") && msg.contains("restart"));
        assert!(msg.contains("T-269"));
        assert!(!msg.to_lowercase().contains("restarted successfully"));
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
