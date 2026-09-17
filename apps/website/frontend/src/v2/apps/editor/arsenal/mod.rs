//! Arsenal tab for editing one slot's persisted loadout.
//!
//! The catalog presents weapon and wear rows, compatible attachments, a doll preview,
//! cargo, weight, and validation. The domain rules live in [`rules`]; loadout JSON and
//! import/export checks live in [`loadout`].
//!
//! Each pick and cargo edit calls [`loadout_commands::set_loadout`] immediately, so a
//! successful write creates one undo step. The current write state is repeated inside
//! the modal because its backdrop obscures the mission title's unsaved indicator.
//! `loadout_commands::set_loadout` begins at `loadout_commands.rs:19`; its accepted-write
//! history tail is at `loadout_commands.rs:23`. A refused write is shown separately so
//! the tab never reports a local pick as saved when the document rejected it.
#![allow(dead_code)]

// Flat registry rows → the Factions palette tree. Pure data, no web-sys: ungated so its unit
// tests run on the native `cargo test` shell.
pub mod asset_catalog;
// The 3D paper-doll mount over the engine's doll renderer — wasm-only, like every other live
// engine host.
#[cfg(target_arch = "wasm32")]
pub mod doll;
// The pure loadout core (serialization, export/import gates, buffer verbs, receipts).
// Re-exported below so every `crate::v2::apps::editor::arsenal::X` path stays one segment deep.
pub mod loadout;
// The Arsenal's writes to the mission document: one slot's loadout, the buffer applied across a
// selection, and the strip. Reaches the live document, so wasm32-only.
#[cfg(target_arch = "wasm32")]
pub mod loadout_commands;
// The Smart-Arsenal domain core: the loadout rows, the compatibility graph, per-row option
// building, validation, the doll region model and the weight readout. Pure and native-tested.
pub mod rules;

// The whole public loadout surface re-exports, used-or-not: `crate::v2::apps::editor::arsenal::X` is the
// documented path (operations/cargo.rs cites `arsenal::buffer_draw` / `arsenal::stripped_loadout`
// by that name), and a bin crate lints a re-export nothing consumes yet as unused.
#[allow(unused_imports)]
pub use loadout::{
    apply_receipt, buffer_draw, buffer_refusals, commit_one_write, commit_writes, copy_receipt,
    loadout_to_picks, picks_to_export, picks_to_loadout, plan_apply, plan_remove, refusal_line,
    remove_receipt, stripped_loadout, try_export, try_import, BufferedLoadout, ImportedLoadout,
    LoadoutWrite,
};

use loadout::{
    attachments_of, export_modpack_id, import_summary, kit_default_items, loadout_faults,
    slot_asset_id,
};
use std::collections::HashMap;

use leptos::prelude::*;

use crate::v2::apps::editor::arsenal::rules::{
    format_loadout_weight, index_by_name, loadout_weight, row_options, CompatFeed,
};
use crate::v2::apps::editor::ui::arsenal::panels::{cargo_panel, compat_panel, doll_view};
use crate::v2::core::api::dto::RegistryItem;
use website_map_engine::editing::hosted_commands as engine_ops;

const CONTROL: &str = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";

/// Explains the immediate commit and undo contract to the author.
const PERSIST_ALWAYS: &str = "Every pick and cargo edit here is written to the mission document the moment you make it — the Arsenal has no Save button by design, and Ctrl+Z undoes one pick.";

/// The half of the persistence line that reads the live `mission_history` dirty flag: the mission
/// itself has nothing waiting for the server. Paired with [`PERSIST_UNSAVED`].
const PERSIST_CLEAN: &str = "The mission has no unsaved changes.";

/// The dirty half: the doc holds work no server version carries yet. This is the same state the top
/// strip's `•` reports — which this modal's backdrop is busy blurring, hence the repeat here.
const PERSIST_UNSAVED: &str =
    "The mission has unsaved changes — Save Version publishes them to the server.";

/// Explains that the last pick was rejected because its entity no longer exists.
/// This state takes precedence over the mission-wide dirty indicator.
const PERSIST_REFUSED: &str = "That last pick did NOT reach the mission document — this entity is no longer in the mission (deleted, or undone away while the Arsenal was open). Close this panel and re-open the Arsenal on a live entity; nothing you pick here now will be kept.";

/// Does the live mission document hold work the server has not seen?
///
/// `mission_history` is `cfg(target_arch = "wasm32")` (it drives the hosted doc), so the native view
/// shell answers `false`: there is no editor mounted there and therefore nothing unsaved. The read
/// itself is `try_get_untracked`, so the persistence line below tracks a local commit counter to
/// re-run — the modal scrim means an Arsenal commit is the only edit that can happen while this is
/// on screen.
fn mission_has_unsaved_work() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        crate::v2::apps::editor::bridge::document_host::history::is_dirty()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

mod tab_content;

/// Reactive state shared by the catalog view and its action handlers.
#[derive(Clone, Copy)]
struct ArsenalTabState {
    id: StoredValue<String>,
    asset_id: StoredValue<Option<String>>,
    picks: RwSignal<HashMap<String, String>>,
    cargo: RwSignal<Vec<rules::CargoRow>>,
    cargo_present: RwSignal<bool>,
    active_key: RwSignal<String>,
    commits: RwSignal<u32>,
    persist_refused: RwSignal<bool>,
    import_status: RwSignal<String>,
    import_refusals: RwSignal<Vec<String>>,
    buffer_status: RwSignal<String>,
    buffer_refusals: RwSignal<Vec<String>>,
    buffer_epoch: RwSignal<u32>,
    doll_unavailable: RwSignal<bool>,
    filter: RwSignal<String>,
    compat: RwSignal<CompatFeed>,
}

/// Edits a slot loadout using the current registry and compatibility feed.
#[component]
pub fn ArsenalTab(
    slot_id: String,
    /// The slot's current `loadout` JSON (from `engine_ops::read_loadout`).
    loadout_json: Option<String>,
    /// The flat registry gear rows, `None` while loading.
    registry: RwSignal<Option<Vec<RegistryItem>>>,
    /// The compat edge feed (optic/magazine rows + validation).
    compat: RwSignal<CompatFeed>,
) -> impl IntoView {
    // Seed character cargo only when the slot loadout has no cargo key.
    #[cfg(target_arch = "wasm32")]
    let loadout_json = engine_ops::seed_slot_cargo(&slot_id).or(loadout_json);
    // The slot prefab identifies its catalogued default cargo throughout this modal.
    let asset_id = StoredValue::new(slot_asset_id(&slot_id));
    let id = StoredValue::new(slot_id);
    // Reactive picks so the doll, weight, validation, and dependent edge rows all re-render live.
    let picks = RwSignal::new(loadout_to_picks(loadout_json.as_deref()));
    // Cargo rows + whether the loadout carries the `cargo` key (the "user state" marker —
    // absent means a later seed may still fire, so persists stay key-less until touched).
    let (cargo0, cargo_present0) = rules::cargo_from_loadout(loadout_json.as_deref());
    let cargo = RwSignal::new(cargo0);
    let cargo_present = RwSignal::new(cargo_present0);
    // The rail/doll active region (highlighted row + hotspot). Default to the primary weapon.
    let active_key = RwSignal::new("primary".to_string());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (id, cargo_present);

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
                Some(items) => tab_content::loaded_catalog(items, state).into_any(),
            }}
        </div>
    }
}

/// Small check glyph for the current pick row.
#[component]
pub(crate) fn MaterialCheck() -> impl IntoView {
    view! { <span class="material-symbols-outlined shrink-0 text-[16px]">"check"</span> }
}

/// Rail tooltip title per region.
pub(crate) fn region_title(key: &str) -> &'static str {
    rules::LOADOUT_ROWS
        .iter()
        .find(|r| r.key == key)
        .map_or("", |r| r.label)
}

/// Rail icon per region (Material Symbols approximations of the screen-04 glyphs).
fn region_icon(key: &str) -> &'static str {
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
