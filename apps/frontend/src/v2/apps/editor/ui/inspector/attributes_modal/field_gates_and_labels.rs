//! Attributes modal field gates and labels behavior.

use super::*;

/// Base styling for editable attribute controls.
pub(super) const CONTROL: &str = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";
/// Styling added when an attribute control is disabled.
pub(super) const CONTROL_LOCKED: &str = " cursor-not-allowed opacity-40";
/// Labels for the modal tabs in display order.
pub(super) const TABS: [&str; 4] = ["Transform", "Identity", "States", "Arsenal"];

/// Per-field opt-in signals for attributes that differ across a selection.
#[derive(Clone, Copy)]
pub(super) struct MultiOpts {
    pub(super) x: RwSignal<bool>,
    pub(super) y: RwSignal<bool>,
    pub(super) z: RwSignal<bool>,
    pub(super) rotation: RwSignal<bool>,
    pub(super) stance: RwSignal<bool>,
    pub(super) role: RwSignal<bool>,
    pub(super) tag: RwSignal<bool>,
    pub(super) asset_id: RwSignal<bool>,
    pub(super) description: RwSignal<bool>,
}

impl MultiOpts {
    /// Creates disabled opt-in signals for every editable field.
    pub(super) fn new() -> Self {
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

    /// Disables every multi-edit opt-in for a newly opened selection.
    pub(super) fn reset(self) {
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

/// Combines multi-edit opt-in with an unconditional write refusal.
#[derive(Clone, Copy)]
pub(super) struct Gate {
    pub(super) opt: Option<RwSignal<bool>>,
    pub(super) shut: bool,
}

impl Gate {
    /// Creates an unrestricted field gate.
    pub(super) const fn open() -> Self {
        Self {
            opt: None,
            shut: false,
        }
    }

    /// Requires opt-in only when selected values differ.
    pub(super) fn maybe(differs: bool, latch: RwSignal<bool>) -> Self {
        Self {
            opt: differs.then_some(latch),
            shut: false,
        }
    }

    /// Marks a field whose write the core rejects.
    pub(super) const fn refused(self) -> Self {
        Self { shut: true, ..self }
    }

    /// Reports whether selected values disagree for this field.
    pub(super) const fn differs(self) -> bool {
        self.opt.is_some()
    }

    /// Reads reactive state to decide whether a field is disabled.
    pub(super) fn locked(self) -> bool {
        self.shut || self.opt.is_some_and(|o| !o.get())
    }

    /// Reads untracked state for keyboard event handlers.
    pub(super) fn locked_now(self) -> bool {
        self.shut || self.opt.is_some_and(|o| !o.get_untracked())
    }
}

/// Maps coordinate labels to distinct axis colors.
pub(super) fn axis_chip_class(label: &str) -> Option<&'static str> {
    match label {
        "X" => Some("bg-red-500"),
        "Y" => Some("bg-emerald-500"),
        "Z" => Some("bg-sky-500"),
        "Rotation" => Some("bg-amber-500"),
        _ => None,
    }
}

/// Renders the label and optional multi-edit checkbox for a field.
#[cfg(target_arch = "wasm32")]
pub(super) fn field_label(label: &'static str, gate: Gate) -> impl IntoView {
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

/// Counts editable slots and names excluded vehicles in the selection.
#[must_use]
pub(crate) fn attrs_multi_subtitle(slot_n: usize, selection_n: usize) -> String {
    let base = format!("{slot_n} slots selected · multi-edit");
    if selection_n > slot_n {
        format!("{base} · vehicles excluded")
    } else {
        base
    }
}
