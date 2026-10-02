//! Right dock shell behavior.

use super::*;

mod factions_panel;
mod layout;

use factions_panel::{factions_panel, FactionsPanelSignals};
pub use layout::DockRight;

/// the tab strip's own row: the tab group, then the Manage verb + collapse chevron.
pub(super) const TAB_STRIP: &str = "flex shrink-0 items-center justify-between gap-1";
/// the gap between cells inside each group.
pub(super) const TAB_GROUP: &str = "flex items-center gap-0.5";
/// a tab cell, selected. `size-5` (20 px) is the cell budget; the strip's arithmetic is
/// written from it.
pub(super) const TAB_CELL_ON: &str = "flex size-5 shrink-0 items-center justify-center rounded border-b-2 border-primary text-primary";
/// a tab cell at rest.
pub(super) const TAB_CELL_OFF: &str = "flex size-5 shrink-0 items-center justify-center rounded border-b-2 border-transparent text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface";
/// the Manage verb's cell: the same box as a tab, in the primary tint (it is the strip's one
/// verb, not an eighth tab).
pub(super) const TAB_CELL_VERB: &str = "flex size-5 shrink-0 items-center justify-center rounded text-primary transition-colors hover:bg-primary/15";
/// the Favourites|History subtab pair's selected pill (a text pill, not a glyph cell: two
/// words fit inside the tab body where the glyph strip above does not have the room).
pub(super) const SUBTAB_ON: &str =
    "rounded px-2 py-0.5 text-label-sm font-medium text-primary bg-primary/15";
/// the subtab pill at rest.
pub(super) const SUBTAB_OFF: &str = "rounded px-2 py-0.5 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface";
/// how many TAB cells the strip renders (Factions · Vehicles · Zones · Compositions ·
/// Triggers · Favourites · Markers). Stated so the budget test costs a compile error to get wrong,
/// and pinned against the actual `tab_btn` call count in the view.
pub(super) const TAB_COUNT: usize = 7;

/// Returns a glyph for tab `i`. Labels remain available through title and
/// aria-label so compact cells fit the dock width.
#[must_use]
pub(super) fn tab_icon(i: usize) -> &'static str {
    match i {
        0 => "groups",         // Factions — the ORBAT roles palette
        1 => "directions_car", // Vehicles
        2 => "push_pin",       // Markers
        3 => "crop_free",      // Zones — a drawn area
        4 => "dashboard",      // Compositions — prefab clusters
        5 => "bolt",           // Triggers — activation
        6 => "star",           // Favourites and recently placed share a tab.
        _ => "help",
    }
}

/* ══════════  — the Zones panel's selection, reachable from OUTSIDE this component ══════════
 *
 * A zone's selection is deliberately NOT `select_tool`'s: it is `zone_selected`, an `RwSignal` local
 * to [`DockRight`] (see its declaration for why — a zone id in the slot selection reads `SEL 1` with
 * nothing highlighted). That locality is what made 's click-to-select router return `false` for
 * every zone: the router lives in `mission_editor.rs`, holding the `!Send` doc/selection/engine
 * handles, and had no way to reach a signal declared inside this component's body.
 *
 * So the panel EXPOSES its selection the same way the validation panel exposes its router — a
 * thread_local hook registered at mount, read by a free function. This is a SEAM, not a second
 * selection path: the hook only sets the signal this panel already owns (and raises the Zones tab so
 * the selection is visible), and the ONE router is still `route_select_by_subject_id`.
 */

/// The Zones tab's index in the tab strip. Named because THREE places must agree — the tab button,
/// the panel that renders under it, and [`register_select_zone`]'s "show me the selection I just
/// made". A literal in the third place is a silent way for a routed click to select a zone on a tab
/// the author is not looking at.
pub(crate) const ZONES_TAB: usize = 3;

/// The registered zone-selection hook: takes a `zonesById` id and makes it the Zones panel's
/// selection. Set once at [`DockRight`] mount; `None` on the host / pre-mount.
pub(super) type ZoneSelectHook = std::rc::Rc<dyn Fn(&str)>;

thread_local! {
 /// The Zones panel's selection hook. Peer of `validation_panel::SELECT_BY_ID` and
 /// `PAYLOAD_SOURCE`, and thread_local for the same reason: the signal is `!Send` panel state
 /// that no caller can hold.
    static SELECT_ZONE: std::cell::RefCell<Option<ZoneSelectHook>> =
        const { std::cell::RefCell::new(None) };
}

/// Register the current zone-selection hook.
pub(crate) fn register_select_zone(f: ZoneSelectHook) {
    SELECT_ZONE.with(|c| *c.borrow_mut() = Some(f));
}

/// Remove the hook only when it is still the registered instance. This keeps
/// cleanup from an older mount from removing a newer hook.
pub(crate) fn unregister_select_zone(f: &ZoneSelectHook) -> bool {
    let taken = SELECT_ZONE.with(|c| {
        let mut slot = c.borrow_mut();
        if slot
            .as_ref()
            .is_some_and(|live| std::rc::Rc::ptr_eq(live, f))
        {
            slot.take()
        } else {
            None
        }
    });
    taken.is_some()
}

/// Register a zone-selection hook for the reactive owner and remove it on
/// cleanup. Local storage holds the non-Send closure through cleanup.
pub(crate) fn install_select_zone(f: ZoneSelectHook) {
    let mine = StoredValue::new_local(std::rc::Rc::clone(&f));
    register_select_zone(f);
    on_cleanup(move || {
        let _ = mine.try_with_value(unregister_select_zone);
    });
}

/// Select a zone through the mounted panel, returning whether a hook exists.
/// The hook runs outside the cell borrow because it can update signals.
#[must_use]
pub(crate) fn route_select_zone(zone_id: &str) -> bool {
    let hook = SELECT_ZONE.with(|c| c.borrow().clone());
    match hook {
        Some(f) => {
            f(zone_id);
            true
        }
        None => false,
    }
}
