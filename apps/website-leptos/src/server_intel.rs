//! Server Intel (/server-intel) — ported from pages/ServerIntel.tsx. `<AuthGate>` → a `/servers`
//! Resource → `QueryState` → the satellite-map backdrop + (a default server's telemetry panel, or
//! the "No servers configured." empty state).
//!
//! **Gate scope (this slice):** the empty-DB `/servers` golden (`{data:[]}`) → `pickDefaultServer`
//! yields no server → the backdrop + empty-state `<p>` render, byte-exact-verified. The populated
//! telemetry panel (header/launch, performance/theater/settings grid, SSE-fed live stats via
//! useServerTelemetry) is content-golden gated — ported when a seeded server golden exists; the
//! server item type stays `serde_json::Value` until then.
#![allow(dead_code)]
use crate::dto::DataEnvelope;
use crate::ui::AuthGate;
use leptos::prelude::*;
use serde_json::Value;

/// Global tactical map backdrop — gives the glass panels something to frost (byte-identical URL).
const COMMAND_MAP_IMAGE: &str = "https://lh3.googleusercontent.com/aida-public/AB6AXuBqY9NRsaLKSRk7V0g9XrVkysuxuTRsc8FcMfq76JZujkDPkAAihMyRIw6mOuvFI4tTOwRDvDEhOe-p2Coym8zpmONJeueKLL379Yzecw64o3wzqJMRZdGCA7iBbwrno1hge-AU7AZNCE4XVo9q6IXTH5A2NRf3IToSchzAuj5JUT-Y81VVXfb-Ic4CrnLbV_So9xy2vBIxVHrwDztZ-YuY78DL-Jb5qsgNACRmxHXgRYRrsCxsCJnHBrgj-DD3LUVa31rIo4Arzrc";

#[component]
pub fn ServerIntelPage() -> impl IntoView {
    view! {
        <AuthGate>
            <ServerIntelInner />
        </AuthGate>
    }
}

#[component]
fn ServerIntelInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let servers = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<DataEnvelope<Value>>(store, "/servers")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<DataEnvelope<Value>>
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                servers
                    .get()
                    .map(|opt| match opt {
                        // pickDefaultServer(list): first entry (real preference rule is content-gated);
                        // empty list → None → empty state.
                        Some(env) => panel(env.data.into_iter().next()).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

fn panel(server: Option<Value>) -> impl IntoView {
    view! {
        <div class="relative h-full overflow-y-auto">
            // Global satellite-map backdrop — the surface the glass panel frosts.
            <div
                class="absolute inset-0 z-0 bg-cover bg-center"
                style=format!("background-image: url(\"{COMMAND_MAP_IMAGE}\");")
            >
                <div class="absolute inset-0 bg-background/80 backdrop-blur-sm"></div>
                <div class="absolute inset-0 bg-gradient-to-t from-background via-transparent to-transparent"></div>
            </div>
            <div class="relative z-10 flex w-full flex-col">
                {match server {
                    None => {
                        view! { <p class="text-on-surface-variant">"No servers configured."</p> }
                            .into_any()
                    }
                    // Full telemetry panel is content-golden gated (empty DB has no servers).
                    Some(_s) => ().into_any(),
                }}
            </div>
        </div>
    }
}
