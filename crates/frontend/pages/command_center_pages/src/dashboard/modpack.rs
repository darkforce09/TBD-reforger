//! The modpack card: which mod set the servers expect, and how big it is.
//!
//! **Role:** renders the active modpack's name, version and download size, plus a sync chip.
//! **Position:** the last cell of the dashboard's three-column card grid.
//! **Signals & state:** none — the modpack arrives owned and is read once.
//! **Invariants:** with no active modpack the card keeps its shape and reports the empty state
//! in every slot, including a grey `NONE` chip in place of the green `SYNCED` one.

#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::ModpackDto;
#[cfg(target_arch = "wasm32")]
use frontend_ui::byte_formatting::format_download_size;
#[cfg(target_arch = "wasm32")]
use frontend_ui::{MaterialIcon, cn};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The modpack card for `modpack`, the mod set the servers currently require.
#[cfg(target_arch = "wasm32")]
pub(super) fn modpack_card(modpack: Option<ModpackDto>) -> impl IntoView {
    view! {
        <div class="relative flex flex-col overflow-hidden rounded-xl p-6 glass gap-4">
            <div class="flex items-center justify-between border-b border-border-subtle pb-3">
                <h3 class="flex items-center gap-2 text-label-sm text-on-surface-variant uppercase">
                    <MaterialIcon name="extension" class="text-[18px]" />
                    "Modpack"
                </h3>
            </div>
            <div class="mt-2 flex h-full flex-col justify-between">
                <div>
                    <h4 class="mb-1 text-xl font-bold text-on-surface">
                        {match &modpack {
                            Some(m) => format!("{} v{}", m.modpack.name, m.modpack.version),
                            None => "No modpack".to_string(),
                        }}
                    </h4>
                    <span class="font-mono text-xs text-on-surface-variant">
                        {match &modpack {
                            Some(m) => {
                                format!("SIZE: {}", format_download_size(m.modpack.total_size_bytes))
                            }
                            None => "—".to_string(),
                        }}
                    </span>
                </div>
                <div class="mt-4 flex items-center justify-between rounded-lg border border-border-subtle bg-surface-container-lowest p-3">
                    <span class=cn(
                        &[
                            "flex items-center gap-2 font-mono text-xs font-bold tracking-widest",
                            if modpack.is_some() {
                                "text-success"
                            } else {
                                "text-on-surface-variant"
                            },
                        ],
                    )>
                        <MaterialIcon name="check_circle" filled=true class="text-[14px]" />
                        "STATUS: "
                        {if modpack.is_some() { "SYNCED" } else { "NONE" }}
                    </span>
                    <MaterialIcon name="sync" class="text-[18px] text-on-surface-variant" />
                </div>
            </div>
        </div>
    }
}
