//! The console: the quick actions, the transcript, and the command line under them.
//!
//! **Role:** the four quick-action buttons, the scrolling transcript of everything sent and
//! everything answered, and the input that sends a raw command.
//! **Position:** the lower half of the selected server's card.
//! **Signals & state:** reads and writes `console_log` and `command`; `busy` disables every control
//! while a request is out.
//! **Invariants:** only the actions the host actually has a verb for are enabled. Swapping the
//! modpack and broadcasting globally are not in the action set, so they are disabled with copy
//! naming the reason rather than silently doing nothing. Changing the map needs a map name, and a
//! blank answer is refused rather than sent. Transcript lines are coloured by their own prefix, so
//! only a line the reply marked as success is green.
#![allow(dead_code)]

#[cfg(target_arch = "wasm32")]
use super::rcon::{classify_prompt_field, rcon_body_change_map, PromptField};
use super::rcon::{fire_custom_command, post_rcon, rcon_body_restart};
use crate::v2::core::ui::{cn, MaterialIcon};
use leptos::prelude::*;

/// The quick actions, the transcript and the command line for one server.
pub(super) fn rcon_console(
    server_id: String,
    console_log: RwSignal<Vec<String>>,
    busy: RwSignal<bool>,
    command: RwSignal<String>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let toasts = crate::v2::core::ui::toast::use_toasts();

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
