//! Mortar Calculator (/tools/mortar) — ported from pages/doctrine.tsx `MortarCalculatorPage`.
//! `<AuthGate>` → a self-contained firing-solution form (FP/TGT grid inputs + Calculate button + a
//! tactical map preview with the solution panel).
//!
//! T-159.25: live — inputs are signals and Calculate POSTs `/fire-missions/solve`
//! (useSolveFireMission port); the solution card renders the returned distance/azimuth/elevation/
//! TOF (default render byte-parity preserved: same nodes until a solution lands).
#![allow(dead_code)]
use crate::dto::FireSolution;
use crate::ui::{AuthGate, PageHeader};
use leptos::prelude::*;

// OpsCard cn(base,'glass',className) results, tailwind-merged (deferred Rust tw_merge):
//  · inputs card: className "grid …" → grid beats base `flex` (display), `gap-4` beats `gap-3`.
const CARD_INPUTS: &str = "relative flex-col overflow-hidden rounded-xl p-6 glass grid gap-4 sm:grid-cols-2 lg:grid-cols-4";
//  · solution card: className "absolute …" → absolute beats base `relative` (position).
const CARD_SOLUTION: &str = "flex flex-col gap-3 overflow-hidden rounded-xl p-6 glass absolute right-4 bottom-4 w-72 border-t-2 border-tertiary";
const INPUT_CLASS: &str =
    "mt-1 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm";

/// `Math.round(n).toLocaleString()` — default-locale thousands separators (comma).
fn locale_int(n: f64) -> String {
    let v = n.round() as i64;
    let neg = v < 0;
    let digits = v.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        let rem = digits.len() - i;
        out.push(c);
        if rem > 1 && (rem - 1) % 3 == 0 {
            out.push(',');
        }
    }
    if neg {
        format!("-{out}")
    } else {
        out
    }
}

fn num_input(label: &'static str, sig: RwSignal<f64>) -> impl IntoView {
    view! {
        <label class="text-sm">
            {label}
            <input
                type="number"
                // React reflects the controlled value as an attribute at rest ("1000" etc.) — the
                // frozen V golden pins it; prop:value stays the live binding.
                value=move || sig.get().to_string()
                prop:value=move || sig.get().to_string()
                on:input=move |ev| sig.set(event_target_value(&ev).parse().unwrap_or(0.0))
                class=INPUT_CLASS
            />
        </label>
    }
}

#[component]
pub fn MortarCalculatorPage() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = &store;
    let fp_x = RwSignal::new(1000.0);
    let fp_y = RwSignal::new(2000.0);
    let tgt_x = RwSignal::new(2200.0);
    let tgt_y = RwSignal::new(1800.0);
    let solution = RwSignal::new(None::<FireSolution>);
    let busy = RwSignal::new(false);

    let on_solve = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if busy.get_untracked() {
                return;
            }
            busy.set(true);
            let toasts = crate::toast::use_toasts();
            let body = serde_json::json!({
                "fp_x": fp_x.get_untracked(),
                "fp_y": fp_y.get_untracked(),
                "tgt_x": tgt_x.get_untracked(),
                "tgt_y": tgt_y.get_untracked(),
                "weapon_system": "m252_81mm",
            });
            leptos::task::spawn_local(async move {
                match crate::client::api_post::<FireSolution>(store, "/fire-missions/solve", body)
                    .await
                {
                    Ok(s) => solution.set(Some(s)),
                    Err(_) => toasts.error("Could not compute firing solution"),
                }
                busy.set(false);
            });
        }
    };

    view! {
        <AuthGate>
            <div class="relative flex h-full w-full flex-col overflow-hidden">
                <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
                <div class="relative z-10 flex h-full w-full flex-col gap-4 bg-surface-glass p-6 backdrop-blur-xl md:p-8">
                    <PageHeader
                        title="Mortar Calculator"
                        subtitle="Enter grid coordinates for M252 81mm solution."
                    />
                    <div class=CARD_INPUTS>
                        {num_input("FP X", fp_x)} {num_input("FP Y", fp_y)}
                        {num_input("TGT X", tgt_x)} {num_input("TGT Y", tgt_y)}
                    </div>
                    <button
                        type="button"
                        on:click=on_solve
                        prop:disabled=move || busy.get()
                        class="self-start rounded-lg bg-primary px-4 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
                    >
                        {move || if busy.get() { "Computing…" } else { "Calculate Solution" }}
                    </button>
                    <div class="relative min-h-0 flex-1 overflow-hidden rounded-xl border border-border-subtle bg-surface-container-lowest">
                        <div
                            class="absolute inset-0 opacity-30"
                            style="background-image: linear-gradient(rgba(59, 130, 246, 0.08) 1px, transparent 1px), linear-gradient(90deg, rgba(59, 130, 246, 0.08) 1px, transparent 1px); background-size: 40px 40px;"
                        ></div>
                        <div
                            class="absolute top-1/4 left-1/3 h-4 w-4 rounded-full border-2 border-success bg-success/30"
                            title="Fire Position"
                        ></div>
                        <div
                            class="absolute top-1/2 left-2/3 h-4 w-4 rounded-full border-2 border-error bg-error/30"
                            title="Target"
                        ></div>
                        <div class=CARD_SOLUTION>
                            <h2 class="text-sm font-semibold text-primary">
                                "Firing Solution — "
                                {move || {
                                    solution
                                        .get()
                                        .map(|s| s.weapon_system)
                                        .unwrap_or_else(|| "M252 81mm".into())
                                }}
                            </h2>
                            {move || match solution.get() {
                                Some(s) => {
                                    view! {
                                        <dl class="mt-3 space-y-2 font-mono text-sm">
                                            <div class="flex justify-between">
                                                <dt class="text-on-surface-variant">"Distance"</dt>
                                                <dd>{locale_int(s.distance_m)} " m"</dd>
                                            </div>
                                            <div class="flex justify-between">
                                                <dt class="text-on-surface-variant">"Azimuth"</dt>
                                                <dd>{format!("{:.1}°", s.azimuth_deg)}</dd>
                                            </div>
                                            <div class="flex justify-between">
                                                <dt class="text-on-surface-variant">"Elevation"</dt>
                                                <dd class="text-primary">
                                                    {s.elevation_mils}
                                                    " mils"
                                                </dd>
                                            </div>
                                            <div class="flex justify-between">
                                                <dt class="text-on-surface-variant">"TOF"</dt>
                                                <dd>{format!("{:.1} s", s.time_of_flight_s)}</dd>
                                            </div>
                                        </dl>
                                    }
                                        .into_any()
                                }
                                None => {
                                    view! {
                                        <p class="mt-3 text-xs text-on-surface-variant">
                                            "Enter coordinates and calculate to see solution."
                                        </p>
                                    }
                                        .into_any()
                                }
                            }}
                        </div>
                    </div>
                </div>
            </div>
        </AuthGate>
    }
}
