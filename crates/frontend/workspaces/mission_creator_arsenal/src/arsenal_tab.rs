//! The Arsenal tab: the component the Attributes dialog mounts for one slot, the reactive state
//! its loaded view shares, and the persistence lines it states.
//!
//! **Role:** turns a slot's `loadout` JSON into reactive picks and cargo, holds every outcome of
//! the tab (commits, refusals, import and buffer receipts, the doll's availability) in
//! `ArsenalTabState`, and renders "Loading catalog…" until the registry arrives, then the loaded
//! view of `tab_content`.
//! **Position:** mounted by the workspace's Attributes dialog; reads the hosted commands for the
//! cargo seed and the bridge's history for the dirty flag; hands its state to the loaded view,
//! whose handlers write through `loadout_commands`.
//! **Signals & state:** one set of `RwSignal`s and `StoredValue`s per mounted tab, dropped with
//! it; nothing here outlives the dialog.
//! **Invariants:** each pick and cargo edit calls `loadout_commands::set_loadout` immediately, so
//! a successful write creates one undo step; `loadout_commands::set_loadout` begins at
//! `loadout_commands.rs:20` and its accepted-write history tail is at `loadout_commands.rs:25`. The
//! tab repeats the mission's unsaved state because the dialog's backdrop hides the top strip's
//! marker, and a refused write is shown on its own, ahead of that state, so a local pick is never
//! reported as saved when the document rejected it.

#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::loadout::{loadout_to_picks, slot_asset_id};
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::RegistryItem;
#[cfg(target_arch = "wasm32")]
use mission_creator_state::arsenal_rules as rules;
#[cfg(target_arch = "wasm32")]
use mission_creator_state::arsenal_rules::CompatFeed;
#[cfg(target_arch = "wasm32")]
use mission_editing_commands::hosted_commands as engine_ops;
#[cfg(target_arch = "wasm32")]
use orbat_slot_ids::SlotUid;

/// Explains the immediate commit and undo contract to the author.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) const PERSIST_ALWAYS: &str = "Every pick and cargo edit here is written to the mission document the moment you make it — the Arsenal has no Save button by design, and Ctrl+Z undoes one pick.";

/// The half of the persistence line that reads the live `mission_history` dirty flag: the mission
/// itself has nothing waiting for the server. Paired with [`PERSIST_UNSAVED`].
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) const PERSIST_CLEAN: &str = "The mission has no unsaved changes.";

/// The dirty half: the doc holds work no server version carries yet. This is the same state the top
/// strip's `•` reports — which this modal's backdrop is busy blurring, hence the repeat here.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) const PERSIST_UNSAVED: &str =
    "The mission has unsaved changes — Save Version publishes them to the server.";

/// Explains that the last pick was rejected because its entity no longer exists.
/// This state takes precedence over the mission-wide dirty indicator.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) const PERSIST_REFUSED: &str = "That last pick did NOT reach the mission document — this entity is no longer in the mission (deleted, or undone away while the Arsenal was open). Close this panel and re-open the Arsenal on a live entity; nothing you pick here now will be kept.";

/// Does the live mission document hold work the server has not seen?
///
/// `mission_history` is `cfg(target_arch = "wasm32")` (it drives the hosted doc), so the native view
/// shell answers `false`: there is no editor mounted there and therefore nothing unsaved. The read
/// itself is `try_get_untracked`, so the persistence line below tracks a local commit counter to
/// re-run — the modal scrim means an Arsenal commit is the only edit that can happen while this is
/// on screen.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn mission_has_unsaved_work() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        mission_creator_engine_bridge::bridge::document_host::history::is_dirty()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// Reactive state shared by the catalog view and its action handlers.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub(crate) struct ArsenalTabState {
    pub(crate) id: StoredValue<SlotUid>,
    pub(crate) asset_id: StoredValue<Option<String>>,
    pub(crate) picks: RwSignal<HashMap<String, String>>,
    pub(crate) cargo: RwSignal<Vec<mission_operations::cargo_rules::CargoRow>>,
    pub(crate) cargo_present: RwSignal<bool>,
    pub(crate) active_key: RwSignal<String>,
    pub(crate) commits: RwSignal<u32>,
    pub(crate) persist_refused: RwSignal<bool>,
    pub(crate) import_status: RwSignal<String>,
    pub(crate) import_refusals: RwSignal<Vec<String>>,
    pub(crate) buffer_status: RwSignal<String>,
    pub(crate) buffer_refusals: RwSignal<Vec<String>>,
    pub(crate) buffer_epoch: RwSignal<u32>,
    pub(crate) doll_unavailable: RwSignal<bool>,
    pub(crate) filter: RwSignal<String>,
    pub(crate) compat: RwSignal<CompatFeed>,
}

/// Edits a slot loadout using the current registry and compatibility feed.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn ArsenalTab(
    /// The slot whose loadout the tab edits.
    slot_id: SlotUid,
    /// The slot's current `loadout` JSON (from `engine_ops::read_loadout`).
    loadout_json: Option<String>,
    /// The flat registry gear rows, `None` while loading.
    registry: RwSignal<Option<Vec<RegistryItem>>>,
    /// The compat edge feed (optic/magazine rows + validation).
    compat: RwSignal<CompatFeed>,
) -> impl IntoView {
    // Seed character cargo only when the slot loadout has no cargo key.
    let loadout_json = engine_ops::seed_slot_cargo(slot_id.clone()).or(loadout_json);
    // The slot prefab identifies its catalogued default cargo throughout this modal.
    let asset_id = StoredValue::new(slot_asset_id(slot_id.as_str()));
    let id = StoredValue::new(slot_id);
    // Reactive picks so the doll, weight, validation, and dependent edge rows all re-render live.
    let picks = RwSignal::new(loadout_to_picks(loadout_json.as_deref()));
    // Cargo rows + whether the loadout carries the `cargo` key (the "user state" marker —
    // absent means a later seed may still fire, so persists stay key-less until touched).
    let (cargo0, cargo_present0) =
        mission_operations::cargo_rules::cargo_from_loadout(loadout_json.as_deref());
    let cargo = RwSignal::new(cargo0);
    let cargo_present = RwSignal::new(cargo_present0);
    // The rail/doll active region (highlighted row + hotspot). Default to the primary weapon.
    let active_key = RwSignal::new("primary".to_string());

    // A commit tick rerenders the line that reads the document dirty flag untracked.
    let commits = RwSignal::new(0u32);
    // A rejected write has its own verdict because mission dirtiness cannot describe it.
    let persist_refused = RwSignal::new(false);
    // Import receipts and refusals render in separate forms.
    let import_status = RwSignal::new(String::new());
    let import_refusals = RwSignal::new(Vec::<String>::new());

    // Buffer receipts and refusals are distinct from import outcomes.
    let buffer_status = RwSignal::new(String::new());
    let buffer_refusals = RwSignal::new(Vec::<String>::new());
    // The buffer itself lives in an `editor_ops` thread_local (it outlives this modal — you copy in
    // one Arsenal and apply from another), so nothing about it is reactive. This counter is what the
    // Apply affordance re-reads it on.
    let buffer_epoch = RwSignal::new(0u32);

    // The doll falls back to SVG when its renderer cannot be created.
    let doll_unavailable = RwSignal::new(false);
    let filter = RwSignal::new(String::new());
    // Switching regions clears the list filter (each region gets a fresh search).
    Effect::new(move |prev: Option<String>| {
        let k = active_key.get();
        if prev.as_deref().is_some_and(|p| p != k) {
            filter.set(String::new());
        }
        k
    });
    let state = ArsenalTabState {
        id,
        asset_id,
        picks,
        cargo,
        cargo_present,
        active_key,
        commits,
        persist_refused,
        import_status,
        import_refusals,
        buffer_status,
        buffer_refusals,
        buffer_epoch,
        doll_unavailable,
        filter,
        compat,
    };
    view! {
        <div class="flex flex-col gap-2">
            {move || match registry.get() {
                None => view! { <p class="text-label-sm normal-case text-outline">"Loading catalog…"</p> }.into_any(),
                Some(items) => crate::tab_content::loaded_catalog(items, state).into_any(),
            }}
        </div>
    }
}

/// Small check glyph for the current pick row.
#[cfg(target_arch = "wasm32")]
#[component]
pub(crate) fn MaterialCheck() -> impl IntoView {
    view! { <span class="material-symbols-outlined shrink-0 text-[16px]">"check"</span> }
}

/// Rail tooltip title per region.
#[cfg(target_arch = "wasm32")]
pub(crate) fn region_title(key: &str) -> &'static str {
    rules::LOADOUT_ROWS
        .iter()
        .find(|r| r.key == key)
        .map_or("", |r| r.label)
}

/// Rail icon per region (Material Symbols approximations of the screen-04 glyphs).
#[cfg(target_arch = "wasm32")]
pub(crate) fn region_icon(key: &str) -> &'static str {
    match key {
        "primary" => "swords",
        "optic" => "filter_center_focus",
        "magazine" => "dataset",
        "launcher" => "rocket_launch",
        "handgun" => "front_hand",
        "throwable" => "bomb",
        "headCover" => "sports_motorsports",
        "jacket" => "apparel",
        "vest" => "shield",
        "armoredVest" => "security",
        "backpack" => "backpack",
        "handwear" => "waving_hand",
        "pants" => "accessibility",
        _ => "footprint", // boots
    }
}

#[cfg(test)]
#[path = "tests/shell_wiring.rs"]
mod shell_wiring_tests;
