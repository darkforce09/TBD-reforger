//! The mortar calculator route: the catalog, the inputs, the map, the local solve, and the save
//! area.
//!
//! **Role:** owns every signal the page shares — the catalog choice and document, the weapon
//! and shell selection, the terrain, the target and battery drafts, the wind, the burst height
//! and the last solve outcome — loads the catalogs, mounts the map picker, runs the solve bridge
//! on Calculate with the lead gun's crest profile, renders the solution panel, and mounts the
//! save area for a signed-in viewer.
//! **Position:** the public `/tools/mortar` route inside the navigation frame. The catalogs come
//! from the public reads (`catalog_source`); the solve runs on this device (`solve_bridge`); the
//! save area sits behind `MortarSaveSection` — the session gate while the platform answers, a
//! needs-a-connection notice while the catalog comes from the offline copy.
//! **Signals & state:** the signals above; two `LocalResource`s for the catalog list and the
//! chosen document; a local `StoredValue` holding the `TerrainHeights` the mounted map fills;
//! the offline status from `offline_status`. The save area owns the event list, the saved fire
//! missions, the save status and its hydration set.
//! **Invariants:** nothing is solved without a loaded catalog; the selection is reconciled
//! against every catalog that loads; the solve reads every draft once per click, so the answer
//! always matches one consistent set of inputs; the page renders signed out, and no
//! account-scoped request leaves the page before the session admits the save area.

#[cfg(target_arch = "wasm32")]
use super::catalog_source::CatalogOrigin;
#[cfg(target_arch = "wasm32")]
use super::catalog_source::{
    CatalogFailure, CatalogKey, choose_catalog, failure_message, latest_catalog_versions,
    origin_notice,
};
#[cfg(target_arch = "wasm32")]
use super::inputs::INPUT_CLASS;
#[cfg(target_arch = "wasm32")]
use super::inputs::battery::{battery_inputs, default_battery};
#[cfg(target_arch = "wasm32")]
use super::inputs::illumination::illumination_inputs;
#[cfg(target_arch = "wasm32")]
use super::inputs::positions::PositionDraft;
#[cfg(target_arch = "wasm32")]
use super::inputs::positions::{MortarTerrain, position_fields, terrain_select};
#[cfg(target_arch = "wasm32")]
use super::inputs::weapon_and_shell::ArmamentSelection;
#[cfg(target_arch = "wasm32")]
use super::inputs::weapon_and_shell::{reconcile_selection, weapon_and_shell_inputs};
#[cfg(target_arch = "wasm32")]
use super::inputs::wind::WindDraft;
#[cfg(target_arch = "wasm32")]
use super::inputs::wind::wind_inputs;
#[cfg(target_arch = "wasm32")]
use super::map_picker::map_picker;
#[cfg(target_arch = "wasm32")]
use super::map_picker::profile::lead_gun_profile;
#[cfg(target_arch = "wasm32")]
use super::offline_status::offline_pack_line;
#[cfg(target_arch = "wasm32")]
use super::saved_fires::connection_gate::MortarSaveSection;
#[cfg(target_arch = "wasm32")]
use super::saved_fires::save_area::RestoreTargets;
#[cfg(target_arch = "wasm32")]
use super::solution::SolveOutcome;
#[cfg(target_arch = "wasm32")]
use super::solution::solution_panel;
#[cfg(target_arch = "wasm32")]
use super::solve_bridge::{MissionDrafts, solve_mission};
#[cfg(target_arch = "wasm32")]
use ballistics_model::catalog::BallisticsCatalog;
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::ballistics_catalogs::BallisticsCatalogSummary;
#[cfg(target_arch = "wasm32")]
use frontend_map_view::terrain_height::TerrainHeights;
#[cfg(target_arch = "wasm32")]
use frontend_offline::offline_status;
#[cfg(target_arch = "wasm32")]
use frontend_ui::PageHeader;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::sync::Arc;

/// The inputs card's class: a glass panel laid out as a responsive grid.
#[cfg(target_arch = "wasm32")]
const CARD_INPUTS: &str =
    "relative grid gap-4 overflow-hidden rounded-xl p-6 glass sm:grid-cols-2 lg:grid-cols-3";

/// A full-width row inside the inputs grid.
#[cfg(target_arch = "wasm32")]
const FULL_ROW: &str = "sm:col-span-2 lg:col-span-3";

/// The mortar calculator, open to every viewer.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn MortarCalculatorPage() -> impl IntoView {
    let offline = ReadSignal::from(offline_status());
    let catalog_key = RwSignal::new(None::<CatalogKey>);
    let offered = RwSignal::new(Vec::<BallisticsCatalogSummary>::new());
    let catalog = RwSignal::new(None::<Arc<BallisticsCatalog>>);
    let origin = RwSignal::new(None::<CatalogOrigin>);
    let failure = RwSignal::new(None::<CatalogFailure>);
    let selection = RwSignal::new(ArmamentSelection::default());
    let terrain = RwSignal::new(MortarTerrain::Everon);
    let target = RwSignal::new(PositionDraft::default());
    let guns = RwSignal::new(default_battery());
    let wind = RwSignal::new(WindDraft::default());
    let burst_height = RwSignal::new(String::new());
    let outcome = RwSignal::new(None::<SolveOutcome>);
    let heights = StoredValue::new_local(TerrainHeights::new());
    let solved = Signal::derive(move || outcome.get().and_then(Result::ok));
    let restore_into = RestoreTargets {
        target,
        guns,
        selection,
        wind,
        burst_height,
    };

    let list = LocalResource::new(|| async { super::catalog_source::fetch_catalog_list().await });
    Effect::new(move |_| match list.get() {
        None => {}
        Some(Ok(listed)) => {
            let latest = latest_catalog_versions(&listed);
            let chosen = choose_catalog(&latest, catalog_key.get_untracked().as_ref());
            if chosen.is_none() {
                failure.set(Some(CatalogFailure::NoCatalogs));
            }
            offered.set(latest);
            catalog_key.set(chosen);
        }
        Some(Err(reason)) => failure.set(Some(CatalogFailure::ListUnreadable(reason))),
    });

    let document = LocalResource::new(move || {
        let key = catalog_key.get();
        async move {
            let key = key?;
            Some(super::catalog_source::fetch_catalog_document(&key).await)
        }
    });
    Effect::new(move |_| match document.get().flatten() {
        None => {}
        Some(Ok((loaded, read_from))) => {
            selection.update(|s| *s = reconcile_selection(&loaded, s));
            catalog.set(Some(Arc::new(loaded)));
            origin.set(Some(read_from));
            failure.set(None);
        }
        Some(Err(reason)) => {
            catalog.set(None);
            failure.set(Some(CatalogFailure::DocumentUnreadable(reason)));
        }
    });

    let on_solve = move |_| {
        let Some(loaded) = catalog.get_untracked() else {
            return;
        };
        let (selected, target_draft, gun_drafts) = (
            selection.get_untracked(),
            target.get_untracked(),
            guns.get_untracked(),
        );
        let (wind_draft, burst_text) = (wind.get_untracked(), burst_height.get_untracked());
        let on_terrain = terrain.get_untracked();
        let solved = heights.with_value(|h| {
            let height_at = |x: f64, y: f64| h.height_at(x, y);
            let profile = lead_gun_profile(on_terrain, &target_draft, &gun_drafts, height_at);
            let drafts = MissionDrafts {
                selection: &selected,
                terrain: on_terrain,
                target: &target_draft,
                guns: &gun_drafts,
                wind: &wind_draft,
                burst_height: &burst_text,
                crest_profile: profile.as_ref(),
            };
            solve_mission(&loaded, drafts, height_at)
        });
        outcome.set(Some(solved));
    };

    view! {
        <div class="relative flex h-full w-full flex-col overflow-y-auto">
            <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
            <div class="relative z-10 flex w-full flex-col gap-4 bg-surface-glass p-6 backdrop-blur-xl md:p-8">
                <PageHeader
                    title="Mortar Calculator"
                    subtitle="Pick a weapon and shell, place the guns and the target, and solve on this device."
                />
                {offline_pack_line(offline.into())}
                {catalog_status(offered, catalog_key, origin, failure, offline)}
                <div class=CARD_INPUTS data-mortar-inputs="">
                    {weapon_and_shell_inputs(catalog, selection)}
                    {terrain_select(terrain)}
                    <div class=FULL_ROW>
                        {position_fields("Target", target.into(), move |d| target.set(d), terrain)}
                    </div>
                    <div class=FULL_ROW>{battery_inputs(guns, terrain)}</div>
                    {wind_inputs(wind)}
                    {illumination_inputs(catalog, selection, burst_height)}
                </div>
                {map_picker(terrain, target, guns, solved, heights)}
                <button
                    type="button"
                    on:click=on_solve
                    prop:disabled=move || catalog.with(Option::is_none)
                    class="self-start rounded-lg bg-primary px-4 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
                >
                    "Calculate Solution"
                </button>
                {solution_panel(outcome)}
                <MortarSaveSection origin=origin restore_into=restore_into outcome=outcome />
            </div>
        </div>
    }
}

/// The catalog line: the picker when several catalogs are published, the offline notice, or
/// the failure with the offline pack's explanation.
#[cfg(target_arch = "wasm32")]
fn catalog_status(
    offered: RwSignal<Vec<BallisticsCatalogSummary>>,
    catalog_key: RwSignal<Option<CatalogKey>>,
    origin: RwSignal<Option<CatalogOrigin>>,
    failure: RwSignal<Option<CatalogFailure>>,
    offline: ReadSignal<frontend_offline::OfflineStatus>,
) -> impl IntoView {
    let picker = move || {
        (offered.with(Vec::len) > 1).then(|| {
            view! {
                <label class="text-sm">
                    "Catalog"
                    <select
                        prop:value=move || {
                            catalog_key.with(|k| k.as_ref().map(|k| k.catalog_id.clone()).unwrap_or_default())
                        }
                        on:change=move |ev| {
                            let id = event_target_value(&ev);
                            let key = offered
                                .with(|all| all.iter().find(|s| s.catalog_id == id.as_str()).map(CatalogKey::of_summary));
                            catalog_key.set(key);
                        }
                        class=INPUT_CLASS
                        data-mortar-input="catalog"
                    >
                        {offered
                            .get()
                            .into_iter()
                            .map(|s| {
                                let label = format!("{} (v{}, game {})", s.title, s.catalog_version, s.game_build);
                                view! { <option value=s.catalog_id.to_string()>{label}</option> }
                            })
                            .collect_view()}
                    </select>
                </label>
            }
        })
    };
    view! {
        <div class="flex flex-col gap-2" data-mortar-catalog="">
            {picker}
            {move || {
                failure
                    .get()
                    .map(|f| view! { <p class="text-sm text-error">{failure_message(&f, offline.get())}</p> })
            }}
            {move || {
                origin
                    .get()
                    .and_then(origin_notice)
                    .map(|text| view! { <p class="text-sm text-tactical-yellow">{text}</p> })
            }}
        </div>
    }
}
