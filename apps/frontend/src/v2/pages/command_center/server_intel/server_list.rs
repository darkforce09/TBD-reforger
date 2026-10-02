//! The server picker and the frosted shell the chosen server is drawn in.
//!
//! **Role:** picks which server the page reports on, draws the map backdrop behind it, and
//! composes the chosen server's three sections — connect header, telemetry grid, and the
//! intelligence strip at the foot.
//! **Position:** the whole scrolling body of the server intel route.
//! **Signals & state:** reads the live status signal the page owns. The row's own cached status
//! and the connect address are parked in stored values so the render closures can read them
//! without re-parsing the payload.
//! **Invariants:** the live signal wins over the cached row status, and the cached status is
//! only a fallback until the first frame arrives. A row with no status at all is normal — a
//! server that has never reported — while a status the data-transfer type cannot read is a wire
//! breach and is audited.
#![allow(dead_code)]

use super::direct_connect::panel_header;
use super::player_census::telemetry_grid;
use crate::v2::core::api::dto::ServerStatusDto;
use leptos::prelude::*;
use serde_json::Value;

/// Backdrop image behind the frosted panel.
const COMMAND_MAP_IMAGE: &str = "https://lh3.googleusercontent.com/aida-public/AB6AXuBqY9NRsaLKSRk7V0g9XrVkysuxuTRsc8FcMfq76JZujkDPkAAihMyRIw6mOuvFI4tTOwRDvDEhOe-p2Coym8zpmONJeueKLL379Yzecw64o3wzqJMRZdGCA7iBbwrno1hge-AU7AZNCE4XVo9q6IXTH5A2NRf3IToSchzAuj5JUT-Y81VVXfb-Ic4CrnLbV_So9xy2vBIxVHrwDztZ-YuY78DL-Jb5qsgNACRmxHXgRYRrsCxsCJnHBrgj-DD3LUVa31rIo4Arzrc";

/// The string at `k`, or an empty string when the key is absent or is not a string.
pub(super) fn v_str<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or_default()
}

/// The server the page reports on: the first active row, or the first row of any kind.
///
/// Returns `None` only for an empty list. The chosen row is cloned so the caller owns it.
pub(super) fn pick_default(servers: &[Value]) -> Option<Value> {
    servers
        .iter()
        .find(|s| v_bool(s, "is_active"))
        .or_else(|| servers.first())
        .cloned()
}

/// The page body: the map backdrop, and over it either the chosen server or the empty state.
pub(super) fn panel(
    server: Option<Value>,
    live_sig: RwSignal<Option<ServerStatusDto>>,
) -> impl IntoView {
    view! {
        <div class="relative h-full overflow-y-auto">
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
                    Some(s) => server_panel(s, live_sig).into_any(),
                }}
            </div>
        </div>
    }
}

/// One server's panel: the connect header, the telemetry grid, and the intelligence strip.
///
/// Reads `name`, `ip`, `port`, `status`, `required_modpack` and `terrain` from the server row.
/// The status accessor it builds prefers the live stream frame and falls back to the row's own
/// cached status.
pub(super) fn server_panel(s: Value, live_sig: RwSignal<Option<ServerStatusDto>>) -> impl IntoView {
    let name = v_str(&s, "name").to_string();
    let ip = v_str(&s, "ip").to_string();
    let port = v_i64(&s, "port");
    let connect_address = format!("{ip} : {port}");
    let copy_text = StoredValue::new(format!("{ip}:{port}"));
    // An absent status and an unreadable one are different events and are treated differently.
    // A row with no telemetry yet is entirely expected — warning on it would cry wolf on every
    // render of a healthy box. A status that is present but cannot be read is a wire contract
    // breach, and goes through the same deduplicated audit channel as a rejected stream frame.
    let row_status: Option<ServerStatusDto> = match s.get("status") {
        None | Some(Value::Null) => None,
        Some(v) => match serde_json::from_value::<ServerStatusDto>(v.clone()) {
            Ok(dto) => Some(dto),
            Err(e) => {
                let payload: String = v
                    .to_string()
                    .chars()
                    .take(crate::v2::core::api::dto::PAYLOAD_AUDIT_CHARS)
                    .collect();
                crate::v2::core::api::dto::audit_rejected_frame(
                    "server_intel cached row status",
                    &e.to_string(),
                    &payload,
                );
                None
            }
        },
    };
    let row_status = StoredValue::new(row_status);
    let live = move || live_sig.get().or_else(|| row_status.get_value());
    let modpack = s.get("required_modpack").cloned().filter(|m| !m.is_null());
    // A theatre is only named when the join actually produced one. An absent, null or empty key
    // means there is no live match, and inventing a placeholder label for that state is what
    // made the readout meaningless before.
    let terrain_name = {
        let t = v_str(&s, "terrain");
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    };
    let theater_alt = terrain_name
        .as_ref()
        .map(|n| format!("{n} terrain"))
        .unwrap_or_else(|| "Theater of operations".into());

    view! {
        <div class="flex w-full flex-col overflow-hidden bg-surface-glass backdrop-blur-xl">
            {panel_header(name, connect_address, copy_text, live)}
            {telemetry_grid(terrain_name, theater_alt, modpack, live)}

            <div class="bg-surface/20 p-8">
                <span class="mb-4 block text-label-sm uppercase tracking-widest text-on-surface-variant">
                    "Recent Intelligence"
                </span>
                <div class="space-y-3">
                    <div class="flex items-center gap-4 border-b border-white/5 py-2 text-code-md">
                        <span class="shrink-0 text-primary">"[14:02:00Z]"</span>
                        <span class="text-on-surface">
                            "New hostile movement detected in Sector 4"
                        </span>
                    </div>
                    <div class="flex items-center gap-4 border-b border-white/5 py-2 text-code-md">
                        <span class="shrink-0 text-primary">"[13:45:12Z]"</span>
                        <span class="text-on-surface">"Server Uplink maintenance completed"</span>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// The integer at `k`, or zero when the key is absent or is not an integer.
fn v_i64(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(|x| x.as_i64()).unwrap_or_default()
}

/// The boolean at `k`, or `false` when the key is absent or is not a boolean.
pub(super) fn v_bool(v: &Value, k: &str) -> bool {
    v.get(k).and_then(|x| x.as_bool()).unwrap_or_default()
}
