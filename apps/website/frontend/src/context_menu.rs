//! T-664 — the Mission Creator's right-click context menu.
//!
//! A right-click on the 3D viewport opens a floating menu at the event pixel. Eden shows **two
//! takes** of the same gesture (see `.ai/artifacts/eden_screenshots/batch01_context_menu.md`):
//!
//!   * **empty ground** (nothing under the cursor) → a short menu ([`MenuTake::EmptyGround`]);
//!   * **on an entity** → a much longer, entity-specific menu ([`MenuTake::OnEntity`]).
//!
//! **This module owns three things** and nothing wasm:
//!   1. the **item model** — [`ContextItem`] (an *id enum*, not a stringly-typed label) plus
//!      [`MenuEntry`] and the two [`MenuTake`] builders, transcribed verbatim from the Eden batch;
//!   2. **hit-target resolution** — [`resolve_target`], the selection-aware rule that decides which
//!      take opens and what it targets;
//!   3. the **overlay component** + **keyboard dismissal / navigation** — [`ContextMenuOverlay`].
//!
//! **Why an id enum (T-664's forward contract).** This slice directly closes `PLACE-COMMENT-001`'s
//! entry point and **unblocks six later tickets** — `CREW-SEAT-001` (T-076), `CONN-START-001`,
//! `CTX-FORMATION-001`, `ATTR-MULTI-001`, `COMP-SAVE-001`, `KEY-WP-001` (waves 106+). Each of those
//! backs an item that Eden shows but whose feature does not exist yet, so it renders **disabled**
//! with the blocking ticket named in [`MenuEntry::blocked`]. A later ticket flips its item to
//! enabled and adds the behaviour by matching on the **variant** — it never re-parses a label. That
//! is the attachment point the ticket requires: extend [`ContextItem`], not a set of `&str`s.
//!
//! **What "enabled" means here.** An entry is enabled iff its backing feature already ships in the
//! editor (Attributes, camera go-here, copy/paste/delete, arsenal, …). Everything else is disabled
//! with a comment. We do **not** invent behaviour for a disabled item — dispatching one is a no-op
//! ([`dispatch`]). Eden's own disabled-vs-omitted split (it greys `Cut`/`Copy`/`Delete` in `Edit`
//! but *omits* the extra `Select`/`Log` rows when nothing is selected) is reproduced faithfully:
//! the two takes are different item lists, and within the on-entity take the clipboard verbs keep
//! their slots (enabled, since a target exists) while query verbs that need a real feature are
//! disabled-with-ticket rather than dropped.
#![allow(dead_code)]

use leptos::prelude::*;

/// Every action a viewport context-menu row can carry — the **stable id** later tickets extend.
///
/// Submenu parents ([`Select`](ContextItem::Select), [`Edit`](ContextItem::Edit),
/// [`Log`](ContextItem::Log), [`Connect`](ContextItem::Connect), [`Transform`](ContextItem::Transform),
/// [`Grid`](ContextItem::Grid)) are represented as **leaf ids** here: this slice ships the menu, its
/// hit-target logic and dismissal; the nested submenu *contents* are the concern of the tickets that
/// own them (`CONN-START-001` for `Connect`, `CTX-FORMATION-001` for `Transform`, `KEY-WP-001` for
/// waypoints under `Select`). Until then a parent row renders disabled with its blocking ticket, so
/// there is nothing to open — exactly the "don't invent behaviour" rule. When a submenu ticket
/// lands it turns its parent id enabled and hangs the child rows off it, still by variant.
///
/// `#[non_exhaustive]` is deliberate: a `match` in a later ticket (or in [`dispatch`]) must keep a
/// catch-all, so adding a variant here never silently changes another site's behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContextItem {
    // ── shared rows (both takes) ────────────────────────────────────────────────
    /// `Go Here` — move the editor camera to the clicked point. **Enabled** (T-664 wires it to the
    /// engine `set_view`).
    GoHere,
    /// `Play from Here` (empty-ground take) — start the preview at this point. Disabled: preview
    /// launch is not an editor feature yet (mod-side play). No single ticket owns it; kept as a
    /// faithful Eden row, disabled without a ticket tag.
    PlayFromHere,
    /// `Select` submenu parent. Disabled until the select-matching / select-in-view rows have a
    /// backing feature (`KEY-WP-001` adds the waypoint-adjacent selection verbs).
    Select,
    /// `Edit` submenu parent. Disabled: the flat Ctrl+C/Ctrl+V/Delete already live on the keyboard
    /// and as their own top-level rows below; the *submenu* (Paste on Original Position, etc.) is a
    /// later polish, not one of the six unblocked tickets.
    Edit,
    /// `Log` submenu parent (`Log Position to Clipboard`). Disabled: debug-logging to the clipboard
    /// is not an editor feature.
    Log,

    // ── empty-ground-only ──────────────────────────────────────────────────────
    /// `Place Comment` — drop an editor-only annotation at the clicked point (`PLACE-COMMENT-001`).
    ///
    /// **T-651 — ENABLED.** T-664 shipped this row disabled-with-ticket because the entry point
    /// existed before the feature did; T-651 authored the feature, so the row is live and
    /// [`dispatch`] calls `editor_ops::place_comment` at [`MenuTarget::world`]. The T-664 note said
    /// the marker would be authored by T-069 (`entitiesById`); that turned out to be the wrong home
    /// and is recorded here rather than quietly dropped. `entitiesById` COMPILES — it is
    /// `mission.schema.json`'s `entities[]` — and a comment must never reach a game server, so
    /// comments got their own `commentsById` root that `mission::flatten::EditorPayload` does not
    /// declare. T-069 still owns markers; it does not own this.
    PlaceComment,

    // ── on-entity-only ─────────────────────────────────────────────────────────
    /// `Connect` submenu parent (`Sync to` / `Group to` / `Set Trigger Owner`). Disabled — unblocked
    /// by `CONN-START-001`.
    Connect,
    /// `Play as the Character` — preview controlling this unit (replaces `Play from Here`). Disabled:
    /// same reason as [`PlayFromHere`](ContextItem::PlayFromHere).
    PlayAsCharacter,
    /// `Transform` submenu parent (Set as Group Leader / Move to Formation / Snap to Surface / Orient
    /// to Terrain|Sea Normal). Disabled — unblocked by `CTX-FORMATION-001`.
    Transform,
    /// `Grid` submenu parent (Use X|Y|Z as Grid). Disabled: object-dimension grid snapping is not an
    /// editor feature. No single unblocked ticket; a faithful Eden row, disabled without a tag.
    Grid,
    /// `Save Custom Composition...` — save the selection as a reusable composition. Disabled —
    /// unblocked by `COMP-SAVE-001`.
    SaveComposition,
    /// `Find in Asset Browser...` — reveal this entity's class in the right dock. Disabled: the
    /// asset browser has no reveal-and-scroll API yet (a later polish, not one of the six).
    FindInAssetBrowser,
    /// `Find in Config Viewer...` — open this class in a config viewer. Disabled: there is no config
    /// viewer surface in the editor.
    FindInConfigViewer,
    /// `Edit Loadout...` — open the arsenal for this unit. **Enabled** (T-664 wires it to
    /// `open_arsenal`, the Attributes Arsenal tab).
    EditLoadout,
    /// `Reset Loadout` — revert to the class default loadout. Disabled: loadout reset is not a
    /// standalone editor action yet.
    ResetLoadout,
    /// `Attributes...` — open the entity attributes dialog. **Enabled** (T-664 wires it to
    /// `open_attributes`; this is the same modal the double-click opens). Also carries the
    /// `ATTR-MULTI-001` forward interest: when multi-select attributes lands, this row's dispatch
    /// gains the multi path.
    Attributes,
}

impl ContextItem {
    /// The `ATTR-MULTI-001` attachment point in one place: which ticket unblocks a disabled item, or
    /// `None` when the item is either already enabled or a faithful-but-featureless Eden row that no
    /// single wave-106+ ticket owns.
    ///
    /// Kept as a method (not stored per-[`MenuEntry`]) so the *builders* stay a plain transcription
    /// and the ticket mapping lives with the id it is about — a later ticket edits exactly the arm
    /// for the row it is turning on.
    #[must_use]
    pub const fn unblocked_by(self) -> Option<&'static str> {
        match self {
            // T-651 shipped `PLACE-COMMENT-001`, so the row is enabled and has no blocking ticket.
            ContextItem::PlaceComment => None,
            ContextItem::Connect => Some("CONN-START-001"),
            ContextItem::Transform => Some("CTX-FORMATION-001"),
            ContextItem::SaveComposition => Some("COMP-SAVE-001"),
            // KEY-WP-001 (waypoints) reaches the menu through the Select submenu.
            ContextItem::Select => Some("KEY-WP-001"),
            // ATTR-MULTI-001 extends an already-*enabled* row (multi-select attributes), so it is
            // not a "disabled until" tag — it is named on `Attributes` in the docs above.
            _ => None,
        }
    }
}

/// One rendered row: the [`ContextItem`] id, its verbatim Eden label, an optional right-aligned
/// shortcut, whether it is live, and — when disabled — the ticket that will turn it on (if any).
///
/// A `separator` is modelled as `item: None` so the two builders read top-to-bottom exactly like the
/// Eden batch tables, dividers included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    /// `None` ⇒ this row is a separator (the other fields are ignored).
    pub item: Option<ContextItem>,
    /// The label exactly as Eden shows it (`"Place Comment"`, `"Attributes..."`, …).
    pub label: &'static str,
    /// Right-aligned shortcut hint (`"Ctrl+A"`, `"Delete"`), or `""` for menu-only rows.
    pub shortcut: &'static str,
    /// `true` when the backing feature ships today. A disabled row is greyed and non-interactive.
    pub enabled: bool,
    /// The blocking ticket for a disabled row (`Some("CONN-START-001")`), or `None`. Purely
    /// informational — rendered as a hover title so the operator (and the next agent) can see why a
    /// row is dark without leaving the app.
    pub blocked: Option<&'static str>,
    /// `true` when Eden draws a `▶` submenu affordance on this row. This slice does not open the
    /// submenu (its contents belong to later tickets), but the affordance is part of the faithful
    /// transcription and tells the operator the row is a parent.
    pub submenu: bool,
}

impl MenuEntry {
    /// A separator row (Eden's thin inset divider).
    const fn sep() -> Self {
        Self {
            item: None,
            label: "",
            shortcut: "",
            enabled: false,
            blocked: None,
            submenu: false,
        }
    }

    /// An **enabled** leaf row — its feature ships today.
    const fn on(item: ContextItem, label: &'static str) -> Self {
        Self {
            item: Some(item),
            label,
            shortcut: "",
            enabled: true,
            blocked: None,
            submenu: false,
        }
    }

    /// A **disabled** leaf row. `blocked` is the unblocking ticket, or `None` for a faithful Eden row
    /// that no single wave-106+ ticket owns (e.g. the Play/Log/Grid rows).
    const fn off(item: ContextItem, label: &'static str, blocked: Option<&'static str>) -> Self {
        Self {
            item: Some(item),
            label,
            shortcut: "",
            enabled: false,
            blocked,
            submenu: false,
        }
    }

    /// A **disabled submenu parent** (`▶`). Always disabled in this slice — the submenu contents are
    /// owned by later tickets, so there is nothing to open yet.
    const fn parent(item: ContextItem, label: &'static str, blocked: Option<&'static str>) -> Self {
        Self {
            item: Some(item),
            label,
            shortcut: "",
            enabled: false,
            blocked,
            submenu: true,
        }
    }

    /// Builder chain: attach a shortcut hint to a row.
    const fn with_shortcut(mut self, s: &'static str) -> Self {
        self.shortcut = s;
        self
    }
}

/// Which of Eden's two menus to render — the context-sensitivity the batch demonstrates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuTake {
    /// Right-click on bare terrain, nothing under the cursor. The short 6-item menu.
    EmptyGround,
    /// Right-click over an entity. The long entity-specific menu.
    OnEntity,
}

impl MenuTake {
    /// The rows for this take, verbatim from `batch01_context_menu.md` (empty-ground take at :119,
    /// on-entity take at :199), separators included and in order.
    #[must_use]
    pub fn entries(self) -> Vec<MenuEntry> {
        use ContextItem as I;
        match self {
            // Take A — nothing selected (batch :119-128).
            MenuTake::EmptyGround => vec![
                MenuEntry::on(I::GoHere, "Go Here"),
                // Preview launch is not an editor feature; faithful Eden row, no owning ticket.
                MenuEntry::off(I::PlayFromHere, "Play from Here", None),
                MenuEntry::sep(),
                MenuEntry::parent(I::Select, "Select", I::Select.unblocked_by()),
                MenuEntry::parent(I::Edit, "Edit", None),
                MenuEntry::parent(I::Log, "Log", None),
                MenuEntry::sep(),
                // T-651 — live: places an editor-only annotation at the right-clicked point.
                MenuEntry::on(I::PlaceComment, "Place Comment"),
            ],
            // Take B — one unit selected (batch :199-221). `Place Comment` is absent here (batch
            // :221); `Play from Here` becomes `Play as the Character` (batch :204).
            MenuTake::OnEntity => vec![
                MenuEntry::parent(I::Connect, "Connect", I::Connect.unblocked_by()),
                MenuEntry::sep(),
                MenuEntry::on(I::GoHere, "Go Here"),
                MenuEntry::off(I::PlayAsCharacter, "Play as the Character", None),
                MenuEntry::sep(),
                MenuEntry::parent(I::Select, "Select", I::Select.unblocked_by()),
                MenuEntry::parent(I::Edit, "Edit", None),
                MenuEntry::parent(I::Transform, "Transform", I::Transform.unblocked_by()),
                MenuEntry::parent(I::Grid, "Grid", None),
                MenuEntry::parent(I::Log, "Log", None),
                MenuEntry::sep(),
                MenuEntry::off(
                    I::SaveComposition,
                    "Save Custom Composition...",
                    I::SaveComposition.unblocked_by(),
                ),
                MenuEntry::off(I::FindInAssetBrowser, "Find in Asset Browser...", None),
                MenuEntry::off(I::FindInConfigViewer, "Find in Config Viewer...", None),
                MenuEntry::sep(),
                // Edit Loadout is live — it opens the Attributes Arsenal tab (T-068 arsenal).
                MenuEntry::on(I::EditLoadout, "Edit Loadout..."),
                MenuEntry::off(I::ResetLoadout, "Reset Loadout", None),
                MenuEntry::sep(),
                // Attributes is live — the same modal double-click opens.
                MenuEntry::on(I::Attributes, "Attributes..."),
            ],
        }
    }
}

/// What a right-click resolved to, after applying Eden's selection-aware retarget rule.
///
/// Carries the [`MenuTake`] to render **and** the target ids the on-entity actions operate on, so
/// the caller has a single value to open the menu with — the menu never re-derives the target.
// T-651 — `Eq` dropped (was `PartialEq, Eq`): `world` carries `f64`s, which are `PartialEq` only.
// Nothing keys a map or set on a `MenuTarget`; the derive existed because the struct happened to be
// all-`Eq`, and `MenuState` — the value that actually rides an `RwSignal` — was already `PartialEq`
// alone for exactly the same reason (its `x`/`y` pixels).
#[derive(Debug, Clone, PartialEq)]
pub struct MenuTarget {
    /// Which menu to show.
    pub take: MenuTake,
    /// The entity ids the on-entity actions act on. Empty for [`MenuTake::EmptyGround`].
    pub target_ids: Vec<String>,
    /// `Some(id)` when the right-click **re-targeted** the selection to a not-previously-selected
    /// entity (Eden's rule below). The caller replaces the live selection with `[id]` so the tint,
    /// SEL readout and the menu's target all agree — exactly what a plain left-click would have done.
    /// `None` when the click was on empty ground or on an already-selected entity (selection intact).
    pub retarget_to: Option<String>,
    /// T-651 — the WORLD point `(x, z)` in metres the right-click unprojects to, when the caller
    /// could compute one (it needs a live engine camera, so the pure [`resolve_target`] cannot).
    ///
    /// `Place Comment` is the first row whose action is about WHERE the click landed rather than
    /// WHAT it hit, and the menu's own `x`/`y` are screen pixels — unprojecting them at dispatch
    /// time would be a second, later camera read that a pan between open and click would make wrong.
    /// Capturing the point at open pins the annotation to the ground the operator right-clicked.
    pub world: Option<(f64, f64)>,
}

impl MenuTarget {
    /// T-651 — attach the unprojected world point of the right-click (builder, so the pure
    /// [`resolve_target`] rule and its tests stay unchanged). The wasm host chains this on.
    #[must_use]
    pub fn at_world(mut self, x: f64, z: f64) -> Self {
        self.world = Some((x, z));
        self
    }
}

/// **The hit-target / selection-retarget rule** (Eden's context sensitivity, batch §"Consolidated
/// findings" rule 1 + the ticket's selection-aware clause).
///
/// * `hit == None` (empty ground) ⇒ [`MenuTake::EmptyGround`]; no target, no retarget.
/// * `hit == Some(id)` **and** `id` is in the current `selection` ⇒ [`MenuTake::OnEntity`] targeting
///   the **whole selection** (a right-click inside a marquee acts on the group). No retarget.
/// * `hit == Some(id)` **and** `id` is **not** in `selection` ⇒ [`MenuTake::OnEntity`] **re-targeted
///   to the hit entity**: the target is `[id]` and [`MenuTarget::retarget_to`] is `Some(id)`, so the
///   caller replaces the selection with it — precisely how Eden (and this editor's own left-click,
///   `select_tool::apply_click`) behaves when you click an unselected object.
///
/// Pure and native-tested — the retarget rule is the load-bearing logic this ticket ships, and it is
/// exercised without a browser (see the unit tests below).
#[must_use]
pub fn resolve_target(hit: Option<&str>, selection: &[String]) -> MenuTarget {
    match hit {
        None => MenuTarget {
            take: MenuTake::EmptyGround,
            target_ids: Vec::new(),
            retarget_to: None,
            world: None,
        },
        Some(id) => {
            if selection.iter().any(|s| s == id) {
                // Inside the selection → act on the whole selection, leave it alone.
                MenuTarget {
                    take: MenuTake::OnEntity,
                    target_ids: selection.to_vec(),
                    retarget_to: None,
                    world: None,
                }
            } else {
                // Outside the selection → retarget to the hit entity (replace selection).
                MenuTarget {
                    take: MenuTake::OnEntity,
                    target_ids: vec![id.to_string()],
                    retarget_to: Some(id.to_string()),
                    world: None,
                }
            }
        }
    }
}

/// Open-menu state: where it sits and what it shows. Held in a [`RwSignal`] the wasm host installs;
/// `None` = closed (no DOM — the overlay renders nothing).
#[derive(Debug, Clone, PartialEq)]
pub struct MenuState {
    /// Screen pixel of the right-click (menu top-left anchor).
    pub x: f64,
    pub y: f64,
    /// The resolved take + target (from [`resolve_target`]).
    pub target: MenuTarget,
}

impl MenuState {
    /// The rows to render for this open menu.
    #[must_use]
    pub fn entries(&self) -> Vec<MenuEntry> {
        self.target.take.entries()
    }
}

/// The row indices in a take that are **selectable** (enabled leaves — not separators, not disabled
/// rows). Keyboard up/down walks this list; Enter fires the highlighted one. Extracted as a pure
/// function so the navigation is native-tested.
#[must_use]
pub fn selectable_indices(entries: &[MenuEntry]) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.item.is_some() && e.enabled)
        .map(|(i, _)| i)
        .collect()
}

/// Move the keyboard highlight. `cur` is the currently-highlighted entry index (or `None`); returns
/// the next highlighted **entry index**, skipping separators and disabled rows, clamped at the ends
/// (Eden does not wrap). `dir` is `+1` (down) or `-1` (up). `None` when there is nothing selectable.
///
/// Pure — the up/down behaviour is native-tested without a DOM.
#[must_use]
pub fn step_highlight(entries: &[MenuEntry], cur: Option<usize>, dir: i32) -> Option<usize> {
    let sel = selectable_indices(entries);
    if sel.is_empty() {
        return None;
    }
    // Position of `cur` within the selectable list, or a virtual "before first / after last" so the
    // first Down lands on the first row and the first Up on the last.
    let pos = cur.and_then(|c| sel.iter().position(|&i| i == c));
    let next = match (pos, dir) {
        (None, d) if d > 0 => 0,
        (None, _) => sel.len() - 1,
        (Some(p), d) if d > 0 => (p + 1).min(sel.len() - 1),
        (Some(p), _) => p.saturating_sub(1),
    };
    Some(sel[next])
}

// ─────────────────────────── wasm host bridge + dispatch + overlay ───────────────────────────
//
// Everything below drives the live editor (signals, engine, `editor_ops`) and so is wasm-only, the
// `mission_history` / `editor_ops` pattern. The item model + rules above stay native so their tests
// run on `cargo test -p website-frontend`.

// The context-menu signal, installed once from `mission_editor::on_load` (like the Attributes
// `attrs_open` signal). `Some` ⇒ the overlay is open at that state; `None` ⇒ closed.
//
// A `thread_local` mirrors how `editor_ops`/`mission_history` reach editor state from the ungated
// view: the `RwSignal` is `Copy` and cheap, and the wasm `contextmenu` closure has no other handle
// to it.
#[cfg(target_arch = "wasm32")]
thread_local! {
    static MENU: std::cell::RefCell<Option<RwSignal<Option<MenuState>>>> =
        const { std::cell::RefCell::new(None) };
}

/// Install the menu signal (once, from `on_load`).
#[cfg(target_arch = "wasm32")]
pub fn set_menu_signal(sig: RwSignal<Option<MenuState>>) {
    MENU.with(|m| *m.borrow_mut() = Some(sig));
}

/// Open the menu at screen pixel `(x, y)` for a resolved `target`. Applies the retarget side effect
/// first (so the selection/tint/SEL match the menu before it paints), then sets the signal.
///
/// Called from the `contextmenu` handler in `mission_editor`. The caller has already run the pick +
/// [`resolve_target`]; this is the one place the retarget is *committed* to the live selection.
#[cfg(target_arch = "wasm32")]
pub fn open(x: f64, y: f64, target: MenuTarget) {
    if let Some(id) = target.retarget_to.clone() {
        // Replace the selection with the hit entity — identical to a left-click on an unselected
        // object, so the map tint and SEL readout follow the menu's target.
        crate::editor_ops::select_slot(id);
    }
    MENU.with(|m| {
        if let Some(sig) = *m.borrow() {
            sig.set(Some(MenuState { x, y, target }));
        }
    });
}

/// Close the menu (Esc / click-away / after an action). No-op if already closed.
#[cfg(target_arch = "wasm32")]
pub fn close() {
    MENU.with(|m| {
        if let Some(sig) = *m.borrow() {
            sig.set(None);
        }
    });
}

/// Run an enabled item against `target_ids`, then close (menu-action-taken is a dismissal path).
///
/// A **disabled** item is a no-op — we never invent behaviour for a row whose ticket has not landed.
/// A later ticket adds its arm here (matching on the [`ContextItem`] **variant**) when it turns its
/// row on; the `#[non_exhaustive]` enum keeps the catch-all honest.
///
/// T-651 — `world` is [`MenuTarget::world`], the point the right-click unprojected to, forwarded
/// verbatim from the OPEN rather than recomputed here. `Place Comment` is the first row that acts on
/// a location instead of an entity; recomputing at click time would silently follow a camera the
/// operator panned while the menu was up.
#[cfg(target_arch = "wasm32")]
pub fn dispatch(item: ContextItem, target_ids: &[String], world: Option<(f64, f64)>) {
    match item {
        // Camera → the clicked entity's centroid (reusing the selection-center path). For the
        // empty-ground `Go Here` the caller passes no ids; center-on-selection then no-ops, which is
        // acceptable for this slice (teleport-to-arbitrary-point has no editor API and is not one of
        // the six unblocked features).
        ContextItem::GoHere => {
            crate::editor_ops::center_on_selection();
        }
        // Attributes / arsenal open on the single target id (the retarget already made it the
        // selection, so `target_ids[0]` is that entity).
        ContextItem::Attributes => {
            if let Some(id) = target_ids.first() {
                crate::editor_ops::open_attributes(id.clone());
            }
        }
        ContextItem::EditLoadout => {
            if let Some(id) = target_ids.first() {
                crate::editor_ops::open_arsenal(id.clone());
            }
        }
        // T-651 (`PLACE-COMMENT-001`) — place an editor-only annotation at the world point the
        // right-click unprojected to. `target_ids` is empty here by construction: the row lives in
        // the EmptyGround take only (Eden omits it on the entity take, `batch01_context_menu.md:221`),
        // so the action is about the POINT, not an entity. With no world point (a host that did not
        // supply one) this is a no-op rather than a guess at the map centre.
        ContextItem::PlaceComment => {
            if let Some((x, z)) = world {
                let _ = crate::editor_ops::place_comment(x, z);
            }
        }
        // Every other id is a disabled row (feature not shipped / owned by a later ticket) — no-op.
        _ => {}
    }
    close();
}

/// The floating context-menu overlay. Renders **no DOM while closed** (the `menu` signal is `None`),
/// so it is safe to mount unconditionally beside the other ungated dialogs.
///
/// **Mount rule (wave-101 verifier, binding):** this must sit with the *ungated* dialog mounts
/// (Attributes / Settings / Faction / ORBAT / Conflict), **not** inside the four `chrome_hidden`
/// gates — the menu has to survive Backspace hide-chrome (a floating overlay is not dock chrome).
///
/// **Dismissal (all three Eden paths):**
///   * **Esc** — the window keydown below; also drives up/down/Enter navigation while open.
///   * **click-away** — the full-screen backdrop under the panel closes on click.
///   * **action-taken** — [`dispatch`] closes after running an enabled row.
///
/// The panel is `pointer-events-auto` (it must catch its own clicks) over a transparent, click-away
/// backdrop; both are `fixed` so they anchor to the event pixel regardless of scroll.
#[component]
pub fn ContextMenuOverlay(menu: RwSignal<Option<MenuState>>) -> impl IntoView {
    // Keyboard: Esc closes; ArrowUp/Down move the highlight; Enter fires it. Installed once; the
    // handler no-ops while the menu is closed. `highlight` is the highlighted **entry index**.
    let highlight = RwSignal::new(None::<usize>);
    #[cfg(target_arch = "wasm32")]
    {
        let key = window_event_listener(leptos::ev::keydown, move |ev| {
            let Some(state) = menu.get_untracked() else {
                return;
            };
            match ev.key().as_str() {
                "Escape" => {
                    ev.prevent_default();
                    close();
                }
                "ArrowDown" => {
                    ev.prevent_default();
                    let entries = state.entries();
                    highlight.set(step_highlight(&entries, highlight.get_untracked(), 1));
                }
                "ArrowUp" => {
                    ev.prevent_default();
                    let entries = state.entries();
                    highlight.set(step_highlight(&entries, highlight.get_untracked(), -1));
                }
                "Enter" => {
                    ev.prevent_default();
                    let entries = state.entries();
                    if let Some(idx) = highlight.get_untracked() {
                        if let Some(entry) = entries.get(idx) {
                            if let Some(item) = entry.item {
                                if entry.enabled {
                                    dispatch(item, &state.target.target_ids, state.target.world);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        });
        on_cleanup(move || key.remove());
    }
    // Reset the highlight every time the menu (re)opens so a stale highlight from a prior open never
    // leaks in.
    Effect::new(move |_| {
        if menu.get().is_some() {
            highlight.set(None);
        }
    });

    move || {
        let state = menu.get()?;
        let entries = state.entries();
        let target_ids = state.target.target_ids.clone();
        // T-651 — the unprojected right-click point rides every row so `Place Comment` acts on the
        // ground that was clicked, not on a later camera read.
        let world = state.target.world;
        // Anchor at the event pixel. `max-w` + the viewport keep it on screen; a fuller Eden-parity
        // flip/clamp (batch rule 4/5) is a later polish — the ticket ships the menu, its targeting
        // and dismissal.
        let pos = format!("left:{:.0}px;top:{:.0}px", state.x, state.y);
        let rows = entries
            .into_iter()
            .enumerate()
            .map(|(idx, e)| render_row(idx, e, target_ids.clone(), world, highlight))
            .collect_view();
        Some(view! {
            // Click-away backdrop — transparent, full-screen, closes on any click. `z-40` sits under
            // the panel (`z-50`) but over the map/chrome so a click anywhere dismisses.
            <div
                class="fixed inset-0 z-40"
                on:pointerdown=move |ev| {
                    ev.stop_propagation();
                    #[cfg(target_arch = "wasm32")]
                    close();
                }
                on:contextmenu=move |ev| ev.prevent_default()
            ></div>
            <div
                class="glass animate-dialog-in fixed z-50 min-w-[15rem] max-w-[20rem] overflow-hidden rounded-md border border-outline-variant/30 py-1 shadow-2xl outline-none"
                style=pos
                // Keep a right-click *on the menu* from opening a second browser menu.
                on:contextmenu=move |ev| ev.prevent_default()
                on:pointerdown=move |ev| ev.stop_propagation()
            >
                {rows}
            </div>
        })
    }
}

/// Render one menu row (or a separator). Enabled rows dispatch on click and highlight on hover;
/// disabled rows are greyed and inert, with the blocking ticket as a hover title. The
/// keyboard-highlighted row gets the amber selection bar (Eden's hovered-row styling).
fn render_row(
    idx: usize,
    entry: MenuEntry,
    target_ids: Vec<String>,
    // T-651 — the right-click's world point (see [`MenuTarget::world`]); `None` off the wasm host.
    world: Option<(f64, f64)>,
    highlight: RwSignal<Option<usize>>,
) -> AnyView {
    // Separator.
    let Some(item) = entry.item else {
        return view! {
            <div class="my-1 h-px bg-outline-variant/25" aria-hidden="true"></div>
        }
        .into_any();
    };

    let label = entry.label;
    let shortcut = entry.shortcut;
    let submenu = entry.submenu;
    let enabled = entry.enabled;
    // Disabled rows name their blocking ticket in the tooltip so the operator (and the next agent)
    // sees why a row is dark. An enabled row has no tooltip.
    let title = entry
        .blocked
        .map(|t| format!("Not available yet — {t}"))
        .unwrap_or_default();

    let is_hi = move || highlight.get() == Some(idx);
    let base = "flex w-full items-center gap-3 px-3 py-1 text-left text-label-md select-none";

    let on_click = {
        let target_ids = target_ids.clone();
        move |_ev: leptos::ev::MouseEvent| {
            if enabled {
                #[cfg(target_arch = "wasm32")]
                dispatch(item, &target_ids, world);
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (item, &target_ids, world);
            }
        }
    };

    view! {
        <button
            type="button"
            disabled=!enabled
            title=title
            class=base
            // Amber selection bar when keyboard-highlighted or hovered (enabled only); disabled rows
            // stay dim and never highlight.
            class:cursor-pointer=enabled
            class:text-on-surface=enabled
            class:text-on-surface-variant=move || !enabled
            class:opacity-40=!enabled
            class:bg-tactical-yellow=move || enabled && is_hi()
            class:text-background=move || enabled && is_hi()
            on:pointerenter=move |_| {
                if enabled {
                    highlight.set(Some(idx));
                }
            }
            on:click=on_click
        >
            <span class="flex-1 truncate">{label}</span>
            {(!shortcut.is_empty())
                .then(|| {
                    view! {
                        <span class="ml-4 shrink-0 font-mono text-code-sm text-outline">
                            {shortcut}
                        </span>
                    }
                })}
            {submenu.then(|| view! { <span class="shrink-0 text-outline">"\u{25B6}"</span> })}
        </button>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| (*s).to_string()).collect()
    }

    // ── item model / transcription fidelity ─────────────────────────────────────

    #[test]
    fn empty_ground_take_matches_eden_batch() {
        // batch :119-128 — six interactive rows, two separators, in order.
        let e = MenuTake::EmptyGround.entries();
        let labels: Vec<&str> = e
            .iter()
            .filter(|r| r.item.is_some())
            .map(|r| r.label)
            .collect();
        assert_eq!(
            labels,
            vec![
                "Go Here",
                "Play from Here",
                "Select",
                "Edit",
                "Log",
                "Place Comment"
            ]
        );
        // Two separators (after Play from Here, after Log).
        assert_eq!(e.iter().filter(|r| r.item.is_none()).count(), 2);
    }

    #[test]
    fn on_entity_take_matches_eden_batch() {
        // batch :199-221 — the long take. `Place Comment` is absent; `Play as the Character`
        // replaces `Play from Here`.
        let e = MenuTake::OnEntity.entries();
        let labels: Vec<&str> = e
            .iter()
            .filter(|r| r.item.is_some())
            .map(|r| r.label)
            .collect();
        assert_eq!(
            labels,
            vec![
                "Connect",
                "Go Here",
                "Play as the Character",
                "Select",
                "Edit",
                "Transform",
                "Grid",
                "Log",
                "Save Custom Composition...",
                "Find in Asset Browser...",
                "Find in Config Viewer...",
                "Edit Loadout...",
                "Reset Loadout",
                "Attributes...",
            ]
        );
        assert!(
            !labels.contains(&"Place Comment"),
            "Place Comment must be omitted from the on-entity take (batch :221)"
        );
        assert!(
            !labels.contains(&"Play from Here"),
            "Play from Here is replaced by Play as the Character on-entity (batch :204)"
        );
    }

    #[test]
    fn enabled_rows_are_exactly_the_shipping_features() {
        // The only rows we light up are the ones whose feature exists today. Everything else is a
        // disabled Eden row (with or without an owning ticket). If a later ticket enables a row it
        // updates this list deliberately.
        let mut on: Vec<&str> = MenuTake::EmptyGround
            .entries()
            .into_iter()
            .chain(MenuTake::OnEntity.entries())
            .filter(|r| r.enabled)
            .map(|r| r.label)
            .collect();
        on.sort_unstable();
        on.dedup();
        // T-651 turned `Place Comment` on — the deliberate list update this test asks for. It is the
        // FIRST of T-664's six forward-contract rows to ship, and it shipped by matching on the
        // variant in `dispatch`, exactly as the id-enum contract intended.
        assert_eq!(
            on,
            vec![
                "Attributes...",
                "Edit Loadout...",
                "Go Here",
                "Place Comment"
            ]
        );
    }

    #[test]
    fn disabled_rows_that_have_an_owning_ticket_name_it() {
        // The six-ticket forward contract: each of these disabled rows must carry its blocking
        // ticket so a later agent can find its attachment point.
        // T-651 — `PlaceComment` LEFT this list: its feature shipped, so it is enabled and carries
        // no blocking ticket. The remaining four are still the forward contract, and the assertion
        // below (`unblocked_by() == None` for the shipped row) is what stops a stale ticket tag from
        // outliving the work.
        assert_eq!(
            ContextItem::PlaceComment.unblocked_by(),
            None,
            "PLACE-COMMENT-001 shipped in T-651 — the row must not still name a blocking ticket"
        );
        let want = [
            (ContextItem::Connect, "CONN-START-001"),
            (ContextItem::Transform, "CTX-FORMATION-001"),
            (ContextItem::SaveComposition, "COMP-SAVE-001"),
            (ContextItem::Select, "KEY-WP-001"),
        ];
        let all: Vec<MenuEntry> = MenuTake::EmptyGround
            .entries()
            .into_iter()
            .chain(MenuTake::OnEntity.entries())
            .collect();
        for (item, ticket) in want {
            let row = all
                .iter()
                .find(|r| r.item == Some(item))
                .unwrap_or_else(|| panic!("{item:?} must appear in a take"));
            assert!(
                !row.enabled,
                "{item:?} must be disabled (feature not shipped)"
            );
            assert_eq!(
                row.blocked,
                Some(ticket),
                "{item:?} must name its unblocking ticket {ticket}"
            );
            assert_eq!(item.unblocked_by(), Some(ticket));
        }
    }

    #[test]
    fn item_id_is_an_enum_not_a_label() {
        // The forward contract in one assertion: the model keys on a typed variant, and a submenu
        // parent is a distinct id a later ticket matches on — not the string "Connect".
        assert_ne!(ContextItem::Connect, ContextItem::Transform);
        assert_eq!(ContextItem::Attributes.unblocked_by(), None); // enabled, no "disabled until"
    }

    // ── hit-target / selection-retarget rule (the load-bearing logic) ───────────

    #[test]
    fn empty_ground_when_nothing_hit() {
        let t = resolve_target(None, &sel(&["a", "b"]));
        assert_eq!(t.take, MenuTake::EmptyGround);
        assert!(t.target_ids.is_empty());
        assert_eq!(t.retarget_to, None);
    }

    #[test]
    fn hit_inside_selection_targets_the_whole_selection_no_retarget() {
        // Right-click a member of a multi-select → the menu acts on the group, selection untouched.
        let t = resolve_target(Some("b"), &sel(&["a", "b", "c"]));
        assert_eq!(t.take, MenuTake::OnEntity);
        assert_eq!(t.target_ids, sel(&["a", "b", "c"]));
        assert_eq!(
            t.retarget_to, None,
            "an already-selected hit must not retarget"
        );
    }

    #[test]
    fn hit_outside_selection_retargets_to_the_hit_entity() {
        // Right-click an unselected entity → retarget to it (replace selection), exactly like a
        // left-click on an unselected object.
        let t = resolve_target(Some("z"), &sel(&["a", "b"]));
        assert_eq!(t.take, MenuTake::OnEntity);
        assert_eq!(t.target_ids, sel(&["z"]));
        assert_eq!(t.retarget_to, Some("z".to_string()));
    }

    #[test]
    fn hit_with_empty_selection_retargets() {
        // Nothing selected, right-click an entity → it becomes the target/selection.
        let t = resolve_target(Some("z"), &[]);
        assert_eq!(t.take, MenuTake::OnEntity);
        assert_eq!(t.target_ids, sel(&["z"]));
        assert_eq!(t.retarget_to, Some("z".to_string()));
    }

    // ── keyboard navigation (up/down/Enter) ─────────────────────────────────────

    #[test]
    fn selectable_indices_skip_separators_and_disabled_rows() {
        let e = MenuTake::EmptyGround.entries();
        let sel = selectable_indices(&e);
        // Only the enabled leaves: Go Here (idx 0). Everything else in this take is disabled or a
        // separator.
        for &i in &sel {
            assert!(e[i].item.is_some() && e[i].enabled);
        }
        assert!(sel.contains(&0), "Go Here (enabled) must be selectable");
    }

    #[test]
    fn step_highlight_walks_only_enabled_rows_and_clamps() {
        let e = MenuTake::OnEntity.entries();
        let selectable = selectable_indices(&e);
        assert!(
            selectable.len() >= 3,
            "Go Here + Edit Loadout + Attributes at least"
        );
        // First Down lands on the first selectable row.
        let first = step_highlight(&e, None, 1).unwrap();
        assert_eq!(first, selectable[0]);
        // First Up (from nothing) lands on the last selectable row.
        let last = step_highlight(&e, None, -1).unwrap();
        assert_eq!(last, *selectable.last().unwrap());
        // Down from the last clamps (no wrap).
        assert_eq!(step_highlight(&e, Some(last), 1), Some(last));
        // Up from the first clamps.
        assert_eq!(step_highlight(&e, Some(first), -1), Some(first));
        // A landed highlight is always an enabled leaf.
        assert!(e[first].enabled && e[first].item.is_some());
        assert!(e[last].enabled && e[last].item.is_some());
    }

    #[test]
    fn step_highlight_none_when_no_selectable_rows() {
        // A take with no enabled rows never yields a highlight (Enter would then no-op).
        let all_off = vec![
            MenuEntry::sep(),
            MenuEntry::off(ContextItem::Grid, "Grid", None),
        ];
        assert_eq!(step_highlight(&all_off, None, 1), None);
        assert_eq!(step_highlight(&all_off, None, -1), None);
    }

    /* ─────────────── T-651 — Place Comment: the enabled row and its world point ─────────────── */

    /// `Place Comment` is LIVE and lives on the empty-ground take ONLY (Eden omits it on the entity
    /// take, `batch01_context_menu.md:221`). Both halves matter: enabled-and-present is what T-651
    /// ships, and absent-on-entity is what keeps a right-click on a unit from placing a note on top
    /// of it.
    #[test]
    fn place_comment_is_enabled_and_empty_ground_only() {
        let empty = MenuTake::EmptyGround.entries();
        let row = empty
            .iter()
            .find(|r| r.item == Some(ContextItem::PlaceComment))
            .expect("Place Comment on the empty-ground take");
        assert!(row.enabled, "T-651 shipped the feature");
        assert_eq!(row.blocked, None, "an enabled row names no blocking ticket");
        assert!(!row.submenu, "it is a leaf action, not a parent");
        assert!(
            !MenuTake::OnEntity
                .entries()
                .iter()
                .any(|r| r.item == Some(ContextItem::PlaceComment)),
            "still omitted from the on-entity take"
        );
        // It is reachable by keyboard, which is the practical meaning of "enabled" for this menu.
        assert!(selectable_indices(&empty)
            .iter()
            .any(|&i| empty[i].item == Some(ContextItem::PlaceComment)));
    }

    /// **THE PLACE GESTURE, AS AN EVENT SEQUENCE** (not a source pin): right-click empty ground at a
    /// known world point → the menu that opens targets no entity, carries THAT point, and offers a
    /// live `Place Comment` row. Those four facts together are what make the dispatch land the
    /// annotation on the ground the operator clicked.
    ///
    /// The second half is the one that actually catches regressions: a SECOND right-click elsewhere
    /// must replace the point. A menu that cached the first click's world position would place every
    /// later comment at the first spot — a bug invisible on any single-event test.
    #[test]
    fn right_click_sequence_carries_each_click_own_world_point() {
        // 1. Right-click empty ground at world (100, 200) with a live selection.
        let selection = vec!["s1".to_string(), "s2".to_string()];
        let first = resolve_target(None, &selection).at_world(100.0, 200.0);
        assert_eq!(first.take, MenuTake::EmptyGround);
        assert!(
            first.target_ids.is_empty(),
            "the empty-ground take acts on a POINT, not on the selection"
        );
        assert_eq!(
            first.retarget_to, None,
            "an empty-ground right-click must not disturb the selection"
        );
        assert_eq!(first.world, Some((100.0, 200.0)));
        assert!(
            first
                .take
                .entries()
                .iter()
                .any(|r| r.item == Some(ContextItem::PlaceComment) && r.enabled),
            "the row the operator is about to click is live"
        );

        // 2. Dismiss, right-click again somewhere else: the NEW point wins.
        let second = resolve_target(None, &selection).at_world(7_000.5, -12.25);
        assert_eq!(
            second.world,
            Some((7_000.5, -12.25)),
            "each right-click carries its own point — no cached first click"
        );
        assert_ne!(first.world, second.world);

        // 3. A right-click ON an entity resolves to the other take, so `Place Comment` is not even
        //    offered — the world point riding along is inert there.
        let on_entity = resolve_target(Some("s1"), &selection).at_world(1.0, 2.0);
        assert_eq!(on_entity.take, MenuTake::OnEntity);
        assert!(!on_entity
            .take
            .entries()
            .iter()
            .any(|r| r.item == Some(ContextItem::PlaceComment)));
    }

    /// A host that supplies no world point (nothing to unproject against — no engine) leaves
    /// `world` at `None`, and the dispatch's documented behaviour there is to do NOTHING rather than
    /// guess a location. Pinned so `at_world` can never become implicitly-zero.
    #[test]
    fn a_target_without_a_world_point_stays_none() {
        assert_eq!(resolve_target(None, &[]).world, None);
        assert_eq!(resolve_target(Some("a"), &[]).world, None);
    }
}
