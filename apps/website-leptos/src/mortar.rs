//! Mortar Calculator (/tools/mortar) — ported from pages/doctrine.tsx `MortarCalculatorPage`.
//! `<AuthGate>` → a self-contained firing-solution form (FP/TGT grid inputs + Calculate button + a
//! tactical map preview with the solution panel). No data fetch — the solution comes from the
//! `useSolveFireMission` mutation on submit.
//!
//! **Gate scope (this slice):** the initial render (default coordinates, no solution yet) →
//! "Enter coordinates and calculate to see solution." — byte-exact-verified. The computed firing
//! solution + input reactivity are behavior (a mutation + T-interaction) — a follow-up.
#![allow(dead_code)]
use crate::ui::{AuthGate, PageHeader};
use leptos::prelude::*;

// OpsCard cn(base,'glass',className) results, tailwind-merged (deferred Rust tw_merge):
//  · inputs card: className "grid …" → grid beats base `flex` (display), `gap-4` beats `gap-3`.
const CARD_INPUTS: &str = "relative flex-col overflow-hidden rounded-xl p-6 glass grid gap-4 sm:grid-cols-2 lg:grid-cols-4";
//  · solution card: className "absolute …" → absolute beats base `relative` (position).
const CARD_SOLUTION: &str = "flex flex-col gap-3 overflow-hidden rounded-xl p-6 glass absolute right-4 bottom-4 w-72 border-t-2 border-tertiary";
const INPUT_CLASS: &str =
    "mt-1 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm";

#[component]
pub fn MortarCalculatorPage() -> impl IntoView {
    // solution?.weapon_system ?? 'M252 81mm' — no solution on load. A bound value (not a second
    // literal) so it stays a SEPARATE text node from the "Firing Solution — " literal, matching the
    // React literal+expression split (adjacent Leptos string literals would merge into one node).
    let weapon = "M252 81mm";
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
                        <label class="text-sm">
                            "FP X"
                            <input type="number" value="1000" class=INPUT_CLASS />
                        </label>
                        <label class="text-sm">
                            "FP Y"
                            <input type="number" value="2000" class=INPUT_CLASS />
                        </label>
                        <label class="text-sm">
                            "TGT X"
                            <input type="number" value="2200" class=INPUT_CLASS />
                        </label>
                        <label class="text-sm">
                            "TGT Y"
                            <input type="number" value="1800" class=INPUT_CLASS />
                        </label>
                    </div>
                    <button
                        type="button"
                        class="self-start rounded-lg bg-primary px-4 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
                    >
                        "Calculate Solution"
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
                                {weapon}
                            </h2>
                            <p class="mt-3 text-xs text-on-surface-variant">
                                "Enter coordinates and calculate to see solution."
                            </p>
                        </div>
                    </div>
                </div>
            </div>
        </AuthGate>
    }
}
