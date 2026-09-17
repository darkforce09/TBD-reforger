//! The panel header: who the server is, how to reach it, and the launch button.
//!
//! **Role:** renders the online indicator and server name, the address chip with its copy
//! button, and the launch action.
//! **Position:** the top band of the frosted panel, above the telemetry grid.
//! **Signals & state:** reads the status accessor the panel built, so the indicator and its
//! tooltip track the live stream. The address to copy arrives as a stored value.
//! **Invariants:** the copy button reports success only once the clipboard write resolved, so
//! it goes through the one clipboard helper in the crate rather than calling the browser API
//! here. Both actions are browser-only and do nothing in a native build.
#![allow(dead_code)]

use crate::v2::core::api::dto::ServerStatusDto;
use crate::v2::core::ui::{cn, MaterialIcon};
use leptos::prelude::*;

/// The header band for one server.
///
/// `name` and `connect_address` are already formatted for display, `copy_text` holds the
/// address in the form the clipboard should receive, and `live` reports the current status.
pub(super) fn panel_header(
    name: String,
    connect_address: String,
    copy_text: StoredValue<String>,
    live: impl Fn() -> Option<ServerStatusDto> + Copy + Send + Sync + 'static,
) -> impl IntoView {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = copy_text;
    // The clipboard write is a promise that rejects on an insecure context, an unfocused
    // document and a denied permission. The shared helper awaits it and toasts only on the
    // resolve arm; a second clipboard path here is how one of them starts claiming a copy that
    // never happened.
    let copy_address = move |_| {
        #[cfg(target_arch = "wasm32")]
        crate::v2::apps::editor::shell::document_commands::write_clipboard(
            copy_text.get_value(),
            "Server address copied".to_string(),
            crate::v2::core::ui::toast::use_toasts(),
        );
    };
    let launch_stub = move |_| {
        #[cfg(target_arch = "wasm32")]
        crate::v2::core::ui::toast::use_toasts().success("Launch requires the Reforger client");
    };

    view! {
        <div class="flex flex-col justify-between gap-6 border-b border-white/5 bg-surface/40 px-8 py-6 md:flex-row md:items-center">
            <div>
                <div class="mb-2 flex items-center gap-3">
                    <div
                        class=move || {
                            cn(
                                &[
                                    "pulse-dot h-2.5 w-2.5 rounded-full",
                                    if live().map(|l| l.is_online).unwrap_or(false) {
                                        "bg-success"
                                    } else {
                                        "bg-tactical-yellow"
                                    },
                                ],
                            )
                        }
                        title=move || {
                            if live().map(|l| l.is_online).unwrap_or(false) {
                                "Server Online"
                            } else {
                                "Server Offline"
                            }
                        }
                    ></div>
                    <h2 class="text-headline-md uppercase tracking-wider text-on-surface">
                        {name}
                    </h2>
                </div>
                <div class="inline-flex items-center gap-2 rounded-md border border-white/5 bg-surface-container px-3 py-1.5 text-code-md text-on-surface-variant">
                    <MaterialIcon name="dns" class="text-[16px]" />
                    <span>{connect_address}</span>
                    <button
                        type="button"
                        on:click=copy_address
                        aria-label="Copy IP"
                        class="ml-2 transition-colors hover:text-primary"
                    >
                        <MaterialIcon name="content_copy" class="text-[16px]" />
                    </button>
                </div>
            </div>
            <button
                type="button"
                on:click=launch_stub
                class="flex shrink-0 items-center gap-2 rounded-full border border-secondary-container/50 bg-secondary-container px-6 py-3 text-label-md text-on-secondary-container transition-all duration-300 hover:shadow-[0_0_20px_rgba(5,102,217,0.4)]"
            >
                <MaterialIcon name="play_arrow" filled=true />
                "LAUNCH & CONNECT"
            </button>
        </div>
    }
}
