//! Attributes modal — the AttributesModal.tsx + RightInspector/fields.tsx port (T-159.26, spec
//! `t159_23_attributes_modal.md`). Opened by dbl-clicking a slot on the map or activating an
//! outliner row; Esc / backdrop / ✕ close. Tabs: **Transform**
//! (X/Y/Z/Rotation NumberFields committing on blur/Enter via `update_slot_position`, plus a Stance
//! select), **Identity** (Role/Tag TextFields + readonly Squad), **States** (trait stub), and
//! **Arsenal** (live loadout editor — T-068.10 / T-180.9; `open_arsenal` selects tab index 3).
//! Commits run `engine_ops::attrs_update_*` → `after_local_edit` (rebind + persist + one undo
//! step per commit — A4).
//!
//! The field values re-read from the doc on every `doc_ver` bump, so an undo while the modal is
//! open refreshes the fields — and if the slot itself was undone away, the modal closes.
//!
//! **T-818 — vehicle Attributes.** A placed vehicle's dblclick still opens this modal (T-647
//! ATTR-OPEN). Vehicles are off the slot SoA, so the host routes `read_attrs == None` +
//! `is_vehicle_id` into a Heading° / Cargo / Crew body that reuses the DockRight Placed-strip
//! mutators (`set_vehicle_heading` / `set_vehicle_cargo` / `assign_crew_seat` / `clear_crew_seat`).
//! Heading commits through [`number_field`] (T-785). Multi-edit stays slot-only.
//!
//! **T-649 (ATTR-MULTI-001 / ATTR-MULTI-CHK-001) — multi-edit.** A multi-selection used to
//! SUPPRESS this modal (the old A1 rule, a hard `return` in `editor_context::open_attributes`). It now
//! opens it over the whole selection, and every commit fans out to every selected slot. The Eden
//! rule for which fields are live is per-field, not per-modal:
//!   * a field whose value is **the same** on every selected slot shows that value and edits as it
//!     always has — typing in it writes the value to all of them;
//!   * a field whose values **differ** has no truthful value to show, so it renders blank and
//!     **disabled** behind a per-field "Apply to all" checkbox. Ticking the box is the operator
//!     saying "yes, overwrite this column on all of them", and only then does the input accept
//!     input. Untouched fields are passed as `None` and the core leaves those columns alone, so a
//!     stance multi-edit can never also stamp one slot's X onto the rest.
//!
//! `engine_ops::read_attrs_diff` owns the "do they differ" half; the checkbox + disable half is
//! here.
//!
//! **T-700 (3DEN-PLACE-013) — the numeric nudge.** Every `number_field` now moves by PageUp /
//! PageDown — and, since wave-127 F-1, by ArrowUp / ArrowDown, which the browser used to step
//! ONTO THE STEP GRID (`412.37` + one ArrowUp = `413`) until this handler and `step="any"` took
//! them — with `Ctrl` / `Shift` / `Alt` scaling the step (`nudge_step`). A nudge writes the
//! field's local draft only, so a burst coalesces into the one blur/Enter commit the field already
//! made, and a field the T-082 gate has shut refuses the keyboard exactly as it refuses typing.
#![allow(dead_code)]

use leptos::prelude::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub use website_map_engine::data::store::operations::reassign::faction_label;
#[cfg(target_arch = "wasm32")]
use website_map_engine::editing::hosted_commands as engine_ops;

const CONTROL: &str = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";
/// T-649 — added to a field that is disabled because its values differ and its checkbox is
/// unticked. Purely visual; the real gate is the `disabled` attribute.
const CONTROL_LOCKED: &str = " cursor-not-allowed opacity-40";
const TABS: [&str; 4] = ["Transform", "Identity", "States", "Arsenal"];

/// T-649 ATTR-MULTI-CHK-001 — the per-field opt-in latches, one per editable Attributes field.
///
/// They live on the COMPONENT, not inside the render closure, because the modal body re-renders on
/// every `doc_tick` bump: a latch minted inside the render would un-tick itself the instant its own
/// commit landed, so a multi-edit would survive exactly one keystroke.
#[derive(Clone, Copy)]
struct MultiOpts {
    x: RwSignal<bool>,
    y: RwSignal<bool>,
    z: RwSignal<bool>,
    rotation: RwSignal<bool>,
    stance: RwSignal<bool>,
    role: RwSignal<bool>,
    tag: RwSignal<bool>,
    /// T-082 ATTR-FIELD-OBJ-TYPE.
    asset_id: RwSignal<bool>,
    /// T-082 ATTR-FIELD-OBJ-ROLE-DESC.
    description: RwSignal<bool>,
}

impl MultiOpts {
    fn new() -> Self {
        Self {
            x: RwSignal::new(false),
            y: RwSignal::new(false),
            z: RwSignal::new(false),
            rotation: RwSignal::new(false),
            stance: RwSignal::new(false),
            role: RwSignal::new(false),
            tag: RwSignal::new(false),
            asset_id: RwSignal::new(false),
            description: RwSignal::new(false),
        }
    }

    /// Re-arm every latch (all OFF). Run when the modal opens on a new target so an opt-in granted
    /// for one selection can never leak into the next one.
    fn reset(self) {
        for s in [
            self.x,
            self.y,
            self.z,
            self.rotation,
            self.stance,
            self.role,
            self.tag,
            self.asset_id,
            self.description,
        ] {
            s.set(false);
        }
    }
}

/// T-649 ATTR-MULTI-CHK-001 — one field's multi-edit gate.
///
/// `None` is the single-selection case **and** the multi case where every selected slot already
/// agrees: the field renders exactly as it did before this slice — no checkbox, always live.
/// `Some(latch)` means the values DIFFER: blank + disabled + an "Apply to all" checkbox bound to
/// `latch`. Copy, so it can be captured by the field's reactive closures.
///
/// T-082 (wave-102 F-7) — `shut` is a SECOND, independent reason a field can be dead, and it is not
/// a multi-edit concept at all: the core will REFUSE this write whatever the operator ticks
/// (a transform edit against a transform-locked layer). It therefore overrides the latch rather
/// than sharing it — ticking "Apply to all" must not re-enable a field whose write the core drops
/// on the floor, which is the F-7 lie stated as code.
#[derive(Clone, Copy)]
struct Gate {
    opt: Option<RwSignal<bool>>,
    /// The core refuses this write outright — no latch can open it.
    shut: bool,
}

impl Gate {
    /// No gate — single selection, or a field the whole selection agrees on.
    const fn open() -> Self {
        Self {
            opt: None,
            shut: false,
        }
    }

    /// Gated only when the values actually differ across the selection.
    fn maybe(differs: bool, latch: RwSignal<bool>) -> Self {
        Self {
            opt: differs.then_some(latch),
            shut: false,
        }
    }

    /// T-082 — the core refuses this write; disable unconditionally, latch or no latch.
    const fn refused(self) -> Self {
        Self { shut: true, ..self }
    }

    /// The values differ ⇒ there is a checkbox and the displayed value is blank.
    const fn differs(self) -> bool {
        self.opt.is_some()
    }

    /// The input is disabled: the core refuses the write, OR the values differ and the operator has
    /// not opted in. Reactive: call it from inside a view closure.
    fn locked(self) -> bool {
        self.shut || self.opt.is_some_and(|o| !o.get())
    }

    /// T-700 — the NON-reactive peer of [`locked`], for use inside an event handler.
    ///
    /// Same rule and, load-bearingly, the same ORDER: `shut` first, unconditional `||`, so the new
    /// keyboard path cannot become a second and laxer opinion of what "this field is dead" means.
    /// It reads the latch `_untracked` because a keydown is not a render — a tracked read there
    /// would subscribe whichever reactive owner happens to be current when the key is pressed.
    fn locked_now(self) -> bool {
        self.shut || self.opt.is_some_and(|o| !o.get_untracked())
    }
}

/// T-810 (F-23 c) — Eden's spatial-axis colour for a Transform field label, or `None` for every
/// other field. Eden colour-codes the Position/Rotation axes (X=red, Y=green, Z=blue) so an author
/// reads which line is which without reading the letter; TBD's tab was monochrome. The chip rides
/// the LABEL, not the value (per spec) — the value stays plain mono so a coordinate is never tinted.
///
/// Native and outside the wasm block for the same reason [`nudge_step`] is: it is a pure lookup, so
/// the mapping is pinned by CALLING it, and the colour set is checked against the plate rather than
/// asserted from a `view!` string. Returns a Tailwind background token; the four are deliberately at
/// the `-500` step, which reads with contrast on the modal's dark glass plate — the T-827 lesson
/// (measure against the ACTUAL surface, not paper). Rotation earns its own hue (amber) rather than
/// reusing an axis colour: it is not X/Y/Z, and giving it a fourth distinct chip keeps every
/// Transform row self-identifying.
fn axis_chip_class(label: &str) -> Option<&'static str> {
    match label {
        "X" => Some("bg-red-500"),
        "Y" => Some("bg-emerald-500"),
        "Z" => Some("bg-sky-500"),
        "Rotation" => Some("bg-amber-500"),
        _ => None,
    }
}

/// T-649 — the field label row: the field name plus, when the selection disagrees on this field,
/// the checkbox that opts it into the multi-apply.
///
/// This is a `<span>` rather than the field's old wrapping `<label>` because a `<label>` holding
/// two inputs implicitly labels only the first — the checkbox would have stolen the click that
/// should focus the field. The field input carries an explicit `aria-label` instead, and the
/// checkbox gets its own `<label>`.
///
/// T-810 (F-23 c) — a Transform axis label ([`axis_chip_class`]) leads with a small colour chip so
/// the row is identifiable by hue, Eden-style. `aria-hidden` on the dot: it is decoration, and the
/// axis is already named by the adjacent text and the input's `aria-label`, so a screen reader must
/// not announce a bare colour swatch.
#[cfg(target_arch = "wasm32")]
fn field_label(label: &'static str, gate: Gate) -> impl IntoView {
    view! {
        <span class="flex items-center justify-between gap-2 text-label-sm uppercase tracking-wider text-outline">
            <span class="flex items-center gap-1.5">
                {axis_chip_class(label)
                    .map(|c| {
                        view! {
                            <span
                                aria-hidden="true"
                                class=format!("inline-block size-2 shrink-0 rounded-full {c}")
                            ></span>
                        }
                    })}
                {label}
            </span>
            {gate
                .opt
                .map(|o| {
                    view! {
                        <label class="flex cursor-pointer items-center gap-1.5 normal-case tracking-normal text-primary">
                            <input
                                type="checkbox"
                                class="size-3.5 shrink-0 accent-primary"
                                aria-label=format!("Apply {label} to all selected")
                                // T-082 — a refused field's opt-in is inert; do not offer it.
                                disabled=gate.shut
                                prop:checked=move || o.get()
                                on:change=move |ev| o.set(event_target_checked(&ev))
                            />
                            <span>"Apply to all"</span>
                        </label>
                    }
                })}
        </span>
    }
}

/// T-741 — Attributes multi-edit header copy (wave-112 NIT-4).
///
/// Counts the **slot** subset the modal will write — never the full Ctrl+A selection — and says
/// explicitly when vehicles were excluded because
/// [`website_map_engine::editing::hosted_commands::attrs_multi_ids`] drops non-slot ids (vehicles
/// carry none of the SoA columns multi-edit stamps).
#[must_use]
pub(crate) fn attrs_multi_subtitle(slot_n: usize, selection_n: usize) -> String {
    let base = format!("{slot_n} slots selected · multi-edit");
    if selection_n > slot_n {
        format!("{base} · vehicles excluded")
    } else {
        base
    }
}

/* ══════════════ T-939.2 — batch faction / squad reassign: the pure decision ══════════════ */

/// The modal host. Renders nothing while closed (`attrs_open == None`) — V-capture-safe like the
/// suite Dialog. `doc_ver` is the re-read trigger (the doc has no change subscription).
#[component]
pub fn AttributesModal(
    attrs_open: RwSignal<Option<String>>,
    /// T-180.9 — tab index shared with EditorContext (`open_arsenal` sets 3 = Arsenal).
    attrs_tab: RwSignal<usize>,
    doc_tick: RwSignal<u64>,
    /// T-159.27 — flat registry gear rows for the Arsenal tab.
    registry_items: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
    /// T-167 — compat edge feed for the Smart Arsenal (optic/magazine rows + validation).
    compat: RwSignal<crate::v2::apps::editor::arsenal::rules::CompatFeed>,
) -> impl IntoView {
    // Esc closes (React Dialog behavior); the editor's own keydown handler skips editable fields,
    // so this window listener is the one Esc path.
    // T-726 — modal-stack gate; topmost consumes.
    #[cfg(target_arch = "wasm32")]
    {
        let modal_id = crate::v2::core::ui::modal_stack::register(move || {
            attrs_open.try_get_untracked().flatten().is_some()
        });
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if attrs_open.get_untracked().is_some()
                && ev.key() == "Escape"
                && crate::v2::core::ui::modal_stack::is_topmost_open(modal_id)
            {
                crate::v2::apps::editor::bridge::host_state::editor_context::close_attributes();
            }
        });
        on_cleanup(move || {
            esc.remove();
            crate::v2::core::ui::modal_stack::unregister(modal_id);
        });
    }
    // T-649 ATTR-MULTI-CHK-001 — the per-field opt-in latches, minted ONCE on the component (see
    // `MultiOpts`). The effect re-arms them whenever the modal's target changes — it tracks
    // `attrs_open` and deliberately NOT `doc_tick`, so a commit (which bumps `doc_tick`) leaves the
    // operator's ticks alone while a fresh open starts from a clean slate.
    let opts = MultiOpts::new();
    // T-810 (F-23 b) — the REVERT snapshot. Read-at-OPEN, restore-on-Revert (the T-082 lesson: the
    // modal must never re-derive "before" from a store it has since written — it captures the
    // pre-open values ONCE and holds them). Lives on the component like `opts`, refreshed by an
    // `attrs_open`-only effect so a live edit (which bumps `doc_tick`, not `attrs_open`) leaves the
    // snapshot alone — that is what makes Revert restore the state the panel opened on rather than
    // the last keystroke. WASM-ONLY because its element type (`SlotAttrs`) and the `editor_ops` reads
    // are; on native (the pin build) the component renders nothing, so nothing needs it there. One
    // entry per edited id (single-edit is one entry); vehicles are excluded upstream, so all slots.
    #[cfg(target_arch = "wasm32")]
    let snapshot: StoredValue<Vec<engine_ops::SlotAttrs>> = StoredValue::new(Vec::new());
    // The `opts` re-arm runs on both targets (native-safe); the snapshot capture is wasm-only.
    Effect::new(move |_| {
        let open = attrs_open.get();
        opts.reset();
        // Capture the pre-open values for the whole edited set. `attrs_multi_ids` returns the
        // multi-edit targets (empty ⇒ single-edit), so the snapshot set is `[open_id]` in the
        // single case and the slot subset in the multi case — the identical set the commits fan out
        // to and the identical set Revert will write back.
        #[cfg(target_arch = "wasm32")]
        {
            let snap = open
                .as_deref()
                .map(|id| {
                    let mut ids = engine_ops::attrs_multi_ids(id);
                    if ids.is_empty() {
                        ids = vec![id.to_string()];
                    }
                    ids.iter()
                        .filter_map(|i| engine_ops::read_attrs(i))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            snapshot.set_value(snap);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = open;
    });
    // T-167 / T-180.9 — tab lives on EditorContext (passed in) so `open_arsenal` can select Arsenal and
    // a doc change (loadout pick bumps `doc_tick`) no longer snaps back to Identity.
    move || {
        let id = attrs_open.get()?;
        let _ = doc_tick.get(); // re-read fields on every doc change (undo/redo/drag)
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (&id, registry_items, compat, attrs_tab, opts);
        #[cfg(target_arch = "wasm32")]
        {
            match engine_ops::read_attrs(&id) {
                Some(attrs) => {
                    // T-649 — the multi-edit target set (empty ⇒ the untouched single-slot modal)
                    // and which of its fields disagree. Both re-read per render, so a selection or
                    // doc change while the modal is open is reflected immediately.
                    let multi = engine_ops::attrs_multi_ids(&id);
                    // T-741 — full selection length (may include vehicles Ctrl+A picked up).
                    let selection_n = website_map_engine::editing::host::selection_len();
                    let diff = engine_ops::read_attrs_diff(&multi);
                    Some(modal_view(
                        attrs,
                        multi,
                        selection_n,
                        diff,
                        opts,
                        snapshot,
                        registry_items,
                        compat,
                        attrs_tab,
                    ))
                }
                None => {
                    // T-818 — vehicles open Attributes (T-647 ATTR-OPEN) but live off the slot SoA,
                    // so `read_attrs` is None. Route them to the vehicle editor rather than treating
                    // the id as undone-away. True absence (undone / deleted) still closes.
                    if engine_ops::is_vehicle_id(&id) {
                        Some(vehicle_attrs_view(id, registry_items))
                    } else {
                        // T-744 — `None` means the slot is GONE from the raw rows (undone / deleted),
                        // not merely hidden. Hide keeps `read_attrs` at `Some` (raw existence), so this
                        // arm is no longer reachable from H / layer-hide (wave-113 F-2).
                        crate::v2::apps::editor::bridge::host_state::editor_context::close_attributes();
                        None
                    }
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            None::<AnyView>
        }
    }
}

/// T-649 — `multi` is the multi-edit target id set; EMPTY means single-edit, and every field then
/// renders exactly as it did before this slice. `diff` says which fields the set disagrees on and
/// `opts` carries their opt-in latches.
// T-810 — the `snapshot` param (the Revert store) took this one over clippy's 7-arg threshold; the
// crate's idiom for a genuinely wide render seam is the explicit allow (see event_hub / missions).
#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
fn modal_view(
    attrs: engine_ops::SlotAttrs,
    multi: Vec<String>,
    selection_n: usize,
    diff: engine_ops::AttrDiff,
    opts: MultiOpts,
    // T-810 (F-23 b) — the pre-open snapshot the Revert button restores. Captured on open (see
    // `AttributesModal`), one entry per edited slot.
    snapshot: StoredValue<Vec<engine_ops::SlotAttrs>>,
    registry_items: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
    compat: RwSignal<crate::v2::apps::editor::arsenal::rules::CompatFeed>,
    tab: RwSignal<usize>,
) -> AnyView {
    let slot_id = StoredValue::new(attrs.id.clone());
    let is_multi = multi.len() > 1;
    let multi_n = multi.len();
    // T-649 — the commit target set. Single-edit is `[the open id]`, so the two modes differ only
    // in how many ids are in this vector, never in which code path runs.
    let targets = StoredValue::new(if is_multi {
        multi
    } else {
        vec![attrs.id.clone()]
    });
    // T-082 (wave-102 F-7) — asked ONCE per render, over the same id set the commits fan out to, so
    // the Transform tab's disabled state and the core's refusal are answers to the same question.
    // Re-asked on every `doc_tick` like every other value here, so unlocking the layer in the
    // Outliner re-enables the fields without closing the modal.
    let locked_n = engine_ops::attrs_locked_count(&targets.get_value());
    let attrs = StoredValue::new(attrs);
    let subtitle = {
        let a = attrs.get_value();
        if is_multi {
            // T-741 — slot subset + vehicles-excluded when the live selection is wider.
            attrs_multi_subtitle(multi_n, selection_n)
        } else {
            let role = if a.role.is_empty() {
                "Slot".to_string()
            } else {
                a.role.clone()
            };
            format!("{role} · {}", a.id)
        }
    };
    view! {
        <div
            class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
            on:click=move |_| crate::v2::apps::editor::bridge::host_state::editor_context::close_attributes()
        ></div>
        <div class=move || {
            // T-167 — the Smart Arsenal (tab 3) needs the wide 2-column doll layout; other tabs stay compact.
            // T-172 B10 — the Arsenal tab hosts the full Smart Forge (rail · list · 3D doll ·
            // compat panel), so it gets the widest modal tier.
            let width = if tab.get() == 3 { "max-w-6xl" } else { "max-w-lg" };
            format!("glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] {width} -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200")
        }>
            <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                <div class="min-w-0">
                    <h2 class="text-headline-sm text-on-surface">"Attributes"</h2>
                    <p class="mt-1 text-label-md text-on-surface-variant">{subtitle}</p>
                    // T-810 (F-23 b) — STATE THE MODEL. TBD applies every edit live with only a ✕ to
                    // close (no OK/Cancel — the gap the UX review named), so Revert is a bounded undo
                    // convenience, not a transaction boundary. One line, so an operator never mistakes
                    // it for "discard on close": edits are already saved; Revert re-writes the values
                    // this panel opened on.
                    <p class="mt-1 text-label-sm normal-case text-outline">
                        "Edits apply live. Revert restores the values from when this panel opened."
                    </p>
                </div>
                <div class="flex shrink-0 items-center gap-1">
                    // T-810 (F-23 b) — the Revert affordance. Restores the on-open snapshot across
                    // every edited slot (`revert_to_snapshot`). It writes real edits, so it is a plain
                    // button beside Close rather than a destructive dialog. `data-testid` so the
                    // scripted acceptance can drive it.
                    <button
                        type="button"
                        data-testid="attrs-revert"
                        on:click=move |_| revert_to_snapshot(snapshot)
                        class="rounded-md border border-outline-variant/40 px-2.5 py-1 text-label-sm text-on-surface-variant transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                    >
                        "Revert"
                    </button>
                    <button
                        type="button"
                        aria-label="Close"
                        on:click=move |_| crate::v2::apps::editor::bridge::host_state::editor_context::close_attributes()
                        class="rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                    >
                        <crate::v2::core::ui::MaterialIcon name="close" />
                    </button>
                </div>
            </div>
            <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                <div class="flex flex-col gap-4">
                    // T-649 ATTR-MULTI-CHK-001 — the multi-edit banner. It states the rule the
                    // checkboxes implement, so a disabled field is never mistaken for a broken one.
                    {(is_multi && diff.any())
                        .then(|| {
                            view! {
                                <p class="rounded-md border border-primary/30 bg-primary/10 px-3 py-2 text-label-sm normal-case text-on-surface-variant">
                                    "Fields that differ across the selection are blank and locked. Tick "
                                    <span class="text-primary">"Apply to all"</span>
                                    " to overwrite that field on every selected slot."
                                </p>
                            }
                        })}
                    <div class="flex gap-1 rounded-lg bg-surface-container-lowest/50 p-1">
                        {TABS
                            .iter()
                            .enumerate()
                            .map(|(i, label)| {
                                view! {
                                    <button
                                        type="button"
                                        aria-label=*label
                                        on:click=move |_| tab.set(i)
                                        class=move || {
                                            if tab.get() == i {
                                                "flex-1 rounded-md px-2 py-1.5 text-label-md transition-colors bg-primary/20 text-primary"
                                            } else {
                                                "flex-1 rounded-md px-2 py-1.5 text-label-md transition-colors text-on-surface-variant hover:bg-white/5"
                                            }
                                        }
                                    >
                                        {*label}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                    {move || match tab.get() {
                        0 => transform_tab(targets, attrs, is_multi, diff, opts, locked_n)
                            .into_any(),
                        1 => identity_tab(targets, attrs, is_multi, diff, opts, registry_items)
                            .into_any(),
                        2 => states_tab().into_any(),
                        _ => {
                            let loadout = engine_ops::read_loadout(&slot_id.get_value());
                            view! {
                                // T-649 / T-771 — HONESTY BANNER. Inverting the `open_arsenal` guard
                                // is what stops the context menu's "Edit Loadout..." row being
                                // enabled-but-inert on a multi-selection: the modal opens now. Pick
                                // and cargo rows still edit ONE slot; T-699's Copy / Apply / Remove
                                // Everything act on the WHOLE selection. Say both, rather than let
                                // the multi-edit "N slots selected" header (or a one-sided claim) mislead.
                                {is_multi
                                    .then(|| {
                                        view! {
                                            <p class="mb-3 rounded-md border border-outline-variant/40 bg-surface-container-lowest/50 px-3 py-2 text-label-sm normal-case text-on-surface-variant">
                                                "Pick and cargo edits apply to this one entity ("
                                                {slot_id.get_value()}
                                                "); Copy, Apply, and Remove Everything act on the whole selection."
                                            </p>
                                        }
                                    })}
                                <crate::v2::apps::editor::arsenal::ArsenalTab
                                    slot_id=slot_id.get_value()
                                    loadout_json=loadout
                                    registry=registry_items
                                    compat=compat
                                />
                            }
                            .into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
    .into_any()
}

/* ─────────────────────────── T-818 — vehicle Attributes (moved from DockRight Placed strip) ── */

/// T-076 — the **generic** seat model shipped ahead of a per-class seat schema (copied with the
/// controls from `eden_vehicles_panel` so the Attributes call site owns the same seat_ids the
/// strip authored into `vehicle.crew`). `(seat_id, label)`; cargo seats append as `cargoN`.
const FIXED_SEATS: &[(&str, &str)] = &[
    ("driver", "Driver"),
    ("gunner", "Gunner"),
    ("commander", "Commander"),
];

/// T-076 — cargo seats offered when the vehicle has no declared cargo capacity.
const DEFAULT_CARGO_SEATS: usize = 4;

/// T-076 — ordered `(seat_id, label)` list: three fixed stations then `n_cargo` cargo seats.
fn seat_model(n_cargo: usize) -> Vec<(String, String)> {
    FIXED_SEATS
        .iter()
        .map(|(id, label)| ((*id).to_string(), (*label).to_string()))
        .chain((1..=n_cargo).map(|n| (format!("cargo{n}"), format!("Cargo {n}"))))
        .collect()
}

/// T-215 — registry kinds the vehicle cargo picker offers (same allow-list the Placed strip used).
const VEHICLE_CARGO_KINDS: &[&str] = &[
    "magazine",
    "ammo",
    "gear_item",
    "gear_throwable",
    "gear_explosive",
    "gear_primary",
    "gear_handgun",
    "gear_launcher",
    "gear_binoculars",
    "gear_vest",
    "gear_armored_vest",
    "gear_backpack",
    "gear_helmet",
    "gear_jacket",
    "gear_pants",
    "gear_boots",
    "gear_gloves",
    "gear_glasses",
    "optic",
    "attachment",
    "crate",
];

/// T-818 — vehicle Attributes body: Heading° / Add cargo / Crew dropdowns MOVED from the right-dock
/// Placed strip (not redesigned). Same mutators (`set_vehicle_heading` / `set_vehicle_cargo` /
/// `assign_crew_seat` / `clear_crew_seat`) so digest + undo shape stay identical. Vehicles are
/// single-edit — the T-649/T-788 multi-edit machinery is untouched. Heading commits through
/// [`number_field`] (T-785 focused/draft, blur/Enter).
#[cfg(target_arch = "wasm32")]
fn vehicle_attrs_view(
    id: String,
    registry_items: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
) -> AnyView {
    use std::collections::HashMap;
    use website_map_engine::editing::hosted_commands::VehicleCargoRow;

    let Some(v) = engine_ops::vehicle_rows().into_iter().find(|r| r.id == id) else {
        // Race: id was a vehicle at the host gate, then vanished before this render.
        crate::v2::apps::editor::bridge::host_state::editor_context::close_attributes();
        return ().into_any();
    };

    let items = registry_items.get().unwrap_or_default();
    let names: HashMap<String, String> = items
        .iter()
        .map(|i| (i.resource_name.clone(), i.display_name.clone()))
        .collect();
    let title = names
        .get(&v.resource_name)
        .cloned()
        .unwrap_or_else(|| v.resource_name.clone());
    let mut addable: Vec<(String, String)> = items
        .iter()
        .filter(|i| VEHICLE_CARGO_KINDS.contains(&i.kind.as_str()))
        .filter(|i| !i.r#abstract.unwrap_or(false))
        .map(|i| (i.resource_name.clone(), i.display_name.clone()))
        .collect();
    addable.sort_by(|a, b| a.1.cmp(&b.1));
    let addable = StoredValue::new(addable);
    let names = StoredValue::new(names);
    let label_of =
        move |rn: &str| names.with_value(|n| n.get(rn).cloned().unwrap_or_else(|| rn.to_string()));

    let vid = v.id.clone();
    let subtitle = format!("{title} · {vid}");
    let heading = v.rotation;
    let cargo = v.cargo.clone();
    let crew = v.crew.clone();

    // Heading° — number_field (T-785), not the strip's raw on:change input.
    // `on_commit` must be `Copy` (number_field's bound) — stash the id in a StoredValue like
    // Transform's `targets`, never capture a String by move.
    let heading_row = if let Some(h) = heading {
        let id_h = StoredValue::new(vid.clone());
        number_field("Heading", h, Some("°"), Gate::open(), move |raw| {
            let deg = ((raw % 360.0) + 360.0) % 360.0;
            engine_ops::set_vehicle_heading(id_h.get_value(), deg);
        })
        .into_any()
    } else {
        ().into_any()
    };

    let rows_for_edit = cargo.clone();
    let id_add = vid.clone();
    let cargo_rows = cargo
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let label = label_of(&row.item);
            let (base_q, base_r) = (rows_for_edit.clone(), rows_for_edit.clone());
            let (id_q, id_r) = (vid.clone(), vid.clone());
            view! {
                <div class="flex items-center gap-1.5 py-0.5">
                    <span class="min-w-0 flex-1 truncate text-label-sm text-on-surface-variant">
                        {label}
                    </span>
                    <input
                        type="number"
                        min="1"
                        aria-label="Quantity"
                        class="w-14 shrink-0 rounded border border-outline-variant/40 bg-surface-container-lowest/60 px-1 py-0.5 text-right font-mono text-label-sm tabular-nums text-on-surface outline-none focus:border-primary/60"
                        prop:value=row.qty.to_string()
                        on:change=move |ev| {
                            let Ok(q) = event_target_value(&ev).trim().parse::<i64>() else {
                                return;
                            };
                            let mut next = base_q.clone();
                            if let Some(r) = next.get_mut(i) {
                                r.qty = q;
                            }
                            engine_ops::set_vehicle_cargo(id_q.clone(), next);
                        }
                    />
                    <button
                        type="button"
                        aria-label="Remove cargo row"
                        class="shrink-0 rounded p-0.5 text-on-surface-variant hover:text-error-alert"
                        on:click=move |_| {
                            let mut next = base_r.clone();
                            if i < next.len() {
                                next.remove(i);
                            }
                            engine_ops::set_vehicle_cargo(id_r.clone(), next);
                        }
                    >
                        <crate::v2::core::ui::MaterialIcon name="close" class="block text-sm" />
                    </button>
                </div>
            }
        })
        .collect_view();

    let seat_choices = StoredValue::new(engine_ops::placed_slot_choices());
    let n_cargo_seats = DEFAULT_CARGO_SEATS;
    let seat_list = seat_model(n_cargo_seats)
        .into_iter()
        .map(|(seat_id, seat_label)| {
            let occupant = crew.get(&seat_id).cloned().unwrap_or_default();
            let id_seat = vid.clone();
            let sid = seat_id.clone();
            view! {
                <div class="flex items-center gap-1.5 py-0.5">
                    <span class="w-16 shrink-0 text-label-sm text-on-surface-variant">
                        {seat_label}
                    </span>
                    <select
                        aria-label=format!("Assign {seat_id}")
                        class="min-w-0 flex-1 rounded border border-outline-variant/40 bg-surface-container-lowest/60 px-1.5 py-0.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
                        prop:value=occupant.clone()
                        on:change=move |ev| {
                            let slot = event_target_value(&ev);
                            if slot.is_empty() {
                                engine_ops::clear_crew_seat(id_seat.clone(), sid.clone());
                            } else {
                                engine_ops::assign_crew_seat(
                                    id_seat.clone(),
                                    sid.clone(),
                                    slot,
                                );
                            }
                        }
                    >
                        <option value="" selected=occupant.is_empty()>
                            "— empty —"
                        </option>
                        {seat_choices
                            .get_value()
                            .into_iter()
                            .map(|choice| {
                                let is_sel = choice.id == occupant;
                                view! {
                                    <option value=choice.id.clone() selected=is_sel>
                                        {choice.label}
                                    </option>
                                }
                            })
                            .collect_view()}
                    </select>
                </div>
            }
        })
        .collect_view();

    let base_add = rows_for_edit;
    view! {
        <div
            class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
            on:click=move |_| crate::v2::apps::editor::bridge::host_state::editor_context::close_attributes()
        ></div>
        <div class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200">
            <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                <div class="min-w-0">
                    <h2 class="text-headline-sm text-on-surface">"Attributes"</h2>
                    <p class="mt-1 text-label-md text-on-surface-variant">{subtitle}</p>
                    <p class="mt-1 text-label-sm normal-case text-outline">
                        "Edits apply live."
                    </p>
                </div>
                <button
                    type="button"
                    aria-label="Close"
                    on:click=move |_| crate::v2::apps::editor::bridge::host_state::editor_context::close_attributes()
                    class="rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                >
                    <crate::v2::core::ui::MaterialIcon name="close" />
                </button>
            </div>
            <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                <div class="flex flex-col gap-4">
                    {heading_row}
                    <div class="flex flex-col gap-1">
                        <div class="flex items-center gap-1.5">
                            <crate::v2::core::ui::MaterialIcon
                                name="inventory_2"
                                class="block shrink-0 text-sm text-outline"
                            />
                            <span class="text-label-sm font-semibold text-on-surface-variant">
                                "Cargo"
                            </span>
                        </div>
                        {cargo_rows}
                        <select
                            aria-label="Add cargo"
                            class="w-full rounded border border-outline-variant/40 bg-surface-container-lowest/60 px-1.5 py-0.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
                            on:change=move |ev| {
                                let item = event_target_value(&ev);
                                if item.is_empty() {
                                    return;
                                }
                                let mut next = base_add.clone();
                                if let Some(r) = next.iter_mut().find(|r| r.item == item) {
                                    r.qty = r.qty.saturating_add(1);
                                } else {
                                    next.push(VehicleCargoRow { item, qty: 1 });
                                }
                                engine_ops::set_vehicle_cargo(id_add.clone(), next);
                            }
                        >
                            <option value="">"Add cargo…"</option>
                            {addable
                                .get_value()
                                .into_iter()
                                .map(|(rn, label)| view! { <option value=rn>{label}</option> })
                                .collect_view()}
                        </select>
                    </div>
                    <div class="flex flex-col gap-1">
                        <div class="flex items-center gap-1.5">
                            <crate::v2::core::ui::MaterialIcon
                                name="group"
                                class="block shrink-0 text-sm text-outline"
                            />
                            <span class="text-label-sm font-semibold text-on-surface-variant">
                                "Crew"
                            </span>
                        </div>
                        {seat_list}
                    </div>
                </div>
            </div>
        </div>
    }
    .into_any()
}

/* ─────────────────────────── field primitives (fields.tsx ports) ─────────────────────────── */

/* ─────────── T-700 3DEN-PLACE-013 — the numeric nudge, as arithmetic ───────────
 *
 * Eden gives every numeric field a keyboard nudge and scales the step with the modifier keys.
 * [`number_field`] had NO keyboard affordance of its own at all: `type="number"` buys the browser's
 * ±1 arrow keys and nothing else, and PageUp/PageDown just scrolled the modal.
 *
 * **wave-127 F-1 — and the browser's own arrow keys were a PRECISION BUG, not an affordance.** A
 * `type="number"` input with no `step` gets the default `step=1` on step base `0`, and the WHATWG
 * "step up" algorithm does not add the step to the current value: when the value is off the step
 * grid it SNAPS to the grid first. One ArrowUp on a focused field holding `412.37` therefore set the
 * DOM value to `413`, fired `input`, and blur committed the integer — the exact defect T-775 shipped
 * to remove, alive on the key next to the one it fixed. Both halves of the fix are below: the input
 * carries `step="any"` (no grid, so a stray arrow can only ever move by a whole step from where the
 * value already is), and the handler CLAIMS ArrowUp/ArrowDown into the same `nudged()` path as
 * PageUp/PageDown so the modifier scale is the same on every nudge key.
 *
 * The two functions below are deliberately OUTSIDE the `#[cfg(target_arch = "wasm32")]` block that
 * holds the rest of this modal. Everything in a `view!` tree is unreachable from `cargo test`
 * (native) and therefore can only be pinned against its own source; the nudge's decisions —
 * how big a step is, and when there is no legal nudge at all — are pure arithmetic, so they are
 * kept native and tested by CALLING them. The keydown handler is then a thin wire between the two.
 */

/// The step one nudge key (PageUp/PageDown, ArrowUp/ArrowDown) moves a numeric field, given the
/// modifier keys.
///
/// | held    | step |
/// |---------|------|
/// | `Ctrl`  | 0.1  |
/// | `Shift` | 10   |
/// | `Alt`   | 100  |
/// | none    | 1    |
///
/// FIRST MATCH, finest first — not a product of the three. A multiplicative scale has to answer
/// "what is Ctrl+Alt?" and every answer is a surprise; first-match answers it once. Answering it
/// with the FINEST modifier held is the safe direction: a two-finger combo the operator did not
/// mean can then only ever nudge LESS than intended. Overshooting by 1000 is an edit to hunt down
/// and undo; undershooting is one more keypress.
fn nudge_step(ctrl: bool, shift: bool, alt: bool) -> f64 {
    if ctrl {
        0.1
    } else if shift {
        10.0
    } else if alt {
        100.0
    } else {
        1.0
    }
}

/// `from` moved by one `step` (`up` = PageUp/ArrowUp). `None` means **there is no legal nudge**, and the
/// caller must write nothing at all.
///
/// `from` is `None` whenever the field's text does not parse, and the case that matters is the
/// multi-edit one: a field whose selected slots DISAGREE renders EMPTY by [`Gate::differs`], so
/// there is no base value to be relative to. Nudging from an implied `0` would stamp an ABSOLUTE
/// number onto every selected entity while looking to the operator like a relative tweak — so a
/// differing field refuses the nudge until an absolute value is typed into it.
///
/// The result is quantised to 3 decimals because the finest step is 0.1 and binary floats do not
/// add that cleanly: ten Ctrl+PageUps off zero land on `0.9999999999999999`, and that is the string
/// the field would then display and commit.
fn nudged(from: Option<f64>, up: bool, step: f64) -> Option<f64> {
    let from = from.filter(|v| v.is_finite())?;
    let raw = from + if up { step } else { -step };
    let quantised = (raw * 1000.0).round() / 1000.0;
    quantised.is_finite().then_some(quantised)
}

/// **T-775** — what an UNFOCUSED [`number_field`] shows. Presentation only; nothing commits this.
///
/// It used to be `format!("{}", value.round())`, and that single `.round()` was the whole defect:
/// an entity dragged to `x = 412.37` read `412`, the draft seeded from that string, and blur wrote
/// the `412` back over the authored number. The fix is split in two — the field now edits the EXACT
/// value (see `number_field`'s `exact`) and only ROUNDS FOR PRESENTATION here, which is the shape
/// the T-775 spec asked for.
///
/// Three decimals, because that is the resolution the rest of the editor already works at: it is the
/// quantum [`nudged`] snaps every keyboard step onto, and the precision `eden_toolbelt::fmt_coord`
/// prints the cursor readout with. Trailing zeros are trimmed so a whole coordinate still reads
/// `412` rather than `412.000` — the tidiness the old `.round()` was reaching for, without the lie
/// about the integer part.
///
/// Bare `format!("{value}")` is deliberately NOT used for display: it prints the shortest string
/// that round-trips, and a drag-derived f64 (nothing snaps a drag to a grid) can need 17 characters
/// of it. Three of those in the Transform tab's `grid-cols-3` would overflow every field. The exact
/// string is not lost — it is what focus puts in the draft, so the operator sees full precision on
/// the one screen where it is actionable: the one they are editing.
fn field_display(value: f64) -> String {
    if !value.is_finite() {
        return format!("{value}");
    }
    let s = format!("{value:.3}");
    let t = s.trim_end_matches('0').trim_end_matches('.');
    // `-0.0001` formats as `-0.000` and trims to `-0`; a field claiming a negative zero is noise.
    if t == "-0" {
        "0".to_string()
    } else {
        t.to_string()
    }
}

/// **T-775** — does a settled [`number_field`] draft deserve a write? `n` is the parsed draft,
/// `settled` the value the field was showing, `differs` the multi-edit gate's verdict.
///
/// Three rules, and each one is a defect that shipped or was one refactor away:
///   * a NON-FINITE parse writes nothing. `"inf"` and `"NaN"` both parse as `f64` and would sail
///     into the document; the core filters them per-axis, but a refused write still fires
///     `after_local_edit()` at the caller, so the mission goes dirty for a number that never landed.
///   * an UNCHANGED value writes nothing — the T-775 defect itself. Nothing downstream asks "did
///     anything change": `engine_ops::attrs_update_position` writes `position` and calls
///     `after_local_edit()` on any non-refused slot, so a focus/blur on an untouched coordinate
///     dirties the mission, arms a persist and mints an undo step for an edit the operator never
///     made. The comparison is against the EXACT settled value, never the rounded presentation
///     string — comparing against the display is how the rounding kept leaking into the document.
///   * a DIFFERING field is EXEMPT from the equality skip. Under a multi-selection `settled` is one
///     arbitrary member's number, so typing that number is a deliberate stamp onto the whole
///     selection and must commit even though it "equals" the value shown.
///
/// **wave-127 F-3** — this is a function, and native, because the decision was previously an inline
/// expression inside a `#[cfg(target_arch = "wasm32")]` `view!` closure, guarded by nothing but a
/// source pin on its literal text. There is no wasm-bindgen-test harness in this repo, so NOTHING
/// executed it: a correct refactor (inverting the condition) turned the pin red while a subtly wrong
/// rewrite that kept the string shape stayed green. The pin now only checks that `commit` CALLS this;
/// the behaviour is tested by calling it.
fn should_commit(differs: bool, n: f64, settled: f64) -> bool {
    n.is_finite() && (differs || n != settled)
}

/// Mono numeric field committing on blur/Enter (one commit = one undo step). While focused it holds
/// the local draft, seeded from the EXACT doc value; unfocused it mirrors the doc value at
/// presentation precision ([`field_display`]), so a map drag updates it live.
///
/// T-649 — `gate` is the multi-edit gate. When it reports `differs()` there is no single truthful
/// value to display, so the field shows EMPTY (placeholder `—`) rather than one arbitrary member's
/// number, and stays `disabled` until the "Apply to all" checkbox is ticked. The commit path is
/// untouched: whatever the operator types is parsed and handed to `on_commit` exactly as before.
///
/// **T-700 3DEN-PLACE-013 — the keyboard nudge, and why a burst COALESCES.** PageUp/PageDown and
/// (wave-127 F-1) ArrowUp/ArrowDown move
/// the value by [`nudge_step`]; a nudge writes the local **draft** and nothing else, so a run of
/// them settles into the ONE commit that blur/Enter already fires. It is exactly what typing does,
/// and it is that way for two concrete reasons rather than taste:
///   * `attrs_update_position` calls `after_local_edit()` per commit and `MissionDocCore` builds its
///     `UndoManager` with `capture_timeout_millis = 0`, so a per-nudge commit would mint one undo
///     step per keypress — and PageDown auto-repeats. Ten held keys would be ten Ctrl-Zs.
///   * the modal body re-renders on every `doc_tick` bump (see `AttributesModal`), and a commit
///     bumps it. Committing mid-focus would rebuild this very input under the operator's fingers
///     and drop the focus that the next nudge needs, so the second PageUp would land nowhere.
/// The cost is stated honestly: the entity does not move on the map until the field settles, the
/// same as typing a coordinate. The trade is one undo step per visit instead of one per keypress.
///
/// A nudge is a WRITE, so it takes the same gate the typed path takes ([`Gate::locked_now`]) — a
/// T-082 refused field and an un-ticked "Apply to all" both refuse the keyboard exactly as they
/// refuse the keyboard's typed characters.
///
/// **T-775 — the nudge steps from the EXACT value.** It reads the draft, and the draft is seeded on
/// focus from `exact`. When that seed was the rounded display string, PageUp on `412.37` committed
/// `413` rather than `413.37`: the nudge inherited a rounding it never performed. Fixing the seed
/// fixed the nudge, which is why T-700's note and T-775's fix live in the same function.
#[cfg(target_arch = "wasm32")]
fn number_field(
    label: &'static str,
    value: f64,
    suffix: Option<&'static str>,
    gate: Gate,
    on_commit: impl Fn(f64) + Copy + 'static,
) -> impl IntoView {
    let draft = RwSignal::new(String::new());
    let focused = RwSignal::new(false);
    // T-775 — TWO strings, and the split is the fix. `shown` is presentation (see
    // [`field_display`]); `exact` is the value the field actually EDITS, printed at full
    // round-trip precision so that focusing and leaving a field is a genuine no-op.
    let shown = StoredValue::new(field_display(value));
    let exact = StoredValue::new(format!("{value}"));
    // A differing field starts from an EMPTY draft — pre-filling one member's value would make an
    // accidental blur write that member's number onto the whole selection.
    let display = move || {
        if gate.differs() {
            String::new()
        } else {
            shown.get_value()
        }
    };
    let seed = move || {
        if gate.differs() {
            String::new()
        } else {
            exact.get_value()
        }
    };
    let commit = move || {
        focused.set(false);
        if let Ok(n) = draft.get_untracked().parse::<f64>() {
            // T-775 — AN IDLE FOCUS/BLUR IS NOT AN EDIT. The whole decision (and every reason for
            // each of its three rules) lives in [`should_commit`], which is native and therefore
            // actually tested; this is the wire. wave-127 F-2 removed one of the reasons the skip
            // used to carry — an x/y commit no longer flattens a manually authored Z, because
            // `engine_ops::attrs_update_position` now passes the slot's current z back in — but the
            // dirty mission, the armed persist and the undo step for an untouched number remain.
            if should_commit(gate.differs(), n, value) {
                on_commit(n);
            }
        }
    };
    view! {
        <div class="flex flex-col gap-1">
            {field_label(label, gate)}
            <div class="relative">
                <input
                    type="number"
                    // wave-127 F-1 — `step="any"` is a PRECISION guard, not styling. Without it the
                    // input carries the default `step=1` on step base `0`, and the browser's own
                    // arrow keys / spinner run the WHATWG step-up algorithm, which SNAPS an off-grid
                    // value onto the grid: one ArrowUp on `412.37` writes `413`, not `413.37`. With
                    // `step="any"` there is no grid to snap to. The handler below also claims the
                    // arrow keys outright — belt and braces, because this attribute is the only
                    // thing standing between the spinner buttons and an authored coordinate.
                    step="any"
                    aria-label=label
                    disabled=move || gate.locked()
                    placeholder=if gate.differs() { "—" } else { "" }
                    prop:value=move || { if focused.get() { draft.get() } else { display() } }
                    on:focus=move |_| {
                        draft.set(seed());
                        focused.set(true);
                    }
                    on:input=move |ev| draft.set(event_target_value(&ev))
                    on:blur=move |_| commit()
                    on:keydown=move |ev| {
                        let key = ev.key();
                        // T-813 / wave200 F6 — field Escape abandons the draft and consumes so the
                        // modal window listener does not close on the same press (same family as
                        // text_field).
                        if key == "Escape" {
                            ev.stop_propagation();
                            focused.set(false);
                            draft.set(seed());
                            if let Some(t) = ev
                                .target()
                                .and_then(|t| {
                                    wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(t).ok()
                                })
                            {
                                t.blur().ok();
                            }
                            return;
                        }
                        if key == "Enter" {
                            if let Some(t) = ev
                                .target()
                                .and_then(|t| {
                                    wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(t).ok()
                                })
                            {
                                t.blur().ok();
                            }
                            return;
                        }
                        // T-700 3DEN-PLACE-013 — the nudge. wave-127 F-1 added the ARROW keys: the
                        // browser's native stepping on those was a rounding bug (see `step="any"`
                        // above), and taking them here also makes the modifier scale identical on
                        // every key that moves the number.
                        let up = match key.as_str() {
                            "PageUp" | "ArrowUp" => true,
                            "PageDown" | "ArrowDown" => false,
                            _ => return,
                        };
                        // Claimed unconditionally, before any refusal below: whether or not this
                        // field accepts the nudge, the operator asked to move a NUMBER, and the
                        // default action is to scroll the modal out from under them (PageUp/Down) or
                        // to snap the value onto the step grid (the arrows).
                        ev.prevent_default();
                        // T-082 — the core drops a refused field's write on the floor, and an
                        // un-ticked latch is an operator who has not opted this column into the
                        // multi-edit. `disabled` already stops the event in a real browser; this is
                        // the same rule stated where it does not depend on the browser to hold.
                        if gate.locked_now() {
                            return;
                        }
                        let Some(next) = nudged(
                            draft.get_untracked().parse::<f64>().ok(),
                            up,
                            nudge_step(ev.ctrl_key(), ev.shift_key(), ev.alt_key()),
                        ) else {
                            return;
                        };
                        // Draft only — the burst settles into blur/Enter's single commit.
                        draft.set(format!("{next}"));
                    }
                    class=move || {
                        let pad = if suffix.is_some() { " pr-7" } else { "" };
                        let lock = if gate.locked() { CONTROL_LOCKED } else { "" };
                        format!("{CONTROL} font-mono{pad}{lock}")
                    }
                />
                {suffix
                    .map(|s| {
                        view! {
                            <span class="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 font-mono text-label-sm text-outline">
                                {s}
                            </span>
                        }
                    })}
            </div>
        </div>
    }
}

/// Text field committing on **blur/Enter**, holding a local draft while focused — the
/// [`number_field`] focused/draft split, ported to a `String`.
///
/// **T-785 — why this is NOT a per-keystroke commit any more.** It used to be
/// `on:input=move |ev| on_change(...)`, one commit per character. Each commit runs
/// `engine_ops::attrs_update_*` → `after_local_edit()`, which bumps `doc_tick`; the whole
/// `AttributesModal` body re-reads on every `doc_tick` (see [`AttributesModal`]) and Leptos
/// therefore RE-CREATED this very `<input>` between keystrokes. The DOM node the operator was
/// typing into was destroyed after character one, focus fell to `<body>`, and every following
/// character reached the window-level keydown shortcuts as a chord: typing "AT Rifleman" into ROLE
/// left the field holding "A" while `T`/`R`/`e`/Space/`G` collapsed docks, jumped the camera and
/// flipped snap. `number_field` already avoided this the only way that works — commit when focus
/// LEAVES, not while it is held (its own note calls out that a mid-focus commit "would rebuild this
/// very input under the operator's fingers"). This is that pattern for text.
///
/// The trade is the same one `number_field` makes and states: one undo step per visit instead of
/// one per keystroke, and the map/tree do not reflect the edit until the field settles.
///
/// T-649 — same `gate` contract as [`number_field`]: a field the selection disagrees on renders
/// empty with a "Multiple values" placeholder and is disabled until its checkbox is ticked. A
/// differing field also starts from an EMPTY draft, so an accidental blur cannot stamp one member's
/// text onto the whole selection.
#[cfg(target_arch = "wasm32")]
fn text_field(
    label: &'static str,
    value: String,
    placeholder: &'static str,
    gate: Gate,
    on_change: impl Fn(String) + Copy + 'static,
) -> impl IntoView {
    let draft = RwSignal::new(String::new());
    let focused = RwSignal::new(false);
    // The doc value at render time. The modal re-invokes `text_field` with a fresh `value` on every
    // `doc_tick`, so an unfocused field tracks undo/redo and external edits by simply re-rendering;
    // `StoredValue` keeps that snapshot for the unfocused display without making it a fresh input.
    let settled = StoredValue::new(value);
    let ph = if gate.differs() {
        "Multiple values"
    } else {
        placeholder
    };
    // Unfocused presentation: EMPTY when the selection disagrees (never one arbitrary member's
    // string), else the settled doc value. The draft seeds from the same source on focus. Named
    // `text_display` (not the bare `display` `number_field` uses) so the source pins can address
    // this closure unambiguously — two `let display = move ||` in one file would be a shadow the
    // `class_r_scrub` `only_body` extractor refuses to disambiguate.
    let text_display = move || {
        if gate.differs() {
            String::new()
        } else {
            settled.get_value()
        }
    };
    // T-813 — operator-edited latch. A differing field seeds from "" (`text_display`), so the
    // old `gate.differs()` exemption alone stamped that empty draft across every selected slot on a
    // pure focus+blur (wave200 F3). The latch is set only on real `input`; Escape clears it so the
    // blur that follows an abandon cannot write either.
    let edited = RwSignal::new(false);
    let text_commit = move || {
        focused.set(false);
        // Skip the write when the operator did not edit — a focus/blur on an untouched field must
        // not dirty the mission or mint an undo step, INCLUDING when the field differs. A deliberate
        // type (including re-typing one member's value under multi-edit) still stamps because the
        // latch is set on real input, and `gate.differs()` still exempts the settled-equality skip.
        if !edited.get_untracked() {
            return;
        }
        let next = draft.get_untracked();
        if gate.differs() || next != settled.get_value() {
            on_change(next);
        }
    };
    view! {
        <div class="flex flex-col gap-1">
            {field_label(label, gate)}
            <input
                type="text"
                aria-label=label
                disabled=move || gate.locked()
                placeholder=ph
                // While focused the field shows the LOCAL draft — never a value that round-tripped
                // through the store mid-edit, which is the remount `on:input` used to cause.
                prop:value=move || { if focused.get() { draft.get() } else { text_display() } }
                on:focus=move |_| {
                    draft.set(text_display());
                    edited.set(false);
                    focused.set(true);
                }
                on:input=move |ev| {
                    edited.set(true);
                    draft.set(event_target_value(&ev));
                }
                on:blur=move |_| text_commit()
                on:keydown=move |ev| {
                    // Enter commits by blurring — the ONE commit seam, shared with the blur path so
                    // there is exactly one place text reaches the store. Escape abandons the draft
                    // and CONSUMES the event (wave200 F6): first press abandons+blurs, second closes
                    // the modal — without stop_propagation the modal window listener closes both.
                    match ev.key().as_str() {
                        "Enter" => {
                            if let Some(t) = ev
                                .target()
                                .and_then(|t| {
                                    wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(t).ok()
                                })
                            {
                                t.blur().ok();
                            }
                        }
                        "Escape" => {
                            ev.stop_propagation();
                            edited.set(false);
                            focused.set(false);
                            draft.set(text_display());
                            if let Some(t) = ev
                                .target()
                                .and_then(|t| {
                                    wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(t).ok()
                                })
                            {
                                t.blur().ok();
                            }
                        }
                        _ => {}
                    }
                }
                                class=move || {
                    let lock = if gate.locked() { CONTROL_LOCKED } else { "" };
                    format!("{CONTROL}{lock}")
                }
            />
        </div>
    }
}

/* ─────────────────────────── tabs ─────────────────────────── */

/// T-649 — the ONE commit seam for Transform. A single selection keeps the exact original
/// single-slot call so nothing about the pre-T-649 path moved; a multi-selection routes to the
/// `_multi` peer, which applies the same per-field `Option`s to every target under one history
/// tail. `None` fields are never written, so opting one field in cannot drag the others along.
#[cfg(target_arch = "wasm32")]
fn commit_position(
    targets: StoredValue<Vec<String>>,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    rotation: Option<f64>,
) {
    let ids = targets.get_value();
    if ids.len() > 1 {
        engine_ops::attrs_update_position_multi(&ids, x, y, z, rotation);
    } else if let Some(id) = ids.first() {
        engine_ops::attrs_update_position(id, x, y, z, rotation);
    }
}

/// T-649 — the Identity/stance peer of [`commit_position`]; same single vs multi split.
///
/// T-082 — widened by exactly two `Option`s (`asset_id` = ATTR-FIELD-OBJ-TYPE, `description` =
/// ATTR-FIELD-OBJ-ROLE-DESC) so the new fields go through the ONE commit seam rather than around
/// it. Every caller passes `Some` for exactly the field it edits and `None` for the rest, which is
/// what makes the per-field multi-edit opt-in mean anything.
#[cfg(target_arch = "wasm32")]
fn commit_slot(
    targets: StoredValue<Vec<String>>,
    role: Option<String>,
    tag: Option<String>,
    stance: Option<String>,
    asset_id: Option<String>,
    description: Option<String>,
) {
    let ids = targets.get_value();
    if ids.len() > 1 {
        engine_ops::attrs_update_slot_multi(&ids, role, tag, stance, asset_id, description);
    } else if let Some(id) = ids.first() {
        engine_ops::attrs_update_slot(id, role, tag, stance, asset_id, description);
    }
}

/// T-810 (F-23 b) — restore every edited slot to the values captured when the panel opened.
///
/// **Why PER-SLOT single-ops and not the T-788 homogeneous batch.** The registry summary points at
/// `update_slots_attr_batch` for a multi-slot revert, and that batch is the right tool for an
/// apply-to-all EDIT — it stamps ONE value onto every id in one txn. A Revert is the opposite shape:
/// the values it writes are each slot's OWN pre-open value, and under a multi-selection those differ
/// (a field that DIFFERED on open was blank-and-locked until "Apply to all" was ticked, and undoing
/// that tick means putting each slot's distinct original back). A homogeneous batch cannot express
/// "give slot A its value and slot B its other value" — it would flatten the selection onto one
/// member's number, which is the very lie the multi-edit gate exists to prevent. So Revert walks the
/// snapshot and calls the SINGLE-slot [`attrs_update_position`]/[`attrs_update_slot`] per entry,
/// each of which is the exact seam a typed edit uses.
///
/// These are REAL writes (the spec is explicit): each one runs `after_local_edit`, so Revert dirties
/// the mission and mints undo steps like any edit — it is a bounded UNDO CONVENIENCE, not OK/Cancel
/// transactionalism, and the panel says so in one line. `z` is passed as `Some(snap.z)` so the
/// terrain-follow z-keep cannot re-flatten a restored elevation; every field is `Some`, so the
/// all-`None` no-op guards never fire and a locked slot's transform half is simply dropped by the
/// core exactly as it drops a typed transform edit (identity/type restore still lands — T-665 locks
/// transform only). These are UNCONDITIONAL writes — `attrs_update_slot`/`attrs_update_position` do
/// not compare against the current value, so Revert always re-stamps the snapshot and always fires
/// the history tail; that is the intended "real write" semantics, not a bug. The end state is the
/// readback equality the acceptance pins: after Revert, `read_attrs(id)` equals the captured snap.
/// T-939.2 — membership is restored after these existing writes, as one additional undo group for
/// all changed slots. Each destination comes from its own snapshot, including mixed factions;
/// the membership operation leaves authored squads and vehicles in place.
#[cfg(target_arch = "wasm32")]
fn revert_to_snapshot(snapshot: StoredValue<Vec<engine_ops::SlotAttrs>>) {
    for snap in snapshot.get_value() {
        engine_ops::attrs_update_position(
            &snap.id,
            Some(snap.x),
            Some(snap.y),
            Some(snap.z),
            Some(snap.rotation),
        );
        engine_ops::attrs_update_slot(
            &snap.id,
            Some(snap.role.clone()),
            Some(snap.tag.clone()),
            Some(snap.stance.clone()),
            Some(snap.asset_id.clone()),
            Some(snap.description.clone()),
        );
    }
    engine_ops::restore_slot_squads(&snapshot.get_value());
}

#[cfg(target_arch = "wasm32")]
fn transform_tab(
    targets: StoredValue<Vec<String>>,
    attrs: StoredValue<engine_ops::SlotAttrs>,
    is_multi: bool,
    diff: engine_ops::AttrDiff,
    opts: MultiOpts,
    // T-082 (wave-102 F-7) — how many of `targets` sit on a transform-locked layer.
    locked_n: usize,
) -> impl IntoView {
    let a = attrs.get_value();
    let n = targets.get_value().len();
    // T-082 (F-7) — EVERY target refused ⇒ the four coordinate fields are disabled, because the
    // core will drop the write and the modal used to show the typed value as though it had landed.
    // A selection that STRADDLES the lock keeps them live (the unlocked members really do move) and
    // gets the count in the note below instead — claiming a partial write is total would be the
    // same lie pointed the other way.
    let all_locked = n > 0 && locked_n == n;
    // A gate exists only under a multi-selection AND only for a field the selection disagrees on;
    // everything else stays the pre-T-649 always-live field with no checkbox.
    let g = move |differs: bool, latch| {
        let base = Gate::maybe(is_multi && differs, latch);
        if all_locked {
            base.refused()
        } else {
            base
        }
    };
    // Stance is NOT a transform in the core's sense — `update_slot` carries no lock check — so it
    // stays live on a locked slot. Gating it here would invent a refusal the core does not make.
    let stance_gate = Gate::maybe(is_multi && diff.stance, opts.stance);
    view! {
        <div class="flex flex-col gap-4">
            // T-082 (wave-102 F-7) — the lock is stated, not implied by four dead inputs.
            {(locked_n > 0)
                .then(|| {
                    let msg = if all_locked && n > 1 {
                        format!(
                            "All {n} selected entities are on a locked layer. Their position and rotation cannot be edited — unlock the layer in the Outliner.",
                        )
                    } else if all_locked {
                        "This entity is on a locked layer. Its position and rotation cannot be edited — unlock the layer in the Outliner."
                            .to_string()
                    } else {
                        format!(
                            "{locked_n} of {n} selected entities are on a locked layer; a Transform edit will skip those and apply to the other {}.",
                            n - locked_n,
                        )
                    };
                    view! {
                        <p class="rounded-md border border-tertiary/30 bg-tertiary/10 px-3 py-2 text-label-sm normal-case text-on-surface-variant">
                            {msg}
                        </p>
                    }
                })}
            <div class="grid grid-cols-3 gap-3">
                // F-13 — the coordinate fields carry a `m` unit suffix (the same right-aligned glyph
                // Rotation uses for `°`), so a metre reading is not a bare number. Display rounding is
                // already handled by `field_display`; this only adds the unit.
                {number_field(
                    "X",
                    a.x,
                    Some("m"),
                    g(diff.x, opts.x),
                    move |x| commit_position(targets, Some(x), None, None, None),
                )}
                {number_field(
                    "Y",
                    a.y,
                    Some("m"),
                    g(diff.y, opts.y),
                    move |y| commit_position(targets, None, Some(y), None, None),
                )}
                {number_field(
                    "Z",
                    a.z,
                    Some("m"),
                    g(diff.z, opts.z),
                    move |z| commit_position(targets, None, None, Some(z), None),
                )}
            </div>
            {number_field(
                "Rotation",
                a.rotation,
                Some("°"),
                g(diff.rotation, opts.rotation),
                move |r| commit_position(targets, None, None, None, Some(r)),
            )}
            <div class="flex flex-col gap-1">
                {field_label("Stance", stance_gate)}
                <select
                    aria-label="Stance"
                    disabled=move || stance_gate.locked()
                    // A differing stance selects the empty placeholder option below rather than one
                    // member's stance — the select must not claim they all stand.
                    prop:value=if stance_gate.differs() {
                        String::new()
                    } else {
                        a.stance.clone()
                    }
                    on:change=move |ev| {
                        commit_slot(targets, None, None, Some(event_target_value(&ev)), None, None)
                    }
                    class=move || {
                        let lock = if stance_gate.locked() { CONTROL_LOCKED } else { "" };
                        format!("{CONTROL}{lock}")
                    }
                >
                    {stance_gate
                        .differs()
                        .then(|| {
                            view! {
                                <option value="" disabled class="bg-surface-container">
                                    "— Multiple values —"
                                </option>
                            }
                        })}
                    <option value="stand" class="bg-surface-container">"Standing"</option>
                    <option value="crouch" class="bg-surface-container">"Crouched"</option>
                    <option value="prone" class="bg-surface-container">"Prone"</option>
                </select>
            </div>
            <p class="text-label-sm normal-case text-outline">
                // F-23 — DEM shipped (the status-bar Z is terrain-sampled), so the old "Z is manual
                // until DEM ships" hint is stale. Z is still typeable here; it just no longer promises
                // a feature that already landed.
                "Drag on the map or edit coordinates above. Z is sampled from terrain elevation (DEM); edit it here to override."
            </p>
        </div>
    }
}

/// T-810 (F-23 a) — the entity TYPE as a **searchable catalog picker** with a freetext escape hatch.
///
/// Eden's Object:Type is a searchable tree with a magnifier; the author browses Cars/Drones/Men…
/// rather than recalling an asset id. TBD's freetext field was expert-only recall and the surface a
/// stray keystroke corrupted slots through, so this replaces the field's ENTRY affordance with a
/// picker while keeping every T-082 wiring the pins protect (the caller hands in the read, the gate,
/// and the one commit seam). Structure, top to bottom:
///
///   * a **trigger button** showing the current type — the catalog's friendly `display_name` when the
///     id resolves ([`crate::v2::apps::editor::arsenal::asset_catalog::find_catalog_item`]), the raw id when it does not (a
///     modpack-switched or hand-typed id, still shown honestly), or "Faction default" when empty.
///     Empty = faction default stays a FIRST-CLASS option, exactly as the freetext field promised.
///   * an **anchored popover** (this is the containing-block trap the registry names — the popover is
///     `absolute` inside a `relative` wrapper, NOT `fixed`, so it stays inside the stack-governed
///     modal and cannot escape to the viewport). It holds a search box (typing filters the live tree
///     via [`crate::v2::apps::editor::arsenal::asset_catalog::filter_catalog`], the SAME grammar the dock search uses), a
///     "Faction default (clear)" row, and the filtered leaves. Picking a leaf writes its canonical
///     `resource_name` through `on_commit`; the `ASSET-RESOLVES` validator then clears live because
///     that id is in `known_asset_ids_from_registry`.
///   * an **Advanced** disclosure revealing the freetext [`text_field`] for ids no catalog leaf
///     offers — the T-785 draft discipline is kept verbatim (this is the field, unchanged), so the
///     expert path survives behind an affordance instead of being the default.
///
/// **Empty catalog** (dev without seed, or a modpack with no placeable rows): the popover shows the
/// cause+retry surface, never a dead list (F-23 / the T-800 lesson). The copy MIRRORS
/// `eden_dock_right::catalog_failure_view`'s vocabulary — it is a mirrored copy, not the shared fn,
/// because that fn takes `registry_fetch_gen`, a signal the modal is not handed (its call site in
/// `mission_editor.rs` is past this slice's file boundary and cannot be changed to pass one). Retry
/// is therefore a full reload (`window.location.reload`), which the dock's own doc names as the
/// equivalent recovery ("a populated seed then Readies the tree"); it genuinely re-runs the cold
/// `/registry` fetch. While the registry is still LOADING (`registry_items == None`) the popover says
/// so and the trigger stays live — the modal re-renders when the rows land because this reads
/// `registry_items` reactively.
///
/// **Esc layering** (registry: picker → field → modal, one per press): the popover's search input
/// consumes Escape and closes the POPOVER only (`stop_propagation`, so the modal window listener does
/// not also fire); the advanced [`text_field`] keeps its own field-level Escape (abandon draft, blur);
/// the modal's listener is the third press. Three layers, one collapse per keypress.
///
/// **Multi-edit**: the trigger takes the same [`Gate`] every field takes — a differing TYPE across
/// the selection is blank+locked behind "Apply to all", and once ticked a pick writes ALL targets
/// (`commit_slot` routes to the T-788 batch by target count). A refused gate disables the trigger.
#[cfg(target_arch = "wasm32")]
fn type_picker(
    label: &'static str,
    value: String,
    gate: Gate,
    registry_items: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
    // `+ Send` because the popover is a reactive render closure (it rebuilds the leaf list as the
    // query changes) and Leptos requires such closures to be `Send`. The only caller passes a
    // closure capturing `targets: StoredValue<Vec<String>>` (which is `Send`), so the bound is free
    // — and `text_field` below, which takes a bound WITHOUT `Send`, still accepts this stricter one.
    on_commit: impl Fn(String) + Copy + Send + 'static,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let query = RwSignal::new(String::new());
    let advanced = RwSignal::new(false);
    let current = StoredValue::new(value.clone());
    // NB: no `Effect` here on purpose — `type_picker` is called from the modal's per-`doc_tick`
    // render closure, so an effect minted here would accumulate one per re-render. The query is
    // instead reset inline when the trigger OPENS the popover (below), which is the only moment a
    // stale query could leak in.
    // The trigger's label: friendly name if the id resolves in the live catalog, else the raw id,
    // else the first-class empty option. Reactive on `registry_items` so a late catalog load upgrades
    // a raw id to its display name without reopening.
    let trigger_text = move || {
        let id = current.get_value();
        if gate.differs() {
            return "Multiple values".to_string();
        }
        if id.is_empty() {
            return "Faction default".to_string();
        }
        registry_items
            .get()
            .as_deref()
            .and_then(|items| {
                crate::v2::apps::editor::arsenal::asset_catalog::find_catalog_item(items, &id)
            })
            .map_or(id, |it| it.display_name.clone())
    };
    // Pick a leaf / clear: commit, then close the popover. `close` first would drop the closure's
    // capture on some paths, so commit precedes close (both are cheap).
    let pick = move |asset_id: String| {
        on_commit(asset_id);
        open.set(false);
    };
    view! {
        <div class="flex flex-col gap-1">
            {field_label(label, gate)}
            // Anchor: `relative` wrapper so the popover below is `absolute` to HERE (the containing-block
            // trap) and stays inside the modal's stacking context.
            <div class="relative">
                <button
                    type="button"
                    aria-label=label
                    aria-haspopup="listbox"
                    data-testid="type-picker-trigger"
                    disabled=move || gate.locked()
                    on:click=move |_| {
                        if !gate.locked_now() {
                            // Reset the search on OPEN so a stale query never leaks into a new visit.
                            if !open.get_untracked() {
                                query.set(String::new());
                            }
                            open.update(|o| *o = !*o);
                        }
                    }
                    class=move || {
                        let lock = if gate.locked() { CONTROL_LOCKED } else { "" };
                        // A placeholder-toned label when empty/differing, full-strength when set.
                        format!("{CONTROL} flex items-center justify-between text-left{lock}")
                    }
                >
                    <span class=move || {
                        let id = current.get_value();
                        if gate.differs() || id.is_empty() {
                            "truncate text-on-surface-variant"
                        } else {
                            "truncate text-on-surface"
                        }
                    }>{trigger_text}</span>
                    <crate::v2::core::ui::MaterialIcon name="search" />
                </button>
                {move || {
                    open.get().then(|| {
                        // Backdrop: a click anywhere outside closes the popover (the same click-away
                        // idiom the AssetPickerOverlay uses). `z-40` under the popover's `z-50`, and
                        // scoped to the modal via the relative parent — it is `absolute inset-0` on the
                        // wrapper, not `fixed`, so it never covers the rest of the dialog.
                        let items = registry_items.get();
                        let body = match items {
                            None => view! {
                                // Registry not yet loaded — say so; the trigger stays usable and this
                                // re-renders when the rows arrive (reactive read above).
                                <p
                                    class="px-3 py-4 text-label-sm text-on-surface-variant"
                                    data-testid="type-picker-loading"
                                >
                                    "Loading the asset catalog…"
                                </p>
                            }.into_any(),
                            Some(items) => {
                                let full = crate::v2::apps::editor::arsenal::asset_catalog::build_picker_catalog_tree(&items);
                                if crate::v2::apps::editor::arsenal::asset_catalog::catalog_leaf_count(&full) == 0 {
                                    // T-800 MIRRORED vocabulary — cause + retry, never a dead list.
                                    view! {
                                        <div
                                            class="flex flex-col gap-2 px-3 py-3"
                                            data-testid="type-picker-empty"
                                        >
                                            <p class="text-label-sm text-error">
                                                "No modpack is configured, so the asset catalog is empty. Set a current modpack, then retry."
                                            </p>
                                            <button
                                                type="button"
                                                data-testid="type-picker-retry"
                                                class="self-start rounded border border-outline-variant/40 px-2 py-1 text-label-sm text-on-surface transition hover:bg-surface-container-high"
                                                on:click=move |_| {
                                                    // The modal cannot re-kick the in-place cold fetch
                                                    // (the `registry_fetch_gen` signal lives dock-side,
                                                    // past this file's boundary). A full reload IS the
                                                    // dock's documented equivalent recovery and really
                                                    // re-runs `/registry`.
                                                    if let Some(w) = web_sys::window() {
                                                        let _ = w.location().reload();
                                                    }
                                                }
                                            >
                                                "Retry"
                                            </button>
                                        </div>
                                    }.into_any()
                                } else {
                                    let q = query.get();
                                    let filtered = crate::v2::apps::editor::arsenal::asset_catalog::filter_catalog(&full, &q);
                                    // Flatten the (filtered) tree to placeable leaves. A folder carries
                                    // no payload, so `payload.is_some()` is exactly "a pickable leaf".
                                    let mut leaves: Vec<(String, String)> = Vec::new();
                                    fn collect(
                                        nodes: &[crate::v2::apps::editor::arsenal::asset_catalog::CatalogNode],
                                        out: &mut Vec<(String, String)>,
                                    ) {
                                        for n in nodes {
                                            if let Some(p) = &n.payload {
                                                out.push((n.label.clone(), p.asset_id.clone()));
                                            }
                                            collect(&n.children, out);
                                        }
                                    }
                                    collect(&filtered, &mut leaves);
                                    let no_match = !q.trim().is_empty() && leaves.is_empty();
                                    let empty_msg = crate::v2::apps::editor::arsenal::asset_catalog::search_empty_message(&q, "assets");
                                    let rows = leaves
                                        .into_iter()
                                        .map(|(lbl, id)| {
                                            let idc = id.clone();
                                            view! {
                                                <button
                                                    type="button"
                                                    data-testid="type-picker-leaf"
                                                    class="block w-full truncate px-3 py-1.5 text-left text-label-md text-on-surface hover:bg-primary/20"
                                                    title=id
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        pick(idc.clone());
                                                    }
                                                >
                                                    {lbl}
                                                </button>
                                            }
                                        })
                                        .collect_view();
                                    view! {
                                        <div class="min-h-0 flex-1 overflow-y-auto py-1">
                                            // First-class "clear to faction default" row — always
                                            // present so empty stays a deliberate, reachable choice.
                                            <button
                                                type="button"
                                                data-testid="type-picker-clear"
                                                class="block w-full truncate border-b border-outline-variant/20 px-3 py-1.5 text-left text-label-md text-on-surface-variant hover:bg-primary/20"
                                                on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    pick(String::new());
                                                }
                                            >
                                                "Faction default (clear)"
                                            </button>
                                            {no_match
                                                .then(|| view! {
                                                    <p
                                                        class="px-3 py-2 text-label-sm text-on-surface-variant"
                                                        data-testid="type-picker-nomatch"
                                                    >
                                                        {empty_msg}
                                                    </p>
                                                })}
                                            {rows}
                                        </div>
                                    }.into_any()
                                }
                            }
                        };
                        view! {
                            <div
                                class="absolute inset-0 z-40"
                                on:click=move |_| open.set(false)
                            ></div>
                            <div
                                class="glass absolute left-0 right-0 top-full z-50 mt-1 flex max-h-64 flex-col overflow-hidden rounded-md border border-outline-variant/30 shadow-2xl"
                                data-testid="type-picker-popover"
                            >
                                <div class="border-b border-outline-variant/25 p-1.5">
                                    <input
                                        type="search"
                                        // autofocus so typing filters immediately, Eden-style.
                                        autofocus
                                        aria-label="Search asset types"
                                        data-testid="type-picker-search"
                                        class="w-full rounded bg-surface/40 px-2 py-1 text-label-md text-on-surface outline-none placeholder:text-on-surface-variant"
                                        placeholder="Search types…"
                                        on:input=move |ev| query.set(event_target_value(&ev))
                                        on:keydown=move |ev| {
                                            // Esc closes THIS layer (the popover) first and consumes,
                                            // so the modal's window listener does not also close the
                                            // modal on the same press — picker → field → modal.
                                            if ev.key() == "Escape" {
                                                ev.stop_propagation();
                                                open.set(false);
                                            }
                                        }
                                    />
                                </div>
                                {body}
                            </div>
                        }
                    })
                }}
            </div>
            // The freetext escape hatch, behind an Advanced disclosure. It IS the T-785 text_field,
            // unchanged — the draft discipline and its own Esc layer stay exactly as pinned.
            //
            // Shown only when the field is NOT gated behind a differ-checkbox (`!gate.differs()` ⇒
            // `gate.opt` is None ⇒ the field_label the text_field draws carries no "Apply to all"
            // box, so the picker's single box above is never duplicated). Under a DIFFERING
            // multi-selection the operator ticks the picker's box and picks a catalog leaf for all;
            // typing an UNLISTED id across a differing selection is the one corner this trades away,
            // deliberately, to keep exactly one opt-in control on screen. A `shut` (core-refused)
            // field still shows Advanced but the text_field disables itself through the same gate.
            //
            // The `text_field` is rendered EAGERLY (not inside a reactive `move ||`) and hidden via a
            // CSS class toggle. That is deliberate: a reactive render closure must be `Send`, and
            // `on_commit` is a bare `impl Fn` with no such bound, so wrapping the field in `move ||`
            // fails to compile. Visibility is the class closure's job (it captures only `advanced`),
            // and the field's own draft is untouched while hidden.
            {(!gate.differs()).then(|| view! {
                <button
                    type="button"
                    data-testid="type-picker-advanced-toggle"
                    class="self-start text-label-sm normal-case text-primary hover:underline"
                    on:click=move |_| advanced.update(|a| *a = !*a)
                >
                    {move || if advanced.get() { "Hide advanced" } else { "Advanced: enter an asset id" }}
                </button>
                <div class=move || if advanced.get() { "" } else { "hidden" }>
                    {text_field(
                        label,
                        current.get_value(),
                        "Asset id — empty uses the faction default",
                        gate,
                        on_commit,
                    )}
                </div>
            })}
        </div>
    }
}

#[cfg(target_arch = "wasm32")]
fn identity_tab(
    targets: StoredValue<Vec<String>>,
    attrs: StoredValue<engine_ops::SlotAttrs>,
    is_multi: bool,
    diff: engine_ops::AttrDiff,
    opts: MultiOpts,
    // T-810 (F-23 a) — the live catalog source for the TYPE picker.
    registry_items: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
) -> impl IntoView {
    let a = attrs.get_value();
    let g = |differs: bool, latch| Gate::maybe(is_multi && differs, latch);
    // T-939.2 — Squad used to be an inert read-only div here ("{n} entities" under a
    // multi-selection, the raw `squadId` otherwise), so moving a slot between squads — let alone
    // between factions — was unreachable from this modal at any selection size. It is now the
    // two-control `reassign_picker` below, and it has NO per-field gate/checkbox on purpose: the
    // T-649 gate exists for columns a blind multi-edit could overwrite with the first slot's value,
    // and this control never shows a value it did not compute over the whole target set (a mixed
    // selection reads "Mixed", not slot[0]'s squad). Picking is the opt-in.
    let _ = is_multi;
    view! {
        <div class="flex flex-col gap-4">
            // T-082 ATTR-FIELD-OBJ-TYPE / T-810 (F-23 a). The entity TYPE — the slot's `assetId`, the
            // prefab it spawns as. It was authored on palette drop and mutable in the core all along.
            //
            // T-810 turned this from freetext into a SEARCHABLE PICKER over the live catalog (Eden's
            // Object:Type is a searchable tree; freetext asset-id recall is expert-only and was the
            // field a stray keystroke corrupted slots through). `type_picker` owns the popover, the
            // "faction default" clear, the advanced-freetext escape hatch, and the empty-catalog
            // surface; the pieces the T-082 pins fix stay HERE in `identity_tab` and are handed in:
            // the read (`a.asset_id.clone()`), the multi-edit gate (`g(diff.asset_id, opts.asset_id)`),
            // and the ONE commit seam — `commit_slot(targets, None, None, None, Some(asset_id), None)`
            // — so a picked leaf and a typed id both land in the asset_id slot alone, exactly as
            // before, and multi-edit apply-to-all batching (T-788) is unchanged (`commit_slot` routes
            // single vs multi by target count).
            {type_picker(
                "Type",
                a.asset_id.clone(),
                g(diff.asset_id, opts.asset_id),
                registry_items,
                move |asset_id| commit_slot(targets, None, None, None, Some(asset_id), None),
            )}
            {text_field(
                "Role",
                a.role.clone(),
                "Rifleman",
                g(diff.role, opts.role),
                move |role| commit_slot(targets, Some(role), None, None, None, None),
            )}
            // T-082 ATTR-FIELD-OBJ-ROLE-DESC. A field of its OWN, which is the entire point: `role`
            // above is the SHORT label the ORBAT tree, the modal subtitle and the compiled document
            // all use, and until now it was also the only place to put a sentence about what the
            // slot is for. Writing prose into it renamed the role everywhere it appears.
            //
            // Editor-block state — it rides `editor.slots` (survives save/reload and copy/paste)
            // and is structurally absent from the compiled mod document. Say so here rather than
            // let an operator infer it reaches the briefing.
            {text_field(
                "Role Description",
                a.description.clone(),
                "What this slot is for — editor only, not sent to the game",
                g(diff.description, opts.description),
                move |desc| commit_slot(targets, None, None, None, None, Some(desc)),
            )}
            {text_field(
                "Tag",
                a.tag.clone(),
                "MED · ENG · SL…",
                g(diff.tag, opts.tag),
                move |tag| commit_slot(targets, None, Some(tag), None, None, None),
            )}
            {reassign_picker(targets)}
        </div>
    }
}

/// T-939.2 — the Attributes modal's **faction selector + squad picker**, both acting on the WHOLE
/// selection in one undo group.
///
/// # Why there is no local "picked faction" signal
///
/// Both `<select>`s read their current value out of the document, and the faction one COMMITS on
/// change (to that faction's first squad) rather than merely arming the squad list. That is the
/// acceptance — "selecting five slots and choosing another faction moves all five" — and it is also
/// what keeps this control stateless: the commit bumps `doc_tick`, the modal body re-renders, and
/// the selects re-read. A `RwSignal` holding the pick would have to be minted on the component
/// (the T-649 `MultiOpts` lesson: a latch minted inside the render closure un-ticks itself the
/// instant its own commit lands), and there is nothing here that needs to survive a render.
///
/// The one signal that does exist, `refusal`, holds the named reason from a rejected pick. It is
/// per-render on purpose: a refusal changes no document state, so no re-render wipes it, and the
/// next SUCCESSFUL move does — which is exactly when the message stops being true.
///
/// # What "Mixed" means
///
/// Under a multi-selection whose slots do not all sit in the same squad (or faction), the select
/// shows a `Mixed` placeholder rather than the first slot's value. Showing slot[0]'s squad as if it
/// were the group's is the defect this replaces, one layer down.
#[cfg(target_arch = "wasm32")]
fn reassign_picker(targets: StoredValue<Vec<String>>) -> impl IntoView {
    use website_map_engine::editing::hosted_commands as ops;

    let (factions, squads) = ops::reassign_rows();
    // Deterministic faction order, the same `id` sort `build_orbat` uses, so the dropdown and the
    // Outliner list factions in the same order.
    let mut ordered = factions.clone();
    ordered.sort_by(|a, b| a.id.cmp(&b.id));

    // The selection's current squads and factions, over the whole target set.
    //
    // Inverted from the squad rows already in hand — `SquadRow::slot_ids` — rather than asking
    // `read_attrs` per id: that reader materializes the WHOLE slot SoA on every call, so a
    // fifty-slot selection would pay fifty full scans on every render of this modal, which
    // re-renders on every `doc_tick`. One inversion, then lookups.
    let ids = targets.get_value();
    let squad_by_slot: std::collections::HashMap<&str, &str> = squads
        .iter()
        .flat_map(|s| {
            s.slot_ids
                .iter()
                .map(move |sid| (sid.as_str(), s.id.as_str()))
        })
        .collect();
    let current_squads: Vec<String> = ids
        .iter()
        .filter_map(|id| squad_by_slot.get(id.as_str()).map(|s| (*s).to_string()))
        .collect();
    let one_squad = current_squads
        .first()
        .filter(|first| {
            current_squads.len() == ids.len() && current_squads.iter().all(|s| &s == first)
        })
        .cloned()
        .unwrap_or_default();
    let faction_of = |sid: &String| {
        squads
            .iter()
            .find(|s| &s.id == sid)
            .map(|s| s.faction_id.clone())
            .unwrap_or_default()
    };
    let current_factions: Vec<String> = current_squads.iter().map(faction_of).collect();
    let one_faction = current_factions
        .first()
        .filter(|first| {
            current_factions.len() == ids.len() && current_factions.iter().all(|f| &f == first)
        })
        .cloned()
        .unwrap_or_default();

    // The squad list follows the DISPLAYED faction; with a mixed selection there is no one faction
    // to list, so the operator picks a faction first and the squad list fills in on the re-render.
    let listed_faction = one_faction.clone();
    let squad_options: Vec<(String, String)> = ordered
        .iter()
        .find(|f| f.id == listed_faction)
        .map(|f| {
            f.squad_ids
                .iter()
                .filter_map(|sid| squads.iter().find(|s| &s.id == sid))
                .map(|s| {
                    let name = if s.name.trim().is_empty() {
                        s.id.clone()
                    } else {
                        s.name.clone()
                    };
                    (s.id.clone(), format!("{name} ({})", s.slot_ids.len()))
                })
                .collect()
        })
        .unwrap_or_default();

    let no_squads = squad_options.is_empty();
    let refusal = RwSignal::new(String::new());
    let n = ids.len();
    // One commit seam for both controls: the faction select passes an empty squad id ("this
    // faction, its first squad"), the squad select names one. Everything else — the refusal, the
    // undo group, the keep-source core path — is identical, so the two controls can never disagree
    // about what a reassign is.
    let commit = move |faction_id: String, squad_id: String| {
        let target = ops::ReassignTarget {
            faction_id,
            squad_id,
        };
        match ops::reassign_slots(&targets.get_value(), &target) {
            Ok(_) => refusal.set(String::new()),
            Err(reason) => refusal.set(reason),
        }
    };
    let commit_faction = commit;
    let commit_squad = commit;
    let faction_for_squad = one_faction.clone();

    view! {
        <div class="flex flex-col gap-3">
            <label class="flex flex-col gap-1">
                <span class="text-label-sm uppercase tracking-wider text-outline">"Faction"</span>
                <select
                    aria-label="Faction"
                    class=CONTROL
                    prop:value=one_faction.clone()
                    on:change=move |ev| {
                        let picked = event_target_value(&ev);
                        if !picked.is_empty() {
                            commit_faction(picked, String::new());
                        }
                    }
                >
                    // The mixed / unfiled placeholder is only ever a READ: picking it is not a
                    // destination, so it commits nothing (the `is_empty` guard above).
                    <option value="" selected=one_faction.is_empty()>
                        {if n > 1 { "Mixed — pick a faction to move all" } else { "Unfiled" }}
                    </option>
                    {ordered
                        .iter()
                        .map(|f| {
                            let sel = f.id == one_faction;
                            view! {
                                <option value=f.id.clone() selected=sel>
                                    {faction_label(f)}
                                </option>
                            }
                        })
                        .collect_view()}
                </select>
            </label>
            <label class="flex flex-col gap-1">
                <span class="text-label-sm uppercase tracking-wider text-outline">"Squad"</span>
                <select
                    aria-label="Squad"
                    class=CONTROL
                    prop:value=one_squad.clone()
                    disabled=no_squads
                    on:change=move |ev| {
                        let picked = event_target_value(&ev);
                        if !picked.is_empty() {
                            commit_squad(faction_for_squad.clone(), picked);
                        }
                    }
                >
                    <option value="" selected=one_squad.is_empty()>
                        {if no_squads {
                            "No squads under this faction"
                        } else if n > 1 {
                            "Mixed — pick a squad to move all"
                        } else {
                            "Unfiled"
                        }}
                    </option>
                    {squad_options
                        .into_iter()
                        .map(|(id, label)| {
                            let sel = id == one_squad;
                            view! { <option value=id selected=sel>{label}</option> }
                        })
                        .collect_view()}
                </select>
            </label>
            // Requirement 4 — the named reason, in the modal, where the pick was made.
            {move || {
                let why = refusal.get();
                (!why.is_empty())
                    .then(|| {
                        view! {
                            <p
                                role="alert"
                                class="rounded-md border border-error/40 bg-error/10 px-2.5 py-1.5 text-label-sm normal-case text-error"
                            >
                                {why}
                            </p>
                        }
                    })
            }}
            <p class="text-label-sm normal-case text-outline">
                {if n > 1 {
                    format!("Applies to all {n} selected entities, as one undo step.")
                } else {
                    "Moving a slot out never deletes the squad it left.".to_string()
                }}
            </p>
        </div>
    }
}

#[cfg(target_arch = "wasm32")]
fn states_tab() -> impl IntoView {
    view! {
        <div class="flex flex-col gap-3">
            <p class="text-label-sm normal-case text-outline">
                "Unit traits — wired to the compiler in a later phase."
            </p>
            <div class="flex items-center justify-between py-0.5">
                <span class="text-label-md text-on-surface-variant">"Medic (soon)"</span>
                <span class="text-label-sm text-outline">"—"</span>
            </div>
            <div class="flex items-center justify-between py-0.5">
                <span class="text-label-md text-on-surface-variant">"Engineer (soon)"</span>
                <span class="text-label-sm text-outline">"—"</span>
            </div>
        </div>
    }
}
/* ─────────────────────────── T-082 source pins ─────────────────────────── */

#[cfg(test)]
#[path = "tests/attributes_modal/batch_faction_and_squad_reassignment.rs"]
mod batch_faction_and_squad_reassignment_tests;
#[cfg(test)]
#[path = "tests/attributes_modal/identity_and_raw_attributes.rs"]
mod identity_and_raw_attributes_tests;
#[cfg(test)]
#[path = "tests/attributes_modal/modal_escape_stack.rs"]
mod modal_escape_stack_tests;
#[cfg(test)]
#[path = "tests/attributes_modal/numeric_field_input.rs"]
mod numeric_field_input_tests;
#[cfg(test)]
#[path = "tests/attributes_modal/position_and_selection.rs"]
mod position_and_selection_tests;
#[cfg(test)]
#[path = "tests/attributes_modal/transform_field_copy.rs"]
mod transform_field_copy_tests;
#[cfg(test)]
#[path = "tests/attributes_modal/type_picker_revert_and_vehicle.rs"]
mod type_picker_revert_and_vehicle_tests;
