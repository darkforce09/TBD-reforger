//! The native/wasm ballistics agreement bench served at `/debug/ballistics-agreement`.
//!
//! **Role:** reads the public ballistics catalog list and one catalog version's document, draws
//! the seeded agreement cases over that catalog, solves every case with the one fire-mission
//! assembler in the browser's wasm build, and writes the whole reading as JSON into
//! `<pre data-ballistics-agreement>`: per case its id, the solved inputs, the solution (every
//! `f64` value), the bit pattern of every `f64`, and the lead gun's recommended ring and time
//! of flight.
//! **Position:** a routed workspace under `v2::apps::debug`, mounted by `app_routes.rs`. The pure
//! half ([`agreement_report`], [`bench_query`]) builds the reading and parses the URL on every
//! target; the browser half (`live`) fetches and schedules the solves. The native agreement gate
//! `gate ballistics-agreement` of `tools_v2/developer-tools/` reads the `<pre>` over the
//! DevTools protocol and compares it with its own native solves.
//! **Signals & state:** three Leptos `RwSignal`s — the bench state, the status line and the
//! reading's JSON — written by the browser half and read by the view.
//! **Invariants:** every parameter of a run is in the URL (`?seed=&count=&catalog=&version=`),
//! so a reading reproduces exactly; the reads never carry credentials; the bench never writes a
//! mission document and never persists. `data-ballistics-agreement-state` is `loading` until the
//! reading is complete, then `ready` (the `<pre>` holds the whole reading) or `failed` (the
//! status line holds the cause and the `<pre>` stays empty).

use leptos::prelude::*;

pub mod agreement_report;
pub mod bench_query;
#[cfg(target_arch = "wasm32")]
mod live;

/// Seed of the case lattice when the URL names none.
pub const DEFAULT_SEED: u64 = 1;
/// Number of cases when the URL names none.
pub const DEFAULT_COUNT: usize = 32;
/// Most cases one reading solves.
pub const MAX_COUNT: usize = 512;

/// Where the bench is in its one run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchState {
    /// Reading the catalog or solving the cases.
    Loading,
    /// The reading is complete and written into the `<pre>`.
    Ready,
    /// The run stopped; the status line holds the cause.
    Failed,
}

impl BenchState {
    /// The value of the `data-ballistics-agreement-state` attribute.
    pub fn as_attribute(self) -> &'static str {
        match self {
            Self::Loading => "loading",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }
}

/// The bench's view: the state attribute, the status line and the reading.
#[component]
pub fn BallisticsAgreementPage() -> impl IntoView {
    let state = RwSignal::new(BenchState::Loading);
    let status = RwSignal::new(String::from("booting…"));
    let reading = RwSignal::new(String::new());
    #[cfg(target_arch = "wasm32")]
    live::run(live::Signals {
        state,
        status,
        reading,
    });
    view! {
        <div
            class="h-full w-full overflow-auto bg-surface-container-lowest p-3 text-xs text-on-surface"
            data-ballistics-agreement-state=move || state.get().as_attribute()
        >
            <div class="font-semibold">"ballistics agreement bench"</div>
            <div data-ballistics-agreement-status class="text-on-surface-variant">
                {move || status.get()}
            </div>
            <pre data-ballistics-agreement class="mt-2 whitespace-pre-wrap break-all font-mono text-[10px]">
                {move || reading.get()}
            </pre>
        </div>
    }
}
