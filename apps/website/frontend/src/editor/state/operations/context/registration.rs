//! Role: registration.
//! Position: `editor/state/operations/context` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

/// Domain representation of ops ctx.
pub(in crate::editor::state::operations) struct OpsCtx {
    /// Doc.
    pub(in crate::editor::state::operations) doc: DocHandle,

    /// Engine.
    pub(in crate::editor::state::operations) engine: EngineHandle,

    /// Selection.
    pub(in crate::editor::state::operations) selection: SelectionHandle,

    /// Active layer.
    pub(in crate::editor::state::operations) active_layer: RwSignal<Option<String>>,

    /// Active side.
    pub(in crate::editor::state::operations) active_side: RwSignal<String>,

    /// Objects mode.
    pub(in crate::editor::state::operations) objects_mode: RwSignal<bool>,

    /// Dock mirrors — `MissionDocCore` has no change subscription, so these are pushed from [`refresh_docks`] at every mutation site, like the OBJ/SEL readouts.
    pub(in crate::editor::state::operations) outliner_nodes: RwSignal<Vec<OutlinerNode>>,

    /// Orbat nodes.
    pub(in crate::editor::state::operations) orbat_nodes: RwSignal<Vec<OutlinerNode>>,

    /// Selected ids.
    pub(in crate::editor::state::operations) selected_ids: RwSignal<Vec<String>>,

    /// Attrs open.
    pub(in crate::editor::state::operations) attrs_open: RwSignal<Option<String>>,

    /// Attrs tab.
    pub(in crate::editor::state::operations) attrs_tab: RwSignal<usize>,

    /// Doc tick.
    pub(in crate::editor::state::operations) doc_tick: RwSignal<u64>,

    /// The in-flight palette drag: `Some` between a leaf `pointerdown` and the canvas `pointerup`.
    pub(in crate::editor::state::operations) pending: RefCell<Option<Pending>>,

    /// Monotonic minter for placed-slot ids; [`mint_id`] still proves uniqueness against the doc.
    pub(in crate::editor::state::operations) next_id: Cell<u32>,
}

/// The discriminant lives here, on the armed value, rather than on a separate "current tab" signal: the tab can change (or the dock can unmount) between the leaf's `pointerdown` and the canvas's `pointerup`, and a place must commit the entity the operator actually picked up.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::editor::state::operations) enum Pending {
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

    /// Domain representation of zone.
    Zone(ZoneDraft),
}

/// Lives on `ctx.pending` (rather than in a signal of its own) for the same reason the palette arms do: `has_pending()` is what makes `mission_editor`'s pointer handlers route a canvas release to [`place_at`] instead of the select/marquee machine, and re-deriving "is a draw in flight" from a second source is how the two get out of step.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneDraft {
    /// Schema `zone.type`, taken from `$defs/zone/properties/type/enum` by the dock — never typed.
    pub kind: String,

    /// Shape.
    pub shape: ZoneShape,

    /// Circle: the centre, set by the first click. `None` until then.
    pub centre: Option<(f64, f64)>,

    /// Polygon: the ring so far, one vertex per click.
    pub verts: Vec<(f64, f64)>,

    /// Target.
    pub target: Option<String>,

    /// Collection.
    pub collection: DrawTarget,
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

/// Install the ops context (once, from `on_load`, after the doc is seeded).
#[allow(clippy::too_many_arguments)]
pub fn set_ctx(
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
    OPS_CTX.with(|c| {
        *c.borrow_mut() = Some(OpsCtx {
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
