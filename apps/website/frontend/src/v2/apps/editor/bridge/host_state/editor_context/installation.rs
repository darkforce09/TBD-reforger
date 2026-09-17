//! Role: the installed context itself — the handles and signals the editor is wired to at load,
//! the value an armed placement carries until it is dropped, and the registration of the side
//! signals a panel opens and closes.
//! Position: `editor/bridge/host_state/editor_context` in the frontend editor shell.
//! Signals & state: the one thread-local context and the per-signal thread-locals above it.
//! Invariants: the context is installed once, after the document is seeded, and never replaced
//! piecemeal. A side signal that was never registered is silence — opening or closing it is a
//! no-op rather than an error, because a panel that is not mounted has nothing to show.

use super::*;

/// Every handle and signal the editor's panels reach the open mission through.
pub(crate) struct EditorContext {
    /// Doc.
    pub(crate) doc: DocHandle,

    /// Engine.
    pub(crate) engine: EngineHandle,

    /// Selection.
    pub(crate) selection: SelectionHandle,

    /// Active layer.
    pub(crate) active_layer: RwSignal<Option<String>>,

    /// Active side.
    pub(crate) active_side: RwSignal<String>,

    /// Objects mode.
    pub(crate) objects_mode: RwSignal<bool>,

    /// Dock mirrors — `MissionDocCore` has no change subscription, so these are pushed from [`refresh_docks`] at every mutation site, like the OBJ/SEL readouts.
    pub(crate) outliner_nodes: RwSignal<Vec<OutlinerNode>>,

    /// Orbat nodes.
    pub(crate) orbat_nodes: RwSignal<Vec<OutlinerNode>>,

    /// Selected ids.
    pub(crate) selected_ids: RwSignal<Vec<String>>,

    /// Attrs open.
    pub(crate) attrs_open: RwSignal<Option<String>>,

    /// Attrs tab.
    pub(crate) attrs_tab: RwSignal<usize>,

    /// Doc tick.
    pub(crate) doc_tick: RwSignal<u64>,

    /// The in-flight palette drag: `Some` between a leaf `pointerdown` and the canvas `pointerup`.
    pub(crate) pending: RefCell<Option<Pending>>,

    /// Monotonic minter for placed-slot ids; [`mint_id`] still proves uniqueness against the doc.
    pub(crate) next_id: Cell<u32>,
}

/// Expose website mission core :: doc :: operations :: entity ::  zone draft at this domain boundary.
pub use website_map_engine::data::store::operations::entity::ZoneDraft;

/// The discriminant lives here, on the armed value, rather than on a separate "current tab" signal: the tab can change (or the dock can unmount) between the leaf's `pointerdown` and the canvas's `pointerup`, and a place must commit the entity the operator actually picked up.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Pending {
    /// Domain representation of character.
    Character(PlacePayload),

    /// Domain representation of vehicle.
    Vehicle(PlacePayload),

    /// Domain representation of object.
    Object(PlacePayload),

    /// Domain representation of composition.
    Composition(String),

    /// Domain representation of marker.
    Marker(String),

    /// A zone or trigger being drawn. The draft lives HERE, on the armed value, rather than in a
    /// signal of its own, for the same reason the palette arms do: `has_pending()` is what makes
    /// `mission_editor`'s pointer handlers route a canvas release to the draw instead of the
    /// select/marquee machine, and re-deriving "is a draw in flight" from a second source is how
    /// the two get out of step.
    Zone(ZoneDraft),
}

/// Set place with crew using the supplied domain data.
pub fn set_place_with_crew(with_crew: bool) {
    PLACE_WITH_CREW.with(|f| f.set(with_crew));
}

/// Place with crew using the supplied domain data.
#[must_use]
pub fn place_with_crew() -> bool {
    PLACE_WITH_CREW.with(Cell::get)
}

/// Set asset picker signal using the supplied domain data.
pub fn set_asset_picker_signal(sig: RwSignal<Option<AssetPickerState>>) {
    ASSET_PICKER.with(|s| *s.borrow_mut() = Some(sig));
}

/// Open asset picker using the supplied domain data.
pub fn open_asset_picker(wx: f64, wy: f64, screen_x: f64, screen_y: f64) {
    ASSET_PICKER.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(Some(AssetPickerState {
                wx,
                wy,
                screen_x,
                screen_y,
            }));
        }
    });
}

/// Close asset picker using the supplied domain data.
pub fn close_asset_picker() {
    ASSET_PICKER.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(None);
        }
    });
}

/// Set comment editor signal using the supplied domain data.
pub fn set_comment_editor_signal(sig: RwSignal<Option<String>>) {
    COMMENT_EDITOR.with(|s| *s.borrow_mut() = Some(sig));
}

/// Open comment editor using the supplied domain data.
pub fn open_comment_editor(id: String) {
    COMMENT_EDITOR.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(Some(id));
        }
    });
}

/// Close comment editor using the supplied domain data.
pub fn close_comment_editor() {
    COMMENT_EDITOR.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(None);
        }
    });
}

/// Drop an armed composition place that names `id`, after that composition has been removed from
/// the library. Scoped to the one id so a different arm survives the delete, and silent when
/// nothing is armed — the arm is host state, not document state, so nothing about it is undoable.
pub fn cancel_armed_composition(id: &str) {
    EDITOR_CONTEXT.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let armed = matches!(&*ctx.pending.borrow(), Some(Pending::Composition(p)) if p == id);
            if armed {
                *ctx.pending.borrow_mut() = None;
            }
        }
    });
}

/// Install the editor context — once, at load, after the document is seeded.
#[allow(clippy::too_many_arguments)]
pub fn install(
    doc: DocHandle,
    engine: EngineHandle,
    selection: SelectionHandle,
    active_layer: RwSignal<Option<String>>,
    active_side: RwSignal<String>,
    objects_mode: RwSignal<bool>,
    outliner_nodes: RwSignal<Vec<OutlinerNode>>,
    orbat_nodes: RwSignal<Vec<OutlinerNode>>,
    selected_ids: RwSignal<Vec<String>>,
    attrs_open: RwSignal<Option<String>>,
    attrs_tab: RwSignal<usize>,
    doc_tick: RwSignal<u64>,
) {
    EDITOR_CONTEXT.with(|c| {
        *c.borrow_mut() = Some(EditorContext {
            doc,
            engine,
            selection,
            active_layer,
            active_side,
            objects_mode,
            outliner_nodes,
            orbat_nodes,
            selected_ids,
            attrs_open,
            attrs_tab,
            doc_tick,
            pending: RefCell::new(None),
            next_id: Cell::new(0),
        });
    });
}
