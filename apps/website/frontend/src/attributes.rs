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
}

/// T-649 — the field label row: the field name plus, when the selection disagrees on this field,
/// the checkbox that opts it into the multi-apply.
///
/// This is a `<span>` rather than the field's old wrapping `<label>` because a `<label>` holding
/// two inputs implicitly labels only the first — the checkbox would have stolen the click that
/// should focus the field. The field input carries an explicit `aria-label` instead, and the
/// checkbox gets its own `<label>`.
#[cfg(target_arch = "wasm32")]
fn field_label(label: &'static str, gate: Gate) -> impl IntoView {
    view! {
        <span class="flex items-center justify-between gap-2 text-label-sm uppercase tracking-wider text-outline">
            <span>{label}</span>
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
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if attrs_open.get_untracked().is_some() && ev.key() == "Escape" {
                crate::editor_ops::close_attributes();
            }
        });
        on_cleanup(move || esc.remove());
    }
    // T-649 ATTR-MULTI-CHK-001 — the per-field opt-in latches, minted ONCE on the component (see
    // `MultiOpts`). The effect re-arms them whenever the modal's target changes — it tracks
    // `attrs_open` and deliberately NOT `doc_tick`, so a commit (which bumps `doc_tick`) leaves the
    // operator's ticks alone while a fresh open starts from a clean slate.
    let opts = MultiOpts::new();
    Effect::new(move |_| {
        let _ = attrs_open.get();
        opts.reset();
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
                    let diff = crate::editor_ops::read_attrs_diff(&multi);
                    Some(modal_view(
                        attrs,
                        multi,
                        diff,
                        opts,
                        registry_items,
                        compat,
                        attrs_tab,
                    ))
                }
                None => {
                    // Slot undone away while open → close (React's `slot &&` render guard).
                    crate::editor_ops::close_attributes();
                    None
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
#[cfg(target_arch = "wasm32")]
fn modal_view(
    attrs: crate::editor_ops::SlotAttrs,
    multi: Vec<String>,
    diff: crate::editor_ops::AttrDiff,
    opts: MultiOpts,
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
            // Never show one slot's role as the heading for N slots — say how many are being edited.
            format!("{multi_n} entities selected · multi-edit")
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
                </div>
                <button
                    type="button"
                    aria-label="Close"
                    on:click=move |_| crate::editor_ops::close_attributes()
                    class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                >
                    <crate::ui::MaterialIcon name="close" />
                </button>
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
                                    " to overwrite that field on every selected entity."
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
                        1 => identity_tab(targets, attrs, is_multi, diff, opts).into_any(),
                        2 => states_tab().into_any(),
                        _ => {
                            let loadout = crate::editor_ops::read_loadout(&slot_id.get_value());
                            view! {
                                // T-649 — HONESTY BANNER. Inverting the `open_arsenal` guard is what
                                // stops the context menu's "Edit Loadout..." row being enabled-but-
                                // inert on a multi-selection: the modal opens now. But the Arsenal
                                // BODY below is `arsenal.rs`'s and still edits ONE slot, so say so
                                // rather than let the "N entities selected" header imply otherwise.
                                {is_multi
                                    .then(|| {
                                        view! {
                                            <p class="mb-3 rounded-md border border-outline-variant/40 bg-surface-container-lowest/50 px-3 py-2 text-label-sm normal-case text-on-surface-variant">
                                                "Loadout edits apply to this one entity ("
                                                {slot_id.get_value()}
                                                "), not to the whole selection."
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

/* ─────────────────────────── field primitives (fields.tsx ports) ─────────────────────────── */

/// Mono numeric field committing on blur/Enter (one commit = one undo step). While focused it holds
/// the local draft; unfocused it mirrors the doc value (rounded), so a map drag updates it live.
///
/// T-649 — `gate` is the multi-edit gate. When it reports `differs()` there is no single truthful
/// value to display, so the field shows EMPTY (placeholder `—`) rather than one arbitrary member's
/// number, and stays `disabled` until the "Apply to all" checkbox is ticked. The commit path is
/// untouched: whatever the operator types is parsed and handed to `on_commit` exactly as before.
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
    let rounded = StoredValue::new(format!("{}", value.round()));
    // A differing field starts from an EMPTY draft — pre-filling one member's value would make an
    // accidental blur write that member's number onto the whole selection.
    let seed = move || {
        if gate.differs() {
            String::new()
        } else {
            rounded.get_value()
        }
    };
    let commit = move || {
        focused.set(false);
        if let Ok(n) = draft.get_untracked().parse::<f64>() {
            if n.is_finite() {
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
                    aria-label=label
                    disabled=move || gate.locked()
                    placeholder=if gate.differs() { "—" } else { "" }
                    prop:value=move || { if focused.get() { draft.get() } else { seed() } }
                    on:focus=move |_| {
                        draft.set(seed());
                        focused.set(true);
                    }
                    on:input=move |ev| draft.set(event_target_value(&ev))
                    on:blur=move |_| commit()
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            if let Some(t) = ev
                                .target()
                                .and_then(|t| {
                                    wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(t).ok()
                                })
                            {
                                t.blur().ok();
                            }
                        }
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

/// Text field committing per input event — the React `TextField` semantics (one undo step per
/// keystroke is the oracle behavior).
///
/// T-649 — same `gate` contract as [`number_field`]: a field the selection disagrees on renders
/// empty with a "Multiple values" placeholder and is disabled until its checkbox is ticked.
#[cfg(target_arch = "wasm32")]
fn text_field(
    label: &'static str,
    value: String,
    placeholder: &'static str,
    gate: Gate,
    on_change: impl Fn(String) + Copy + 'static,
) -> impl IntoView {
    let shown = if gate.differs() { String::new() } else { value };
    let ph = if gate.differs() {
        "Multiple values"
    } else {
        placeholder
    };
    view! {
        <div class="flex flex-col gap-1">
            {field_label(label, gate)}
            <input
                type="text"
                aria-label=label
                disabled=move || gate.locked()
                value=shown
                placeholder=ph
                on:input=move |ev| on_change(event_target_value(&ev))
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
                {number_field(
                    "X",
                    a.x,
                    None,
                    g(diff.x, opts.x),
                    move |x| commit_position(targets, Some(x), None, None, None),
                )}
                {number_field(
                    "Y",
                    a.y,
                    None,
                    g(diff.y, opts.y),
                    move |y| commit_position(targets, None, Some(y), None, None),
                )}
                {number_field(
                    "Z",
                    a.z,
                    None,
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
                "Drag on the map or edit coordinates above. Z is manual until terrain elevation (DEM) ships."
            </p>
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
            // T-082 ATTR-FIELD-OBJ-TYPE. The entity TYPE — the slot's `assetId`, the prefab it
            // spawns as. It was authored on palette drop and mutable in the core all along; what
            // was missing was the READ (`read_attrs` built its snapshot from the SoA, which has no
            // such column), so the modal had nothing to show and therefore nothing to edit.
            //
            // Free text rather than a select, deliberately: the asset vocabulary is the registry's
            // (T-146 Asset Browser Data Wiring is what puts a real catalogue behind a picker), and a
            // hardcoded option list here would be a SHORTER vocabulary than the one the doc already
            // accepts — it would make types the editor can currently author unreachable. Empty
            // clears it, and a slot with no `assetId` compiles to its faction's default kit alias.
            {text_field(
                "Type",
                a.asset_id.clone(),
                "Asset id — empty uses the faction default",
                g(diff.asset_id, opts.asset_id),
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
}
