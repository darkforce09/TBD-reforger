//! Attributes modal — the AttributesModal.tsx + RightInspector/fields.tsx port (T-159.26, spec
//! `t159_23_attributes_modal.md`). Opened by dbl-clicking a slot on the map or activating an
//! outliner row; Esc / backdrop / ✕ close. Tabs: **Transform**
//! (X/Y/Z/Rotation NumberFields committing on blur/Enter via `update_slot_position`, plus a Stance
//! select), **Identity** (Role/Tag TextFields + readonly Squad), **States** (trait stub), and
//! **Arsenal** (live loadout editor — T-068.10 / T-180.9; `open_arsenal` selects tab index 3).
//! Commits run `editor_ops::attrs_update_*` → `after_local_edit` (rebind + persist + one undo
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
//! SUPPRESS this modal (the old A1 rule, a hard `return` in `editor_ops::open_attributes`). It now
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
//! `editor_ops::read_attrs_diff` owns the "do they differ" half; the checkbox + disable half is
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
/// explicitly when vehicles were excluded because [`crate::editor_ops::attrs_multi_ids`] drops
/// non-slot ids (vehicles carry none of the SoA columns multi-edit stamps).
#[must_use]
pub(crate) fn attrs_multi_subtitle(slot_n: usize, selection_n: usize) -> String {
    let base = format!("{slot_n} slots selected · multi-edit");
    if selection_n > slot_n {
        format!("{base} · vehicles excluded")
    } else {
        base
    }
}

/// The modal host. Renders nothing while closed (`attrs_open == None`) — V-capture-safe like the
/// suite Dialog. `doc_ver` is the re-read trigger (the doc has no change subscription).
#[component]
pub fn AttributesModal(
    attrs_open: RwSignal<Option<String>>,
    /// T-180.9 — tab index shared with OpsCtx (`open_arsenal` sets 3 = Arsenal).
    attrs_tab: RwSignal<usize>,
    doc_tick: RwSignal<u64>,
    /// T-159.27 — flat registry gear rows for the Arsenal tab.
    registry_items: RwSignal<Option<Vec<crate::dto::RegistryItem>>>,
    /// T-167 — compat edge feed for the Smart Arsenal (optic/magazine rows + validation).
    compat: RwSignal<crate::arsenal_rules::CompatFeed>,
) -> impl IntoView {
    // Esc closes (React Dialog behavior); the editor's own keydown handler skips editable fields,
    // so this window listener is the one Esc path.
    // T-726 — modal-stack gate; topmost consumes.
    #[cfg(target_arch = "wasm32")]
    {
        let modal_id = crate::ui::modal_stack::register(move || {
            attrs_open.try_get_untracked().flatten().is_some()
        });
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if attrs_open.get_untracked().is_some()
                && ev.key() == "Escape"
                && crate::ui::modal_stack::is_topmost_open(modal_id)
            {
                crate::editor_ops::close_attributes();
            }
        });
        on_cleanup(move || {
            esc.remove();
            crate::ui::modal_stack::unregister(modal_id);
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
    let snapshot: StoredValue<Vec<crate::editor_ops::SlotAttrs>> = StoredValue::new(Vec::new());
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
                    let mut ids = crate::editor_ops::attrs_multi_ids(id);
                    if ids.is_empty() {
                        ids = vec![id.to_string()];
                    }
                    ids.iter()
                        .filter_map(|i| crate::editor_ops::read_attrs(i))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            snapshot.set_value(snap);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = open;
    });
    // T-167 / T-180.9 — tab lives on OpsCtx (passed in) so `open_arsenal` can select Arsenal and
    // a doc change (loadout pick bumps `doc_tick`) no longer snaps back to Identity.
    move || {
        let id = attrs_open.get()?;
        let _ = doc_tick.get(); // re-read fields on every doc change (undo/redo/drag)
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (&id, registry_items, compat, attrs_tab, opts);
        #[cfg(target_arch = "wasm32")]
        {
            match crate::editor_ops::read_attrs(&id) {
                Some(attrs) => {
                    // T-649 — the multi-edit target set (empty ⇒ the untouched single-slot modal)
                    // and which of its fields disagree. Both re-read per render, so a selection or
                    // doc change while the modal is open is reflected immediately.
                    let multi = crate::editor_ops::attrs_multi_ids(&id);
                    // T-741 — full selection length (may include vehicles Ctrl+A picked up).
                    let selection_n = crate::editor_ops::attrs_selection_len();
                    let diff = crate::editor_ops::read_attrs_diff(&multi);
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
                    if crate::editor_ops::is_vehicle_id(&id) {
                        Some(vehicle_attrs_view(id, registry_items))
                    } else {
                        // T-744 — `None` means the slot is GONE from the raw rows (undone / deleted),
                        // not merely hidden. Hide keeps `read_attrs` at `Some` (raw existence), so this
                        // arm is no longer reachable from H / layer-hide (wave-113 F-2).
                        crate::editor_ops::close_attributes();
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
    attrs: crate::editor_ops::SlotAttrs,
    multi: Vec<String>,
    selection_n: usize,
    diff: crate::editor_ops::AttrDiff,
    opts: MultiOpts,
    // T-810 (F-23 b) — the pre-open snapshot the Revert button restores. Captured on open (see
    // `AttributesModal`), one entry per edited slot.
    snapshot: StoredValue<Vec<crate::editor_ops::SlotAttrs>>,
    registry_items: RwSignal<Option<Vec<crate::dto::RegistryItem>>>,
    compat: RwSignal<crate::arsenal_rules::CompatFeed>,
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
    let locked_n = crate::editor_ops::attrs_locked_count(&targets.get_value());
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
            on:click=move |_| crate::editor_ops::close_attributes()
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
                        on:click=move |_| crate::editor_ops::close_attributes()
                        class="rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                    >
                        <crate::ui::MaterialIcon name="close" />
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
                            let loadout = crate::editor_ops::read_loadout(&slot_id.get_value());
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
                                <crate::arsenal::ArsenalTab
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
    registry_items: RwSignal<Option<Vec<crate::dto::RegistryItem>>>,
) -> AnyView {
    use crate::editor_ops::VehicleCargoRow;
    use std::collections::HashMap;

    let Some(v) = crate::editor_ops::vehicle_rows()
        .into_iter()
        .find(|r| r.id == id)
    else {
        // Race: id was a vehicle at the host gate, then vanished before this render.
        crate::editor_ops::close_attributes();
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
            crate::editor_ops::set_vehicle_heading(id_h.get_value(), deg);
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
                            crate::editor_ops::set_vehicle_cargo(id_q.clone(), next);
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
                            crate::editor_ops::set_vehicle_cargo(id_r.clone(), next);
                        }
                    >
                        <crate::ui::MaterialIcon name="close" class="block text-sm" />
                    </button>
                </div>
            }
        })
        .collect_view();

    let seat_choices = StoredValue::new(crate::editor_ops::placed_slot_choices());
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
                                crate::editor_ops::clear_crew_seat(id_seat.clone(), sid.clone());
                            } else {
                                crate::editor_ops::assign_crew_seat(
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
            on:click=move |_| crate::editor_ops::close_attributes()
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
                    on:click=move |_| crate::editor_ops::close_attributes()
                    class="rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                >
                    <crate::ui::MaterialIcon name="close" />
                </button>
            </div>
            <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                <div class="flex flex-col gap-4">
                    {heading_row}
                    <div class="flex flex-col gap-1">
                        <div class="flex items-center gap-1.5">
                            <crate::ui::MaterialIcon
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
                                crate::editor_ops::set_vehicle_cargo(id_add.clone(), next);
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
                            <crate::ui::MaterialIcon
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
///     anything change": `editor_ops::attrs_update_position` writes `position` and calls
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
            // `editor_ops::attrs_update_position` now passes the slot's current z back in — but the
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
/// `editor_ops::attrs_update_*` → `after_local_edit()`, which bumps `doc_tick`; the whole
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
        crate::editor_ops::attrs_update_position_multi(&ids, x, y, z, rotation);
    } else if let Some(id) = ids.first() {
        crate::editor_ops::attrs_update_position(id, x, y, z, rotation);
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
        crate::editor_ops::attrs_update_slot_multi(&ids, role, tag, stance, asset_id, description);
    } else if let Some(id) = ids.first() {
        crate::editor_ops::attrs_update_slot(id, role, tag, stance, asset_id, description);
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
#[cfg(target_arch = "wasm32")]
fn revert_to_snapshot(snapshot: StoredValue<Vec<crate::editor_ops::SlotAttrs>>) {
    for snap in snapshot.get_value() {
        crate::editor_ops::attrs_update_position(
            &snap.id,
            Some(snap.x),
            Some(snap.y),
            Some(snap.z),
            Some(snap.rotation),
        );
        crate::editor_ops::attrs_update_slot(
            &snap.id,
            Some(snap.role.clone()),
            Some(snap.tag.clone()),
            Some(snap.stance.clone()),
            Some(snap.asset_id.clone()),
            Some(snap.description.clone()),
        );
    }
}

#[cfg(target_arch = "wasm32")]
fn transform_tab(
    targets: StoredValue<Vec<String>>,
    attrs: StoredValue<crate::editor_ops::SlotAttrs>,
    is_multi: bool,
    diff: crate::editor_ops::AttrDiff,
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
///     id resolves ([`crate::asset_catalog::find_catalog_item`]), the raw id when it does not (a
///     modpack-switched or hand-typed id, still shown honestly), or "Faction default" when empty.
///     Empty = faction default stays a FIRST-CLASS option, exactly as the freetext field promised.
///   * an **anchored popover** (this is the containing-block trap the registry names — the popover is
///     `absolute` inside a `relative` wrapper, NOT `fixed`, so it stays inside the stack-governed
///     modal and cannot escape to the viewport). It holds a search box (typing filters the live tree
///     via [`crate::asset_catalog::filter_catalog`], the SAME grammar the dock search uses), a
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
    registry_items: RwSignal<Option<Vec<crate::dto::RegistryItem>>>,
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
            .and_then(|items| crate::asset_catalog::find_catalog_item(items, &id))
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
                    <crate::ui::MaterialIcon name="search" />
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
                                let full = crate::asset_catalog::build_picker_catalog_tree(&items);
                                if crate::asset_catalog::catalog_leaf_count(&full) == 0 {
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
                                    let filtered = crate::asset_catalog::filter_catalog(&full, &q);
                                    // Flatten the (filtered) tree to placeable leaves. A folder carries
                                    // no payload, so `payload.is_some()` is exactly "a pickable leaf".
                                    let mut leaves: Vec<(String, String)> = Vec::new();
                                    fn collect(
                                        nodes: &[crate::asset_catalog::CatalogNode],
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
                                    let empty_msg = crate::asset_catalog::search_empty_message(&q, "assets");
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
    attrs: StoredValue<crate::editor_ops::SlotAttrs>,
    is_multi: bool,
    diff: crate::editor_ops::AttrDiff,
    opts: MultiOpts,
    // T-810 (F-23 a) — the live catalog source for the TYPE picker.
    registry_items: RwSignal<Option<Vec<crate::dto::RegistryItem>>>,
) -> impl IntoView {
    let a = attrs.get_value();
    let g = |differs: bool, latch| Gate::maybe(is_multi && differs, latch);
    // Squad is READ-ONLY here, so it has no gate and no checkbox — but under a multi-selection it
    // must not display the first slot's squad as if it were the group's.
    let squad = if is_multi {
        format!("{} entities", targets.get_value().len())
    } else if a.squad.is_empty() {
        "—".to_string()
    } else {
        a.squad.clone()
    };
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
            <label class="flex flex-col gap-1">
                <span class="text-label-sm uppercase tracking-wider text-outline">"Squad"</span>
                <div class="rounded-md border border-outline-variant/20 bg-surface-container-lowest/30 px-2.5 py-1.5 font-mono text-code-md text-on-surface-variant">
                    {squad}
                </div>
            </label>
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

// Everything this ticket touches in the modal lives behind `#[cfg(target_arch = "wasm32")]` — it
// builds `view!` trees over `web_sys` nodes and cannot be instantiated by `cargo test`, which runs
// native. So the modal half is pinned the way the rest of this crate pins its wasm-only surfaces
// (`mission_editor.rs`, `arsenal.rs`): against the SCRUBBED live source, with comments and dead
// `cfg` items removed so a pin can never be satisfied by the prose that describes the code.
//
// The BEHAVIOUR half is not pinned this way and does not need to be: `MissionDocCore::
// update_slot_object` and `slot_layer_is_locked` are native, and `store.rs`'s own tests fire them
// against a real document (`update_slot_object_sets_clears_and_leaves_none_fields_alone`,
// `slot_layer_is_locked_agrees_with_the_update_slot_position_refusal`). These pins cover the wiring
// those tests cannot see: that the modal actually calls them, on the fields it claims to.
#[cfg(test)]
mod tests {
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};

    fn attrs_src() -> String {
        live_code(include_str!("attributes.rs"))
    }

    /// ATTR-FIELD-OBJ-TYPE + ATTR-FIELD-OBJ-ROLE-DESC — both fields exist in the Identity tab and
    /// BOTH route through `commit_slot`'s new argument slots.
    ///
    /// The argument position is the assertion, not the presence of a `text_field` call: the two
    /// fields are the 5th and 6th `Option` of one six-argument commit, and a description wired into
    /// the `asset_id` slot would compile, render, and silently overwrite the entity type. Pinned on
    /// `live_code` (string literals blanked) so a label in a comment or a placeholder cannot satisfy
    /// it — this must be a CALL.
    #[test]
    fn identity_tab_commits_type_and_role_description_through_their_own_argument_slots() {
        let src = attrs_src();
        let body = only_body(&src, "fn identity_tab(");
        assert!(
            body.contains("commit_slot(targets, None, None, None, Some(asset_id), None)"),
            "the Type field must commit into the asset_id slot alone; body was:\n{body}"
        );
        assert!(
            body.contains("commit_slot(targets, None, None, None, None, Some(desc))"),
            "the Role Description field must commit into the description slot alone; body was:\n{body}"
        );
        // And the pre-existing three still commit into theirs — the widening must not have shifted
        // Role into Tag's position, which is the one way this edit breaks silently.
        assert!(
            body.contains("commit_slot(targets, Some(role), None, None, None, None)"),
            "Role must still commit into the role slot"
        );
        assert!(
            body.contains("commit_slot(targets, None, Some(tag), None, None, None)"),
            "Tag must still commit into the tag slot"
        );
    }

    /// The two new fields read from `SlotAttrs`'s new columns and participate in the T-649 per-field
    /// multi-edit opt-in (`g(diff.…, opts.…)`) rather than bypassing it — the seam T-649 left.
    #[test]
    fn the_new_fields_read_their_own_columns_and_take_the_multi_edit_gate() {
        let src = attrs_src();
        let body = only_body(&src, "fn identity_tab(");
        for needle in [
            "a.asset_id.clone()",
            "g(diff.asset_id, opts.asset_id)",
            "a.description.clone()",
            "g(diff.description, opts.description)",
        ] {
            assert!(
                body.contains(needle),
                "identity_tab must contain `{needle}`"
            );
        }
    }

    /// The labels an operator actually reads. `live_source` KEEPS string literals — this pin is
    /// about user-visible copy, which is the one thing `live_code` deliberately cannot see.
    #[test]
    fn the_two_new_fields_are_labelled_type_and_role_description() {
        let src = live_source(include_str!("attributes.rs"));
        let body = only_body(&src, "fn identity_tab(");
        assert!(body.contains("\"Type\""), "the type field is labelled Type");
        assert!(
            body.contains("\"Role Description\""),
            "the description field is labelled Role Description, not Description — it is the \
             description OF the role, and `Role` above is the short label it is distinct from"
        );
    }

    /// Wave-102 F-7 — a Transform field the core will REFUSE must be disabled, and no multi-edit
    /// latch may re-open it.
    ///
    /// Pinned on `Gate::locked`, which is the single place the `disabled` attribute is decided for
    /// every field in this modal: `shut ||` must come FIRST and must be an unconditional `||`, so
    /// that `refused()` overrides the opt-in rather than being one vote among two. Ticking "Apply
    /// to all" on a locked slot re-enabling the input is exactly the lie F-7 banked.
    #[test]
    fn a_refused_transform_field_is_disabled_whatever_the_multi_edit_latch_says() {
        let src = attrs_src();
        let locked = only_body(&src, "fn locked(self) -> bool");
        assert!(
            locked.contains("self.shut || self.opt.is_some_and(|o| !o.get())"),
            "Gate::locked must short-circuit on `shut`; body was:\n{locked}"
        );
        // `refused()` must actually be reachable from the Transform tab, and only from there.
        let transform = only_body(&src, "fn transform_tab(");
        assert!(
            transform.contains("base.refused()"),
            "transform_tab must hard-shut its gates when every target is locked"
        );
        let identity = only_body(&src, "fn identity_tab(");
        assert!(
            !identity.contains("refused()"),
            "identity fields must NOT be lock-gated: T-665 locks TRANSFORM only, and `update_slot` \
             / `update_slot_object` carry no lock check, so a role or type edit on a locked slot \
             really does land"
        );
    }

    /// F-7's other half: the count must come from the CORE's own predicate, and `all_locked` must
    /// mean every target — a partially-locked selection still moves its unlocked members, so
    /// disabling the fields there would be the same lie in reverse.
    #[test]
    fn the_lock_affordance_asks_the_core_and_distinguishes_all_locked_from_some_locked() {
        let src = attrs_src();
        let modal = only_body(&src, "fn modal_view(");
        assert!(
            modal.contains("crate::editor_ops::attrs_locked_count(&targets.get_value())"),
            "the modal must ask the core over the same id set the commits fan out to"
        );
        let transform = only_body(&src, "fn transform_tab(");
        assert!(
            transform.contains("let all_locked = n > 0 && locked_n == n;"),
            "all_locked must require EVERY target to be locked; body was:\n{transform}"
        );
    }

    /// `read_attrs` must read the two new fields off the RAW slot rows. This is the defect the
    /// ticket named: the type was unreadable because the read path was the SoA, which has no such
    /// column — not because the mutator was missing.
    #[test]
    fn read_attrs_reads_asset_id_and_description_from_the_raw_slot_rows() {
        let ops = live_code(include_str!("editor_ops.rs"));
        let body = only_body(&ops, "pub fn read_attrs(id: &str) -> Option<SlotAttrs>");
        assert!(
            body.contains("raw_slot_rows(core)"),
            "read_attrs must consult the raw rows, not `materialize()` alone"
        );
        for needle in ["asset_id: row_str(", "description: row_str("] {
            assert!(body.contains(needle), "read_attrs must fill `{needle}…`");
        }
    }

    /* ─────────── T-744 — hide must not close Attributes like undo-away ─────────── */

    /// wave-113 F-2 / T-744: `read_attrs` Option-gates on RAW membership, not SoA membership.
    ///
    /// `materialize()` drops layer-hidden / `editorHidden` slots. The pre-fix body used
    /// `soa.ids.iter().position(|s| s == id)?` as the Option gate, so Hide returned `None` and the
    /// modal's `None` arm called `close_attributes()` — the same path as "slot was undone away".
    ///
    /// Hollow-pin rules: `live_code` blanks comments + string literals, so a docstring claiming the
    /// fix cannot green these needles. delete-prod: stripping the raw gate from a forged copy must
    /// drop the existence needle (proves the pin is about production, not this test module).
    #[test]
    fn read_attrs_gates_existence_on_raw_rows_not_soa_membership() {
        let ops = live_code(include_str!("editor_ops.rs"));
        let body = only_body(&ops, "pub fn read_attrs(id: &str) -> Option<SlotAttrs>");
        let raw_gate = "!rows.contains_key(id)";
        assert!(
            body.contains(raw_gate),
            "T-744: Option must gate on raw membership; body was:\n{body}"
        );
        assert!(
            !body.contains("soa.ids.iter().position(|s| s == id)?"),
            "T-744: SoA position must not be the Option gate (that made hide look like undo-away)"
        );
        assert!(
            body.contains("core.materialize()"),
            "T-744: SoA fields still come from materialize() when the slot is visible"
        );
        assert!(
            body.contains("slot_attrs_from_raw(&rows, id)"),
            "T-744: hidden-but-present slots must fall back to raw field values, not invent zeros"
        );
        // F1: the needle alone is not an exit — an empty `if !rows.contains_key(id) {}` arm kept
        // the pin green while missing ids still yielded Some. Require a real absence return in that
        // arm (wave-135 adversarial).
        let after_gate = body.split(raw_gate).nth(1).expect("raw gate present above");
        let brace = after_gate
            .find('{')
            .expect("T-744: raw gate must open an if-arm");
        let arm_tail = &after_gate[brace + 1..];
        let close = arm_tail.find('}').expect("T-744: raw-gate arm must close");
        let arm = arm_tail[..close].trim();
        assert!(
            arm.contains("return None")
                || arm.split_whitespace().collect::<Vec<_>>().join(" ") == "None",
            "T-744: raw absence arm must exit with None (empty arm must RED); arm was:\n{arm}"
        );
        // delete-prod control: remove the raw gate from a forged production body → needle gone →
        // the positive assert above would RED. (This forged copy is never compiled; it proves the
        // pin is load-bearing on the production token, not on a comment or this test's own source.)
        let forged = body.replacen(raw_gate, "false /* delete-prod */", 1);
        assert!(
            !forged.contains(raw_gate),
            "delete-prod control: stripping the raw gate must remove the existence needle"
        );
    }

    /// The modal's `None` arm still closes — but only for true absence. Esc alone must not green
    /// this pin: require the `read_attrs` match and **two** `close_attributes()` call sites in the
    /// host (Esc listener + None arm). `live_code` blanks comments; `live_source` keeps call paths.
    #[test]
    fn attributes_modal_none_arm_still_closes_on_true_absence() {
        let code = live_code(include_str!("attributes.rs"));
        let host = only_body(&code, "pub fn AttributesModal(");
        assert!(
            host.contains("read_attrs(&id)"),
            "AttributesModal must ask read_attrs for existence; body was:
{host}"
        );
        assert!(
            host.matches("close_attributes()").count() >= 2,
            "Esc path AND the None/undo-away arm must both call close_attributes; found {} in:
{host}",
            host.matches("close_attributes()").count()
        );
        let src = live_source(include_str!("attributes.rs"));
        let host_src = only_body(&src, "pub fn AttributesModal(");
        assert!(
            host_src.contains("read_attrs(&id)")
                && host_src.matches("close_attributes()").count() >= 2,
            "live_source pin: read_attrs + dual close_attributes must remain real call sites"
        );
    }

    /// The write path: both new fields land through `update_slot_object`, and `update_slot`'s three
    /// original columns are not dragged along by a commit that only touches a new one.
    #[test]
    fn attrs_update_slot_routes_the_new_fields_through_update_slot_object() {
        let ops = live_code(include_str!("editor_ops.rs"));
        let body = only_body(&ops, "pub fn attrs_update_slot(");
        assert!(
            body.contains("core.update_slot_object(id, asset_id, description)"),
            "the object half must go through the core mutator that leaves None keys alone"
        );
        assert!(
            body.contains("if role.is_some() || tag.is_some() || stance.is_some() {"),
            "a type-only or description-only commit must not open an update_slot transaction"
        );
    }

    /// T-745 Class-R: `attrs_update_slot` must no-op on all-None and on a missing id.
    ///
    /// Production already carries both guards (editor_ops.rs). Without this lasting pin a one-hunk
    /// revert ships green — the sibling route pin only requires `update_slot_object` / slot-half
    /// gating (wave-136 F1).
    ///
    /// RED: strip the five-field all-None early `return` before `let did`.
    /// RED: strip `!raw_slot_rows(core).contains_key(id) → false`.
    #[test]
    fn attrs_update_slot_noops_when_all_none_or_id_missing() {
        let ops = live_code(include_str!("editor_ops.rs"));
        let body = only_body(&ops, "pub fn attrs_update_slot(");

        // (1) five-field all-None early `return` before `let did`
        let before_did = body
            .split("let did")
            .next()
            .expect("attrs_update_slot must bind `let did`");
        for field in [
            "role.is_none()",
            "tag.is_none()",
            "stance.is_none()",
            "asset_id.is_none()",
            "description.is_none()",
        ] {
            assert!(
                before_did.contains(field),
                "T-745: all-None guard must check `{field}` before `let did`; prelude was:\n{before_did}"
            );
        }
        assert!(
            before_did.contains("return"),
            "T-745: all-None must early-return before `let did`; prelude was:\n{before_did}"
        );

        // (2) `!raw_slot_rows(core).contains_key(id)` → false arm
        let raw_gate = "!raw_slot_rows(core).contains_key(id)";
        assert!(
            body.contains(raw_gate),
            "T-745: missing id must gate on raw membership; body was:\n{body}"
        );
        let after_gate = body.split(raw_gate).nth(1).expect("raw gate present above");
        let brace = after_gate
            .find('{')
            .expect("T-745: raw gate must open an if-arm");
        let arm_tail = &after_gate[brace + 1..];
        let close = arm_tail.find('}').expect("T-745: raw-gate arm must close");
        let arm = arm_tail[..close].trim();
        assert!(
            arm.contains("return false")
                || arm.split_whitespace().collect::<Vec<_>>().join(" ") == "false",
            "T-745: raw absence arm must yield false (empty arm must RED); arm was:\n{arm}"
        );
    }

    /* ─────────── T-700 3DEN-PLACE-013 — the numeric nudge ─────────── */

    /// The step scale, exercised by CALLING it — the whole reason [`super::nudge_step`] lives
    /// outside the wasm block. Two properties, and the second is the one a refactor breaks:
    ///
    ///  1. each modifier alone selects its own step, and bare PageUp is 1;
    ///  2. the scale is FIRST-MATCH and finest-first, so **no combination of modifiers can produce
    ///     a step larger than the largest single modifier** — the safety argument the doc comment
    ///     makes. A multiplicative rewrite (`Shift`×`Alt` = 1000) fails this outright.
    #[test]
    fn nudge_step_is_first_match_finest_first_and_never_compounds() {
        use super::nudge_step;
        assert_eq!(nudge_step(false, false, false), 1.0, "bare PageUp is 1");
        assert_eq!(nudge_step(true, false, false), 0.1, "Ctrl is the fine step");
        assert_eq!(
            nudge_step(false, true, false),
            10.0,
            "Shift is the coarse step"
        );
        assert_eq!(
            nudge_step(false, false, true),
            100.0,
            "Alt is the coarsest step"
        );
        // Finest held wins, whichever else is down.
        assert_eq!(
            nudge_step(true, true, false),
            0.1,
            "Ctrl+Shift takes Ctrl's step"
        );
        assert_eq!(
            nudge_step(true, false, true),
            0.1,
            "Ctrl+Alt takes Ctrl's step"
        );
        assert_eq!(
            nudge_step(true, true, true),
            0.1,
            "all three take Ctrl's step"
        );
        assert_eq!(
            nudge_step(false, true, true),
            10.0,
            "Shift+Alt takes Shift's step"
        );
        // The property, over the whole 2^3 space: a combo never out-steps the single modifiers.
        let solo: f64 = [
            nudge_step(true, false, false),
            nudge_step(false, true, false),
            nudge_step(false, false, true),
            nudge_step(false, false, false),
        ]
        .into_iter()
        .fold(0.0, f64::max);
        for ctrl in [false, true] {
            for shift in [false, true] {
                for alt in [false, true] {
                    let s = nudge_step(ctrl, shift, alt);
                    assert!(
                        s <= solo,
                        "ctrl={ctrl} shift={shift} alt={alt} stepped {s}, larger than the biggest \
                         single-modifier step {solo} — the scale has started compounding"
                    );
                    assert!(s > 0.0, "a step must move the value");
                }
            }
        }
    }

    /// [`super::nudged`] — direction, quantisation, and the refusal that protects a multi-edit.
    #[test]
    fn a_nudge_quantises_and_refuses_a_field_with_no_truthful_base() {
        use super::{nudge_step, nudged};
        assert_eq!(nudged(Some(12.0), true, 1.0), Some(13.0));
        assert_eq!(nudged(Some(12.0), false, 1.0), Some(11.0));
        assert_eq!(nudged(Some(12.0), true, 10.0), Some(22.0));
        assert_eq!(nudged(Some(-3.0), false, 100.0), Some(-103.0));
        // A field the selection DISAGREES on renders empty; `"".parse::<f64>()` is Err, so the
        // caller hands us None and there must be NO write. Nudging from an implied 0 would stamp
        // an absolute number onto every selected entity.
        assert_eq!(
            nudged(None, true, 1.0),
            None,
            "an empty (multi-value) field has no base to be relative to — refuse the nudge"
        );
        assert_eq!(nudged(Some(f64::NAN), true, 1.0), None);
        assert_eq!(nudged(Some(f64::INFINITY), true, 1.0), None);
        // Quantisation: ten fine nudges off zero must land on 1, not 0.9999999999999999.
        let fine = nudge_step(true, false, false);
        let mut v = 0.0_f64;
        for _ in 0..10 {
            v = nudged(Some(v), true, fine).expect("a finite base nudges");
        }
        assert_eq!(
            v, 1.0,
            "ten Ctrl nudges off zero must land exactly on 1.0, got {v}"
        );
        assert_eq!(
            format!("{v}"),
            "1",
            "the quantised value is what the field displays and commits"
        );
    }

    /// The wiring, pinned where `cargo test` cannot reach: `number_field`'s keydown must consult
    /// the SAME gate the typed path does before it touches the draft.
    ///
    /// The ORDER is the assertion. `gate.locked_now()` has to be checked before `nudged(` is even
    /// called — a guard placed after the arithmetic would still be a guard, but one refactor away
    /// from writing first and asking later. And `locked_now` must carry `shut` first and
    /// unconditionally, exactly like the reactive `locked`, so the keyboard cannot become a second
    /// laxer opinion of "dead field" (the F-7 lie, re-told through a different input path).
    #[test]
    fn the_nudge_takes_the_same_refusal_gate_as_a_typed_edit() {
        let src = attrs_src();
        let field = only_body(&src, "fn number_field(");
        let guard = field
            .find("gate.locked_now()")
            .expect("number_field's keydown must consult the gate before nudging");
        let arith = field
            .find("nudged(")
            .expect("number_field must call the nudge arithmetic");
        assert!(
            guard < arith,
            "the refusal must be checked BEFORE the nudge is computed; guard at {guard}, \
             nudged( at {arith}"
        );
        let now = only_body(&src, "fn locked_now(self) -> bool");
        assert!(
            now.contains("self.shut || self.opt.is_some_and(|o| !o.get_untracked())"),
            "locked_now must short-circuit on `shut` exactly as `locked` does; body was:\n{now}"
        );
    }

    /// A nudge writes the DRAFT, never the document — the coalescing decision, stated as code.
    ///
    /// `number_field` must hold exactly ONE `on_commit(` call site, the one inside `commit`, so a
    /// burst of nudges cannot mint one undo step (and one modal re-render) per keypress. This is
    /// the assertion that goes red the moment someone "improves" the nudge into a live commit.
    #[test]
    fn a_nudge_writes_the_draft_and_leaves_the_commit_to_blur_or_enter() {
        let src = attrs_src();
        let field = only_body(&src, "fn number_field(");
        assert_eq!(
            field.matches("on_commit(").count(),
            1,
            "number_field must commit from exactly one place (the `commit` closure); a per-nudge \
             commit is one undo step per keypress against `capture_timeout_millis = 0`"
        );
        let commit = only_body(&src, "let commit = move ||");
        assert!(
            commit.contains("on_commit(n)"),
            "the single commit site is the blur/Enter closure; body was:\n{commit}"
        );
        assert!(
            field.contains("draft.set(format!("),
            "the nudge's only write is the local draft"
        );
    }

    /// The keys themselves. `live_source` KEEPS string literals — `ev.key()` is compared against
    /// literals, so this is the one pin that can see which keys are actually bound, and it must
    /// not be satisfiable by the doc comment that describes them.
    ///
    /// **wave-127 F-1 — the arrow keys and `step="any"` are part of this pin now.** A `type="number"`
    /// input with no `step` gets `step=1` on step base `0`, and the WHATWG step-up algorithm SNAPS an
    /// off-grid value onto the grid rather than adding to it: the browser's own ArrowUp on a focused
    /// `412.37` set the DOM value to `413`, fired `input`, and blur committed the integer — T-775's
    /// defect on the adjacent key. Losing EITHER half (the attribute or the interception) hands the
    /// arrows back to the browser, so both are asserted here.
    #[test]
    fn the_nudge_is_bound_to_the_page_and_arrow_keys_and_never_steps_on_a_grid() {
        let src = live_source(include_str!("attributes.rs"));
        let field = only_body(&src, "fn number_field(");
        for key in ["\"PageUp\"", "\"PageDown\"", "\"ArrowUp\"", "\"ArrowDown\""] {
            assert!(field.contains(key), "number_field must bind {key}");
        }
        // The arrows must share the nudge's OWN match arms — bound to some other handler they would
        // not take `nudge_step`, and (worse) might not prevent the browser's default stepping.
        for arm in [
            "\"PageUp\" | \"ArrowUp\" => true,",
            "\"PageDown\" | \"ArrowDown\" => false,",
        ] {
            assert!(
                field.contains(arm),
                "the arrow keys must enter the SAME nudge as the page keys; missing arm `{arm}` in \
                 body:\n{field}"
            );
        }
        assert!(
            field.contains("step=\"any\""),
            "the input must carry step=\"any\": with the default step=1 the browser's own arrow \
             keys and spinner snap an authored 412.37 onto the integer grid"
        );
        assert!(
            field.contains("ev.prevent_default()"),
            "PageUp/PageDown scroll by default and the arrows step by default — the nudge must \
             claim the key or the modal moves (or the value rounds) instead of the number"
        );
        // All three modifiers reach the step scale, in the argument order `nudge_step` declares.
        assert!(
            field.contains("nudge_step(ev.ctrl_key(), ev.shift_key(), ev.alt_key())"),
            "the step must be scaled from the live modifier state, in (ctrl, shift, alt) order"
        );
    }

    /// **T-775** — [`super::field_display`] is presentation, and presentation is allowed to round
    /// only because nothing commits it. What it may NOT do is what the old `format!("{}",
    /// value.round())` did: report an authored `412.37` as the integer `412`.
    #[test]
    fn the_display_keeps_the_working_resolution_and_never_flattens_to_an_integer() {
        use super::{field_display, nudge_step, nudged};
        // The ticket's own repro value. This assertion IS the bug.
        assert_eq!(
            field_display(412.37),
            "412.37",
            "an authored coordinate must not be displayed as an integer"
        );
        assert_eq!(field_display(412.371), "412.371");
        assert_eq!(field_display(-45.5), "-45.5");
        // Whole numbers stay tidy — the tidiness the old `.round()` was reaching for, kept.
        assert_eq!(field_display(412.0), "412");
        assert_eq!(field_display(4120.0), "4120");
        assert_eq!(field_display(0.0), "0");
        // No field may claim a negative zero.
        assert_eq!(field_display(-0.0), "0");
        assert_eq!(field_display(-0.0001), "0");
        // The display's precision is `nudged`'s quantum, so a nudged value always survives it
        // verbatim — the keyboard can never produce a number its own field cannot show.
        for step in [
            nudge_step(true, false, false),
            nudge_step(false, true, false),
            nudge_step(false, false, true),
            nudge_step(false, false, false),
        ] {
            let n = nudged(Some(412.37), true, step).expect("a finite base nudges");
            assert_eq!(
                field_display(n),
                format!("{n}"),
                "a nudge quantises to 3 decimals; the display must not round it further"
            );
        }
        // Past the working resolution the display DOES round — which is precisely why focus seeds
        // the draft from `exact` instead of from this string. Pinned below.
        assert_eq!(field_display(412.371_234_5), "412.371");
    }

    /// **T-775** — the two halves that make focusing and leaving a field a genuine no-op.
    ///
    /// A source pin because `number_field` is `#[cfg(target_arch = "wasm32")]` and `cargo test`
    /// cannot build it. Each half is useless without the other:
    ///   * FOCUS seeds the draft from `exact`, the full round-trip printing — not from the rounded
    ///     presentation. Seeding from the display is what made T-700's PageUp on `412.37` commit
    ///     `413` instead of `413.37`: the nudge inherited a rounding it never performed.
    ///   * BLUR skips `on_commit` when the parsed draft still equals the settled value. Nothing
    ///     downstream will do this for it — `editor_ops::attrs_update_position` writes and calls
    ///     `after_local_edit()` on every non-refused slot, so without this an idle click dirties the
    ///     mission and mints an undo step for a number nobody touched. (It also used to flatten a
    ///     manually authored Z; that is fixed at the caller now — wave-127 F-2, pinned below.)
    #[test]
    fn an_untouched_field_commits_nothing_and_the_draft_seeds_from_the_exact_value() {
        let src = attrs_src();
        let field = only_body(&src, "fn number_field(");
        assert!(
            !field.contains("value.round()"),
            "the field must not round the authored value away — that was the T-775 defect"
        );
        assert!(
            field.contains("StoredValue::new(field_display(value))"),
            "the unfocused display must go through `field_display`"
        );
        // `live_source` KEEPS literals: `format!("{value}")` is the assertion here, and `live_code`
        // blanks exactly the part that distinguishes it from a rounded format string.
        let live_src = live_source(include_str!("attributes.rs"));
        let live = only_body(&live_src, "fn number_field(");
        assert!(
            live.contains("let exact = StoredValue::new(format!(\"{value}\"));"),
            "`exact` must be the full round-trip printing of the value; body was:\n{live}"
        );
        let seed = only_body(&src, "let seed = move ||");
        assert!(
            seed.contains("exact.get_value()") && !seed.contains("shown.get_value()"),
            "focus must seed the draft from the EXACT value, never the presentation; body was:\n\
             {seed}"
        );
        assert!(
            field.contains("draft.set(seed())"),
            "the focus handler is what puts the exact value into the draft"
        );
        let commit = only_body(&src, "let commit = move ||");
        // wave-127 F-3 — the pin's job is now WIRING only: that `commit` asks `should_commit`, with
        // the gate's verdict and the settled value, before it writes. What the answer should BE is
        // decided by `should_commit_writes_only_a_new_finite_value` below, which CALLS the function.
        assert!(
            commit.contains("should_commit(gate.differs(), n, value)"),
            "commit must route the decision through `should_commit`, handing it the multi-edit \
             gate's verdict and the SETTLED value; body was:\n{commit}"
        );
        let guard = commit
            .find("should_commit(")
            .expect("the skip must gate the commit, not merely be computed");
        let call = commit
            .find("on_commit(n)")
            .expect("commit must still hold its one write");
        assert!(
            guard < call,
            "the no-op skip must be checked BEFORE the write; guard at {guard}, on_commit(n) at \
             {call}"
        );
    }

    /// **wave-127 F-3** — the blur/Enter decision, EXERCISED. Previously this lived as an inline
    /// expression inside a wasm-only `view!` closure and was guarded by a source pin on its literal
    /// text; with no wasm-bindgen-test harness in the repo, nothing ran it. A semantically identical
    /// rewrite turned that pin red and a subtly wrong one kept it green — the exact inversion of what
    /// a test is for.
    #[test]
    fn should_commit_writes_only_a_new_finite_value() {
        use super::should_commit;
        // The T-775 case: focus and leave an untouched field ⇒ no write, no undo step.
        assert!(
            !should_commit(false, 412.37, 412.37),
            "an idle focus/blur on an unchanged value must not commit"
        );
        assert!(!should_commit(false, 0.0, 0.0));
        // A real edit commits, however small — 3 decimals is the working resolution.
        assert!(should_commit(false, 412.371, 412.37));
        assert!(should_commit(false, -1.0, 1.0));
        // A DIFFERING field is exempt from the equality skip: under a multi-selection the settled
        // value is one arbitrary member's number, so typing it is a deliberate stamp on the rest.
        assert!(
            should_commit(true, 412.37, 412.37),
            "a differing (multi-value) field must commit even when the typed number equals the one \
             arbitrary member's value it was compared against"
        );
        assert!(should_commit(true, 5.0, 9.0));
        // Non-finite refuses under BOTH gate verdicts. `"inf"`/`"NaN"` parse as f64 and the core
        // filters them per axis, but a refused write still fires `after_local_edit()` at the caller
        // — a dirty mission and an undo step for a number that never landed.
        for differs in [false, true] {
            for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                assert!(
                    !should_commit(differs, n, 1.0),
                    "a non-finite draft must never commit (differs={differs}, n={n})"
                );
            }
        }
        // NaN is not equal to itself, so the equality skip alone would have LET IT THROUGH — the
        // finite check has to be its own rule, not a consequence of the comparison.
        assert!(
            !should_commit(false, f64::NAN, f64::NAN),
            "NaN != NaN, so the equality skip cannot be what stops a non-finite draft"
        );
    }

    /// **T-785 — `text_field` commits on BLUR/ENTER, never per keystroke.** This is the whole bug.
    ///
    /// A source pin because `text_field` is `#[cfg(target_arch = "wasm32")]` and there is no
    /// wasm-bindgen-test harness in this repo — the same reason `number_field`'s behaviour is pinned
    /// this way above. The defect it guards: `on:input=move |ev| on_change(...)` committed one store
    /// round-trip per character, each bumping `doc_tick`, which re-rendered the `AttributesModal`
    /// body and RE-CREATED this input mid-word. Focus fell to `<body>` and the tail of the word ran
    /// as editor chords. The fix is the `number_field` shape: a `focused`/`draft` split, `on:input`
    /// writing the DRAFT only, and the commit on `on:blur` / Enter.
    ///
    /// The assertions are the anti-regression: `on_change(` must NOT be reachable from `on:input`,
    /// and it must be reachable from the blur `commit` closure. `on:input` writing `draft.set(` is
    /// the positive half — a `text_field` with no `on:input` at all would pass a naive "no commit on
    /// input" check while making the field un-typeable.
    #[test]
    fn text_field_commits_on_blur_or_enter_and_never_remounts_mid_keystroke() {
        let src = attrs_src();
        let field = only_body(&src, "fn text_field(");
        // The remount cause, banned: the input handler must not commit. It writes the draft.
        assert!(
            field.contains("on:input=move |ev|")
                && field.contains("draft.set(event_target_value(&ev))")
                && field.contains("edited.set(true)"),
            "text_field's on:input must write the local DRAFT and set the edited latch, not commit;              body was:\n{field}"
        );
        // The commit lives behind blur/Enter, exactly like number_field. Exactly one `on_change(`
        // call site, and it is inside the `text_commit` closure the blur handler fires — a
        // per-keystroke commit is a per-keystroke remount against the modal's `doc_tick` re-render.
        assert_eq!(
            field.matches("on_change(").count(),
            1,
            "text_field must commit from exactly one place (the blur/Enter `text_commit` closure); a \
             per-input commit is what remounted the input and dropped focus to <body>. Body:\n{field}"
        );
        let commit = only_body(&src, "let text_commit = move ||");
        assert!(
            commit.contains("on_change(next)"),
            "the single commit site is the blur/Enter closure; body was:\n{commit}"
        );
        // Enter commits by blurring — one seam shared with the blur path, never a second `on_change`.
        // The key is a string LITERAL, so it is read from the literal-kept half (`attrs_src` /
        // `live_code` blanks it); `.blur()` is code and survives either way.
        let live_src = live_source(include_str!("attributes.rs"));
        let live_field = only_body(&live_src, "fn text_field(");
        assert!(
            live_field.contains("\"Enter\" =>") && field.contains(".blur()"),
            "Enter must commit by blurring the input (the shared seam), not by a second commit call"
        );
        // The focused/draft split itself: while focused the input shows the draft, and focus seeds
        // the draft. Without this the input is a plain `value=` again and the remount returns.
        assert!(
            field.contains(
                "prop:value=move || { if focused.get() { draft.get() } else { text_display() } }"
            ),
            "text_field must show the DRAFT while focused (the number_field split); body was:\n{field}"
        );
        assert!(
            field.contains("draft.set(text_display())") && field.contains("focused.set(true)"),
            "the focus handler must seed the draft and mark the field focused"
        );
    }

    /// **T-785** — the `text_field` no-op skip and the multi-edit contract, pinned together.
    ///
    /// An untouched focus/blur must not write (it would dirty the mission and mint an undo step, the
    /// same way `number_field`'s did before T-775). A DIFFERING multi-value field is EXEMPT from that
    /// skip — typing one member's string back is a deliberate stamp onto the whole selection — and it
    /// stays `disabled` (locked) until "Apply to all" is ticked, which the review flagged must not
    /// regress. `field_display`/`should_commit` are `number_field`'s; text has no parse, so the skip
    /// is a direct string comparison against the settled value.
    #[test]
    fn text_field_skips_the_no_op_write_but_a_differing_field_still_stamps() {
        let src = attrs_src();
        let field = only_body(&src, "fn text_field(");
        let commit = only_body(&src, "let text_commit = move ||");
        // T-813 — an operator-edited latch gates the write. A differing field alone must NOT
        // commit (that was wave200 F3: focus+blur stamped "" across the selection). Real input
        // sets the latch; Escape clears it before blur.
        assert!(
            field.contains("let edited = RwSignal::new(false)")
                && field.contains("edited.set(true)")
                && field.contains("edited.set(false)"),
            "text_field must latch real input edits; body was:\n{field}"
        );
        assert!(
            commit.contains("if !edited.get_untracked()")
                && commit.contains("gate.differs() || next != settled.get_value()"),
            "commit must require the edited latch, then skip an unchanged value yet still stamp a              differing (multi-value) field once edited; body was:\n{commit}"
        );
        // The multi-value display is EMPTY, never one arbitrary member's string — same rule as
        // number_field, and the seed for focus reads the same `text_display()`.
        let display = only_body(&src, "let text_display = move ||");
        assert!(
            display.contains("gate.differs()") && display.contains("String::new()"),
            "a differing field must render EMPTY, not one member's value; body was:\n{display}"
        );
        // The lock stays: disabled while the gate is locked (multi-edit not opted in / refused).
        assert!(
            field.contains("disabled=move || gate.locked()"),
            "text_field must stay disabled while the gate is locked — the 'Multiple values'              locked-state the review said must not regress"
        );
        // T-813 / wave200 F6 — field Escape must consume so the modal does not close on abandon.
        let live = live_source(include_str!("attributes.rs"));
        let live_field = only_body(&live, "fn text_field(");
        assert!(
            live_field.contains("\"Escape\" =>") && live_field.contains("stop_propagation()"),
            "text_field Escape must stop_propagation so the modal stays open; body was:\n{live_field}"
        );
        let live_num = only_body(&live, "fn number_field(");
        assert!(
            live_num.contains("key == \"Escape\"") && live_num.contains("stop_propagation()"),
            "number_field Escape must stop_propagation (same family); body was:\n{live_num}"
        );
    }

    /// **T-785 — the chord guard reads the LIVE `document.activeElement` directly.** This is the last
    /// line of defence F-26-root asked to harden: every editor chord (E/R docks, Space camera, G
    /// snap, Ctrl+A, copy/paste) sits behind `mission_history::in_editable_field()`, so whatever it
    /// returns decides "typed character" vs "chord". It must read the tag and contentEditable state
    /// off `activeElement` at the moment of the keypress — never a cached "is a field open?" flag —
    /// so a field that has lost focus can never keep swallowing keys, and a chord can never fire
    /// while a field genuinely holds focus.
    ///
    /// A cross-file source pin because `mission_history` is `#[cfg(target_arch = "wasm32")]` and does
    /// not compile under native `cargo test`; `attributes` does, and reads it as a string here — the
    /// same shape as the `editor_ops.rs` pins in this module.
    #[test]
    fn the_chord_guard_reads_active_element_tag_and_content_editable_directly() {
        let mh = live_code(include_str!("mission_history.rs"));
        let body = only_body(&mh, "pub fn in_editable_field() -> bool");
        // The source of truth is the LIVE focused node, fetched every call.
        assert!(
            body.contains("active_element()"),
            "in_editable_field must read document.activeElement, not a cached flag; body:\n{body}"
        );
        // Native form controls by tag, and contentEditable hosts by property — both direct off the
        // element, so focus loss to <body> (neither) reads as "not editable" the instant it happens.
        assert!(
            body.contains("tag_name()") && body.contains("is_content_editable"),
            "in_editable_field must check the element tag AND contentEditable directly; body:\n{body}"
        );
        // It must NOT gate on a cached "is a field open?" flag — reading the live activeElement is
        // the whole point, so a field that has lost focus stops swallowing keys immediately. (The
        // editor keydown's own guard call is pinned by mission_editor's tests, which own that file.)
        assert!(
            !body.contains("attrs_open") && !body.contains("renaming"),
            "in_editable_field must not consult an 'is a field open' signal — read activeElement live"
        );
    }

    /// **wave-127 F-2** — an Attributes x/y edit must not silently discard an authored Z.
    ///
    /// `update_slot_position` terrain-follows on any x/y write (`pz = 0.0` when `z` is `None`). That
    /// matches the JS oracle, whose caller then re-samples the DEM — but NOTHING re-samples after an
    /// Attributes commit in this frontend, so the `0.0` was final: an operator who authored a rooftop
    /// Z lost it the moment they nudged X by a metre, inside the same undo step as the X edit.
    ///
    /// The fix is pinned at the FRONTEND CALLERS deliberately. `map-engine-core`'s mutator keeps its
    /// documented byte-parity with `ydoc.updateSlotPosition`; these two functions read the current Z
    /// and pass it back in, which makes the follow a no-op for this path alone.
    ///
    /// A source pin because `editor_ops` is wasm32-only and `cargo test` cannot build it.
    #[test]
    fn an_attributes_x_or_y_commit_carries_the_slots_current_z_back_in() {
        let ops = live_code(include_str!("editor_ops.rs"));
        // Single-slot still goes through update_slot_position.
        {
            let f = "pub fn attrs_update_position(";
            let body = only_body(&ops, f);
            let resolve = body.find("z.or_else(").unwrap_or_else(|| {
                panic!("{f} must resolve a missing z before writing; body was:\n{body}")
            });
            let write = body
                .find("core.update_slot_position(")
                .unwrap_or_else(|| panic!("{f} must still write through the core mutator"));
            assert!(
                resolve < write,
                "{f} must resolve the sticky z BEFORE the write; resolve at {resolve}, write at \
                 {write}"
            );
            assert!(
                body.contains("slot_z("),
                "{f} must read the slot's CURRENT z, not invent one; body was:\n{body}"
            );
        }
        // T-732 — multi stamps via update_entity_transforms; sticky-z still before the batch.
        {
            let f = "pub fn attrs_update_position_multi(";
            let body = only_body(&ops, f);
            let resolve = body.find("z.or_else(").unwrap_or_else(|| {
                panic!("{f} must resolve a missing z before writing; body was:\n{body}")
            });
            let write = body.find("update_entity_transforms(").unwrap_or_else(|| {
                panic!("{f} must commit via update_entity_transforms (T-732 atomic batch)")
            });
            assert!(
                resolve < write,
                "{f} must resolve the sticky z BEFORE the batch write; resolve at {resolve}, write \
                 at {write}"
            );
            assert!(
                body.contains("slot_z("),
                "{f} must read the slot's CURRENT z, not invent one; body was:\n{body}"
            );
            let per_id = ["core.update_slot", "_position(id,"].concat();
            assert!(
                !body.contains(&per_id),
                "T-732: {f} must not call per-id update_slot_position (N undo steps)"
            );
        }
        // The read is conditional on the commit being able to zero a z at all — an explicit z write
        // or a rotation-only edit must not pay for an O(document) JSON read.
        let rows = only_body(&ops, "fn keep_z_rows(");
        assert!(
            rows.contains(
                "(z.is_none() && (x.is_some() || y.is_some())).then(|| raw_slot_rows(core))"
            ),
            "keep_z_rows must read the rows exactly when an x/y edit would otherwise zero the z; \
             body was:\n{rows}"
        );
        // And the read is off the EXACT raw row, not the materialized SoA: the SoA's `zs` is f32 (a
        // round-trip would rewrite the authored value) and it OMITS slots on hidden layers (T-665),
        // where a failed read is a zeroed z.
        let live_ops = live_source(include_str!("editor_ops.rs"));
        let read = only_body(&live_ops, "fn slot_z(");
        assert!(
            read.contains("\"position\"") && read.contains("\"z\""),
            "slot_z must read `position.z` off the raw slot row; body was:\n{read}"
        );
        assert!(
            !read.contains("materialize"),
            "slot_z must not go through the f32 SoA — it drops hidden-layer slots and rounds the \
             value it exists to preserve; body was:\n{read}"
        );
    }

    /// **wave-127 F-5** — the PLACEMENT commands must not flatten an authored Z either.
    ///
    /// Same defect as F-2 above, one path over: `editor_ops::commit_positions` — the shared commit
    /// behind the top strip's Align, Distribute and placement-pattern menus — used to write every
    /// slot as `update_slot_position(.., Some(x), Some(y), None, ..)`, the exact shape the core
    /// mutator terrain-follows to `pz = 0.0`. T-732 now batches through
    /// `MissionDocCore::update_entity_transforms`, but the sticky-z resolution must still happen
    /// BEFORE the batch is built.
    ///
    /// Pinned here, beside its sibling, for the same reason: `editor_ops` is `wasm32`-only, so no
    /// test inside it is built by the native harness.
    #[test]
    fn a_placement_commit_carries_each_slots_current_z_back_in() {
        let ops = live_code(include_str!("editor_ops.rs"));
        let body = only_body(&ops, "fn commit_positions(");
        // The old zeroing write, verbatim: x and y set, z hard-coded absent.
        assert!(
            !body.contains("Some(t.x), Some(t.y), None, None"),
            "commit_positions must not write `z = None` on an x/y move — that stores pz = 0.0 and \
             the authored z is gone; body was:\n{body}"
        );
        let read = body.find("slot_z(").unwrap_or_else(|| {
            panic!("commit_positions must read each slot's CURRENT z, not invent one; body was:\n{body}")
        });
        let write = body.find("update_entity_transforms(").unwrap_or_else(|| {
            panic!("commit_positions must commit via update_entity_transforms (T-732 atomic batch)")
        });
        assert!(
            read < write,
            "commit_positions must resolve the sticky z BEFORE the batch write; read at {read}, \
             write at {write}"
        );
        // The rows are an O(document) JSON parse and this commits k entities, so the batch read must
        // be HOISTED above the per-entity loop and happen exactly once.
        let rows = body.find("keep_z_rows(").unwrap_or_else(|| {
            panic!(
                "commit_positions must resolve its rows through keep_z_rows — \
                 a second z-resolution path is its own defect; body was:\n{body}"
            )
        });
        let loop_at = body.find("for (e, t) in").unwrap_or_else(|| {
            panic!("commit_positions must still walk entities/targets pairwise")
        });
        assert!(
            rows < loop_at,
            "the keep_z_rows read must be hoisted ABOVE the per-entity loop (read at {rows}, loop \
             at {loop_at}) — one O(document) parse per BATCH, not per entity"
        );
        assert_eq!(
            body.matches("keep_z_rows(").count(),
            1,
            "exactly one keep_z_rows call — more than one means the document is re-parsed per \
             entity; body was:\n{body}"
        );
        // Vehicle branch still carries its own z into the patch.
        assert!(
            body.contains("z: Some(e.z)") || body.contains("z: Some(e.z,"),
            "the vehicle branch must still pass the vehicle's own z through; body was:\n{body}"
        );
        assert!(
            body.contains("is_slot: false"),
            "vehicle patches must be marked is_slot: false; body was:\n{body}"
        );
    }

    /// **T-777** — and neither may PASTE. A copy lands at the elevation it was copied from.
    ///
    /// Third path in the same family as F-2 and F-5 above. `editor_ops::paste_at_cursor` pushed a
    /// hard-coded ground value into `paste_slots`' `zs` column for every clipboard row, justified
    /// in a comment as byte-parity with the flat-map JS oracle. The **operator set that parity
    /// aside on 2026-08-08** — it was a migration safety net, never a contract — and the zero was
    /// FINAL either way: the oracle's caller re-sampled the DEM and wrote the real elevation back,
    /// nothing in this frontend does, so copying a rooftop entity dropped the copy to the ground
    /// inside the paste's own undo step.
    ///
    /// A SOURCE pin for the same reason as its two siblings: `editor_ops` is
    /// `#![cfg(target_arch = "wasm32")]`, so the native harness builds nothing inside it and no
    /// test here can call the function. The other half — that a non-zero elevation carried across
    /// the seam actually survives into the document, and that a multi-slot paste does not hand one
    /// entity another's z — is a live native test at
    /// `crates/map-engine-core/tests/paste_keeps_authored_z.rs`. Neither half is sufficient alone:
    /// this one cannot see the document, that one cannot see which value the frontend chooses.
    #[test]
    fn a_paste_carries_each_copied_slots_authored_z_into_the_copy() {
        let ops = live_code(include_str!("editor_ops.rs"));
        // FILE-WIDE, not scoped to the paste body: the failure mode is the literal coming back, and
        // it does not have to come back in the function it was removed from.
        let flattening_push = ["zs.push(", "0.0)"].concat();
        assert!(
            !ops.contains(&flattening_push),
            "no path may push a hard-coded ground elevation into a zs column — nothing re-samples \
             terrain afterwards, so that value is what the operator is left with"
        );
        // The overruled rationale must not survive as a live comment either; it would send the next
        // reader to restore the behaviour the operator just removed. Checked on RAW source because
        // `live_code` strips exactly the thing under test.
        assert!(
            !include_str!("editor_ops.rs").contains("DEM not ready"),
            "the paste's parity rationale was overruled on 2026-08-08 and must not be left standing"
        );

        let body = only_body(&ops, "pub fn paste_at_cursor(");
        // Resolved through the SHARED reader. A second z-resolution vocabulary is its own defect
        // class here — F-2, F-5, F-6 and this path must all read a z the same way.
        assert!(
            body.contains("slot_z("),
            "paste_at_cursor must read each copied slot's authored z, not invent one; body was:\n\
             {body}"
        );
        // Resolved ONCE for the whole paste and HOISTED above the per-slot walk: building the
        // lookup inside the loop would be quadratic in the paste size.
        let rows = body.find("let z_rows").unwrap_or_else(|| {
            panic!("paste_at_cursor must resolve its z lookup up front; body was:\n{body}")
        });
        let loop_at = body
            .find("for slot in &clip")
            .unwrap_or_else(|| panic!("paste_at_cursor must still walk the clipboard rows"));
        assert!(
            rows < loop_at,
            "the z lookup must be built ABOVE the per-slot loop (built at {rows}, loop at \
             {loop_at}) — once per PASTE, not once per slot"
        );
        assert_eq!(
            body.matches("let z_rows").count(),
            1,
            "exactly one z lookup; a second one means a second z-resolution path"
        );

        // ORDER CORRESPONDENCE — the whole reason this fix can be worse than the bug if it is
        // wrong. `zs[i]` must be the elevation of the row that minted `ids[i]`, and a mismatched
        // zip hands one entity another's elevation while looking perfectly green.
        //
        // Proved structurally, not by convention: BOTH pushes live inside the ONE walk over
        // `clip`, and each occurs EXACTLY ONCE in the whole function. One iteration therefore
        // appends exactly one element to each vector, in lockstep — a total, order-preserving map
        // from clipboard row to (id, z) pair. No zip, no id lookup, no second source to drift.
        let per_slot = only_body(&body, "for slot in &clip");
        assert!(
            per_slot.contains("ids.push(mint_id("),
            "the id mint must stay inside the clipboard walk; loop body was:\n{per_slot}"
        );
        assert!(
            per_slot.contains("zs.push("),
            "the z push must sit in the SAME iteration as the id mint, or the two vectors are only \
             conventionally aligned; loop body was:\n{per_slot}"
        );
        for (needle, what) in [("ids.push(mint_id(", "id mint"), ("zs.push(", "z push")] {
            assert_eq!(
                body.matches(needle).count(),
                1,
                "exactly one {what} in paste_at_cursor — a second one appends off-cadence and \
                 shifts every later index by one"
            );
        }
        // And nothing re-orders either vector between the walk that builds them and the single
        // hand-off. Scoped to that REGION on purpose: this asserts a property OF the region, not a
        // ban on a token (those are file-wide, see the top of this test).
        let hand_off = body.find("core.paste_slots(").unwrap_or_else(|| {
            panic!("paste_at_cursor must still commit through the bulk paste mutator")
        });
        assert!(
            loop_at < hand_off,
            "the parallel arrays must be built before they are handed off"
        );
        for reorder in [
            ".sort",
            ".reverse(",
            ".dedup",
            ".retain(",
            ".swap(",
            ".rotate_",
        ] {
            assert!(
                !body[loop_at..hand_off].contains(reorder),
                "nothing may `{reorder}` between building the paste arrays and handing them to \
                 paste_slots — the index correspondence is the contract"
            );
        }
    }

    /* ─────────── T-741 — multi-edit header honesty (wave-112 NIT-4) ─────────── */

    /// Behaviour pin: mixed slot+vehicle selection must name the SLOT write set and disclose
    /// that vehicles are excluded. RED under the original overclaim ("N entities selected") and
    /// under a hollow that counts the FULL selection as the header number.
    #[test]
    fn attrs_multi_subtitle_counts_slots_and_names_excluded_vehicles() {
        // Original defect: 2 slots + 3 vehicles under Ctrl+A.
        assert_eq!(
            super::attrs_multi_subtitle(2, 5),
            "2 slots selected · multi-edit · vehicles excluded"
        );
        // Slot-only multi-edit stays terse.
        assert_eq!(
            super::attrs_multi_subtitle(3, 3),
            "3 slots selected · multi-edit"
        );
        // Hollow shape 1 — original overclaim wording (filtered count, but "entities").
        assert_ne!(
            super::attrs_multi_subtitle(2, 5),
            "2 entities selected · multi-edit"
        );
        // Hollow shape 2 — counting the FULL selection as the write-set size.
        assert_ne!(
            super::attrs_multi_subtitle(2, 5),
            "5 entities selected · multi-edit"
        );
        assert_ne!(
            super::attrs_multi_subtitle(2, 5),
            "5 slots selected · multi-edit"
        );
    }

    /// Wiring pin (`live_code` / `only_body`): the modal must CALL the honesty helper with both
    /// the filtered slot count and the live selection length — an inlined old `format!` cannot
    /// satisfy this (literals blanked; the call shape is what remains).
    ///
    /// Wave-137 F1: also pin value *flow* through `AttributesModal` — a dead
    /// `let _ = attrs_selection_len()` (hollow B) or binding `selection_n` then passing
    /// `multi.len()` / `multi_n` into `modal_view` (hollow B2) must go RED.
    #[test]
    fn modal_view_routes_multi_subtitle_through_the_honesty_helper() {
        let src = attrs_src();
        let body = only_body(&src, "fn modal_view(");
        assert!(
            body.contains("attrs_multi_subtitle(multi_n, selection_n)"),
            "modal_view must format the multi header via attrs_multi_subtitle(multi_n, selection_n); body was:\n{body}"
        );
        let host = only_body(&src, "pub fn AttributesModal(");
        // (1) Assignment, not a dead call — hollow B (`let _ = attrs_selection_len(); let selection_n = multi.len()`) RED.
        assert!(
            host.contains("let selection_n = crate::editor_ops::attrs_selection_len()"),
            "AttributesModal must bind `let selection_n = crate::editor_ops::attrs_selection_len()` (not a discarded call); body was:\n{host}"
        );
        // (2) That binding must be the modal_view selection-length argument — hollow B2
        // (`modal_view(..., multi.len(), ...)` while keeping the binding) RED.
        let compact = host.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            compact.contains("modal_view( attrs, multi, selection_n,")
                || compact.contains("modal_view(attrs, multi, selection_n,"),
            "AttributesModal must pass `selection_n` into modal_view (not multi.len()/multi_n); body was:\n{host}"
        );
    }

    /// Copy pin (`live_source`): banner says "every selected slot", never the overclaiming
    /// "every selected entity". Second hollow: the old header format string must be gone from
    /// `modal_view`.
    #[test]
    fn multi_edit_copy_names_slots_not_every_selected_entity() {
        let src = live_source(include_str!("attributes.rs"));
        let body = only_body(&src, "fn modal_view(");
        assert!(
            body.contains("every selected slot"),
            "differing-fields banner must say every selected slot; body was:\n{body}"
        );
        assert!(
            !body.contains("every selected entity"),
            "banner must not overclaim vehicles as editable entities; body was:\n{body}"
        );
        assert!(
            !body.contains("entities selected · multi-edit"),
            "modal_view must not keep the old entities-selected header format; body was:\n{body}"
        );
    }

    /// `attrs_multi_ids` still filters to SoA slot ids — the subset the header is honest about.
    #[test]
    fn attrs_multi_ids_still_filters_selection_to_slot_soa() {
        let ops = live_code(include_str!("editor_ops.rs"));
        let body = only_body(&ops, "pub fn attrs_multi_ids(open_id: &str) -> Vec<String>");
        assert!(
            body.contains("soa.ids.iter().any(|r| r == s)"),
            "attrs_multi_ids must keep filtering to slot SoA ids; body was:\n{body}"
        );
        let sel = only_body(&ops, "pub fn attrs_selection_len() -> usize");
        assert!(
            sel.contains("ctx.selection.borrow().len()"),
            "attrs_selection_len must read the live selection length; body was:\n{sel}"
        );
    }
}

/// T-726 — Attributes modal Esc through the modal stack.
#[cfg(test)]
mod t726_attributes_esc_stack {
    use crate::arsenal::class_r_scrub::{live_code, only_body};

    #[test]
    fn attributes_modal_gates_escape_on_modal_stack() {
        let code = live_code(include_str!("attributes.rs"));
        let body = only_body(&code, "pub fn AttributesModal(");
        let reg = ["modal_stack", "::", "register("].concat();
        let top = ["modal_stack", "::", "is_topmost_open(modal_id)"].concat();
        let unreg = ["modal_stack", "::", "unregister(modal_id)"].concat();
        assert!(body.contains(&reg), "T-726: AttributesModal must register");
        assert!(
            body.contains(&top),
            "T-726: AttributesModal must gate Escape on is_topmost_open"
        );
        assert!(
            body.contains(&unreg),
            "T-726: AttributesModal must unregister"
        );
    }
}

/// T-807 — the Transform-tab copy debts: the stale DEM hint (F-23) and the coordinate unit suffix
/// (F-13). Both are about user-visible strings, so these pin on `live_source` (string literals
/// KEPT — `live_code` would blank the very copy under test and make the pin hollow, the T-759 class).
#[cfg(test)]
mod t807_transform_tab_copy {
    use crate::arsenal::class_r_scrub::{live_source, only_body};

    /// F-23 — DEM shipped, so the "Z is manual until terrain elevation (DEM) ships" hint is stale.
    /// The old promise must be gone from the whole live source.
    #[test]
    fn stale_dem_manual_hint_is_gone() {
        let code = live_source(include_str!("attributes.rs"));
        // Concat so this test's own literal cannot self-match.
        let stale = ["Z is manual until terrain elevation ", "(DEM) ships"].concat();
        assert!(
            !code.contains(&stale),
            "F-23: the stale 'Z is manual until DEM ships' hint must be replaced (DEM has shipped)"
        );
    }

    /// F-13 — the X/Y/Z coordinate fields carry a metre unit suffix (Rotation already carried `°`).
    /// Pinned to `transform_tab`'s body so it is the coordinate fields, not a stray literal.
    #[test]
    fn coordinate_fields_suffix_metres() {
        let code = live_source(include_str!("attributes.rs"));
        let body = only_body(&code, "fn transform_tab(");
        // Three coordinate fields, each `Some("m")`; Rotation keeps its own `Some("°")`.
        let m_suffix = body.matches("Some(\"m\")").count();
        assert!(
            m_suffix >= 3,
            "F-13: X/Y/Z must each pass Some(\"m\") as the unit suffix (found {m_suffix})"
        );
        // The bare `None` suffix on a coordinate field is exactly the pre-fix state.
        assert!(
            body.contains("Some(\"\u{b0}\")"),
            "F-13: Rotation must still carry its ° suffix"
        );
    }
}

/// T-810 (F-23) — the searchable TYPE picker, the Revert affordance, and Eden's axis colours.
#[cfg(test)]
mod t810_type_picker_revert_axes {
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};

    /// F-23 (c) — the axis labels carry THREE DISTINCT colours (X/Y/Z), plus a fourth for Rotation,
    /// and no other field is tinted. This is the acceptance's "3 distinct colours" pinned by CALLING
    /// the pure mapping (the reason [`super::axis_chip_class`] lives outside the wasm block, like
    /// `nudge_step`) rather than scraping a `view!` string.
    #[test]
    fn axis_chip_class_is_three_distinct_axis_colours_plus_rotation() {
        use super::axis_chip_class;
        let x = axis_chip_class("X").expect("X is coloured");
        let y = axis_chip_class("Y").expect("Y is coloured");
        let z = axis_chip_class("Z").expect("Z is coloured");
        let rot = axis_chip_class("Rotation").expect("Rotation is coloured");
        // Three DISTINCT axis colours (the acceptance counts three).
        let mut axes = vec![x, y, z];
        axes.sort_unstable();
        axes.dedup();
        assert_eq!(
            axes.len(),
            3,
            "X/Y/Z must be three distinct colours; got {x}/{y}/{z}"
        );
        // Rotation is its own hue, not a re-used axis colour.
        assert!(
            rot != x && rot != y && rot != z,
            "Rotation must carry a fourth distinct chip"
        );
        // Every NON-axis field is untinted — the chip rides only the spatial rows.
        for other in [
            "Role",
            "Tag",
            "Type",
            "Stance",
            "Role Description",
            "Squad",
            "",
        ] {
            assert!(
                axis_chip_class(other).is_none(),
                "{other} must not get an axis chip"
            );
        }
    }

    /// F-23 (c) — the chip rides the LABEL, not the value, and it is decorative (`aria-hidden`). Pin
    /// `field_label`'s body: it must consult `axis_chip_class(label)` and mark the swatch aria-hidden.
    #[test]
    fn the_axis_chip_rides_the_label_and_is_aria_hidden() {
        let code = live_code(include_str!("attributes.rs"));
        let body = only_body(&code, "fn field_label(");
        assert!(
            body.contains("axis_chip_class(label)"),
            "field_label must derive the chip from the label; body was:\n{body}"
        );
        let src = live_source(include_str!("attributes.rs"));
        let body_src = only_body(&src, "fn field_label(");
        assert!(
            body_src.contains("aria-hidden"),
            "the colour swatch must be aria-hidden — a screen reader must not announce a bare colour"
        );
    }

    /// F-23 (a) — the TYPE field is a PICKER, and the T-082 wiring the pins protect stays in
    /// `identity_tab`: it calls `type_picker` (not `text_field`) for Type, still reads
    /// `a.asset_id.clone()`, still takes the multi-edit gate, and still routes the ONE commit seam.
    /// (`identity_tab_commits_type_and_role_description_through_their_own_argument_slots` above still
    /// pins the exact `commit_slot(...)` shape; this pins that the ENTRY became the picker.)
    #[test]
    fn the_type_field_is_the_searchable_picker_not_a_freetext_field() {
        let code = live_code(include_str!("attributes.rs"));
        let body = only_body(&code, "fn identity_tab(");
        assert!(
            body.contains("type_picker("),
            "the Type entry must be `type_picker`, not a bare text_field; body was:\n{body}"
        );
        // The picker still receives the live catalog and the gate, and still commits via commit_slot.
        assert!(
            body.contains("registry_items") && body.contains("g(diff.asset_id, opts.asset_id)"),
            "the picker must take the live catalog and the asset_id multi-edit gate"
        );
        // The commit seam is unchanged (also pinned by the T-082 test) — a picked leaf lands in the
        // asset_id slot alone.
        assert!(
            body.contains("commit_slot(targets, None, None, None, Some(asset_id), None)"),
            "the picker must commit into the asset_id slot alone, exactly as the field did"
        );
    }

    /// F-23 (a) — the picker keeps the freetext escape hatch (the T-785 `text_field`) behind an
    /// Advanced affordance, so an unlisted id is still typeable. `type_picker`'s body must still call
    /// `text_field` (the freetext half) and offer an "advanced" control.
    #[test]
    fn the_picker_keeps_a_freetext_advanced_escape_hatch() {
        let code = live_code(include_str!("attributes.rs"));
        let body = only_body(&code, "fn type_picker(");
        assert!(
            body.contains("text_field("),
            "type_picker must keep the T-785 text_field for unlisted ids; body was:\n{body}"
        );
        let src = live_source(include_str!("attributes.rs"));
        let body_src = only_body(&src, "fn type_picker(");
        assert!(
            body_src.to_lowercase().contains("advanced"),
            "the freetext field must sit behind an 'advanced' affordance"
        );
        // Empty is a first-class option: the picker offers a clear-to-faction-default row.
        assert!(
            body_src.contains("Faction default"),
            "empty = faction default must stay a first-class, reachable choice"
        );
    }

    /// F-23 (a) — the empty catalog shows CAUSE + RETRY, never a dead list, MIRRORING the T-800
    /// dock vocabulary (`eden_dock_right::catalog_failure_view`). The empty branch turns on
    /// `catalog_leaf_count == 0` and offers a retry.
    #[test]
    fn the_empty_catalog_shows_cause_and_retry_not_a_dead_list() {
        let code = live_code(include_str!("attributes.rs"));
        let body = only_body(&code, "fn type_picker(");
        assert!(
            body.contains("catalog_leaf_count("),
            "the empty state must key on catalog_leaf_count, not render a bare list"
        );
        let src = live_source(include_str!("attributes.rs"));
        let body_src = only_body(&src, "fn type_picker(");
        // The mirrored T-800 cause + a Retry control.
        assert!(
            body_src.contains("No modpack is configured"),
            "the empty state must state the cause (mirrored T-800 vocabulary)"
        );
        assert!(
            body_src.contains("Retry") && body.contains("reload()"),
            "the empty state must offer a working Retry (a full reload re-runs /registry)"
        );
    }

    /// F-23 (a) — ESC LAYERING: the picker popover's own Escape closes the POPOVER first and CONSUMES
    /// the event (`stop_propagation`), so the modal's window listener does not also close the modal on
    /// the same press. Layer order: picker → field (the advanced text_field's own Esc) → modal.
    #[test]
    fn the_picker_popover_consumes_its_own_escape_first() {
        let code = live_code(include_str!("attributes.rs"));
        let body = only_body(&code, "fn type_picker(");
        // Find the popover's keydown handler and prove it stops propagation AND closes the popover.
        assert!(
            body.contains("stop_propagation()"),
            "the popover's Escape must stop_propagation so the modal listener does not fire too"
        );
        assert!(
            body.contains("open.set(false)"),
            "the popover's Escape must close the popover layer itself"
        );
        // The advanced field is the T-785 text_field, which carries its OWN Escape (field layer) —
        // pinned already by `text_field_commits_on_blur_or_enter_and_never_remounts_mid_keystroke`
        // and the modal's Esc is the third layer (`attributes_modal_none_arm_still_closes...`).
    }

    /// F-23 (b) — REVERT restores the on-open snapshot as REAL writes, PER SLOT (not the homogeneous
    /// T-788 batch, which would flatten a differing multi-selection onto one member's value).
    /// `revert_to_snapshot` must walk the snapshot and call the SINGLE-slot commit ops per entry.
    #[test]
    fn revert_restores_per_slot_through_the_single_slot_commit_ops() {
        let code = live_code(include_str!("attributes.rs"));
        let body = only_body(&code, "fn revert_to_snapshot(");
        assert!(
            body.contains("attrs_update_position(") && body.contains("attrs_update_slot("),
            "Revert must restore BOTH the transform half and the identity/type half; body was:\n{body}"
        );
        // Per-slot (a loop over the snapshot), NOT the `_multi` batch — the batch is homogeneous and
        // cannot express each slot's own pre-open value.
        assert!(
            body.contains("for ") && body.contains("snapshot.get_value()"),
            "Revert must iterate the snapshot per slot"
        );
        assert!(
            !body.contains("_multi("),
            "Revert must NOT use the homogeneous multi batch — it would flatten differing slots"
        );
    }

    /// F-23 (b) — the snapshot is READ AT OPEN (the T-082 lesson): the capture effect tracks
    /// `attrs_open` and NOT `doc_tick`, so a live edit does not overwrite the pre-open values, and the
    /// modal STATES the model in one line. Pin the host body.
    #[test]
    fn the_revert_snapshot_is_captured_on_open_and_the_model_is_stated() {
        let code = live_code(include_str!("attributes.rs"));
        let host = only_body(&code, "pub fn AttributesModal(");
        // The capture reads read_attrs into the snapshot store, driven by the attrs_open effect.
        assert!(
            host.contains("snapshot.set_value("),
            "AttributesModal must capture the snapshot into its store on open"
        );
        let src = live_source(include_str!("attributes.rs"));
        let modal = only_body(&src, "fn modal_view(");
        // The one-line model statement lives in the panel (per spec).
        assert!(
            modal.contains("Edits apply live. Revert restores"),
            "the panel must state the revert model in one line"
        );
        // And a Revert control exists and calls the restore.
        let modal_code = only_body(&code, "fn modal_view(");
        assert!(
            modal_code.contains("revert_to_snapshot(snapshot)"),
            "the Revert button must call revert_to_snapshot"
        );
    }

    /* ─────────── T-818 — vehicle Attributes gains the dock Placed editor ─────────── */

    /// Host route: when `read_attrs` is None, a vehicle id opens the vehicle editor instead of
    /// closing. A hollow rewrite that keeps `close_attributes` on every None path goes RED.
    #[test]
    fn attributes_modal_routes_vehicles_to_the_vehicle_editor() {
        let code = live_code(include_str!("attributes.rs"));
        let host = only_body(&code, "pub fn AttributesModal(");
        assert!(
            host.contains("is_vehicle_id(&id)") && host.contains("vehicle_attrs_view("),
            "T-818: AttributesModal None arm must route vehicles to vehicle_attrs_view; body was:\n{host}"
        );
        assert!(
            host.contains("close_attributes()"),
            "true absence must still close; body was:\n{host}"
        );
    }

    /// The moved controls call the SAME mutators the Placed strip used — digest/undo parity.
    /// Heading commits through number_field (T-785), not a raw on:change input.
    #[test]
    fn vehicle_attrs_view_wires_heading_cargo_crew_through_existing_mutators() {
        let code = live_code(include_str!("attributes.rs"));
        let body = only_body(&code, "fn vehicle_attrs_view(");
        for needle in [
            "set_vehicle_heading(",
            "set_vehicle_cargo(",
            "assign_crew_seat(",
            "clear_crew_seat(",
            "number_field(",
            "Gate::open()",
            "placed_slot_choices()",
            "seat_model(",
        ] {
            assert!(
                body.contains(needle),
                "T-818: vehicle_attrs_view must contain `{needle}`; body was:\n{body}"
            );
        }
        // Operator-visible section labels survive on live_source (live_code blanks string literals).
        let src = live_source(include_str!("attributes.rs"));
        let live = only_body(&src, "fn vehicle_attrs_view(");
        for label in ["\"Heading\"", "\"Add cargo\"", "\"Crew\"", "\"Cargo\""] {
            assert!(
                live.contains(label),
                "T-818: vehicle editor must show {label}; body was:\n{live}"
            );
        }
        // Multi-edit machinery must stay untouched for vehicles.
        assert!(
            !body.contains("attrs_multi_ids") && !body.contains("Gate::maybe"),
            "T-818: vehicle editor must not pull in T-649 multi-edit; body was:\n{body}"
        );
    }

    /// Seat model parity with the strip: driver/gunner/commander then cargo1..N.
    #[test]
    fn vehicle_attrs_seat_model_matches_the_strip() {
        let seats = super::seat_model(super::DEFAULT_CARGO_SEATS);
        let ids: Vec<&str> = seats.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "driver",
                "gunner",
                "commander",
                "cargo1",
                "cargo2",
                "cargo3",
                "cargo4"
            ]
        );
        assert_eq!(seats[0].1, "Driver");
        assert_eq!(seats[1].1, "Gunner");
        assert_eq!(seats[2].1, "Commander");
    }
}
