//! The weapon, shell and charge the battery fires, chosen from the loaded catalog.
//!
//! **Role:** lists the catalog's weapons, the shells the chosen weapon fires, and every charge
//! of the chosen shell; keeps the choice valid when the catalog or the weapon changes; renders
//! the three pickers.
//! **Position:** the head of the inputs card; the solve bridge reads `ArmamentSelection` and the
//! illumination field reads the chosen shell's time fuze. The catalog is `ballistics_model`'s
//! [`BallisticsCatalog`] as the catalog source decoded it.
//! **Signals & state:** the view reads the page's catalog signal and reads and writes its
//! selection signal; the pure functions hold nothing.
//! **Invariants:** only shells that the weapon lists and the catalog defines are offered; a
//! selection names a weapon of the catalog, a shell that weapon fires, and either the
//! recommended charge or a charge that shell has; the weapon's mils convention is shown with it.
//!
//! [`BallisticsCatalog`]: ballistics_model::catalog::BallisticsCatalog

#[cfg(target_arch = "wasm32")]
use super::INPUT_CLASS;
#[cfg(any(target_arch = "wasm32", test))]
use ballistics_model::catalog::{BallisticsCatalog, Shell, ShellRole};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::sync::Arc;

/// Which charge the battery lays.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum ChargeChoice {
    /// The fewest rings that solve, per gun.
    #[default]
    Recommended,
    /// This many rings for every gun.
    Rings(u32),
}

/// The weapon, shell and charge chosen.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ArmamentSelection {
    /// Weapon identifier in the catalog; empty before a catalog loads.
    pub(crate) weapon_id: String,
    /// Shell identifier in the catalog; empty before a catalog loads.
    pub(crate) shell_id: String,
    /// The charge to lay.
    pub(crate) charge: ChargeChoice,
}

/// One offered option: its value and its label.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PickerOption {
    /// The `<option>` value.
    pub(crate) value: String,
    /// The text shown.
    pub(crate) label: String,
}

/// Every weapon of the catalog, labelled with its mils convention.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn weapon_options(catalog: &BallisticsCatalog) -> Vec<PickerOption> {
    catalog
        .weapons
        .iter()
        .map(|w| PickerOption {
            value: w.weapon_id.to_string(),
            label: format!("{} ({} mils)", w.display_name, w.mils_per_circle),
        })
        .collect()
}

/// The shells `weapon_id` fires that the catalog defines, in the weapon's order.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn shell_options(catalog: &BallisticsCatalog, weapon_id: &str) -> Vec<PickerOption> {
    let Some(weapon) = catalog.weapons.iter().find(|w| w.weapon_id == weapon_id) else {
        return Vec::new();
    };
    weapon
        .shell_ids
        .iter()
        .filter_map(|id| catalog.shells.iter().find(|s| &s.shell_id == id))
        .map(|s| PickerOption {
            value: s.shell_id.to_string(),
            label: format!("{} — {}", s.display_name, role_label(s.role)),
        })
        .collect()
}

/// The recommended choice followed by every charge of `shell_id`, in catalog order.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn charge_options(catalog: &BallisticsCatalog, shell_id: &str) -> Vec<PickerOption> {
    let mut options = vec![PickerOption {
        value: charge_value(ChargeChoice::Recommended),
        label: "Recommended (fewest rings that solve)".to_string(),
    }];
    if let Some(shell) = find_shell(catalog, shell_id) {
        options.extend(shell.charges.iter().map(|c| PickerOption {
            value: charge_value(ChargeChoice::Rings(c.rings)),
            label: if c.is_default {
                format!("Charge {} (game default)", c.rings)
            } else {
                format!("Charge {}", c.rings)
            },
        }));
    }
    options
}

/// The `<option>` value of a charge choice.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn charge_value(choice: ChargeChoice) -> String {
    match choice {
        ChargeChoice::Recommended => "recommended".to_string(),
        ChargeChoice::Rings(rings) => rings.to_string(),
    }
}

/// The charge choice an `<option>` value names; anything unreadable is the recommendation.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn charge_from_value(value: &str) -> ChargeChoice {
    value
        .parse::<u32>()
        .map(ChargeChoice::Rings)
        .unwrap_or(ChargeChoice::Recommended)
}

/// The shell of the catalog named `shell_id`.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn find_shell<'c>(catalog: &'c BallisticsCatalog, shell_id: &str) -> Option<&'c Shell> {
    catalog.shells.iter().find(|s| s.shell_id == shell_id)
}

/// `current` made valid for `catalog`: an unknown weapon falls back to the first weapon, a shell
/// the weapon does not fire to its first shell, and a charge the shell lacks to the
/// recommendation. An empty catalog yields the empty selection.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn reconcile_selection(
    catalog: &BallisticsCatalog,
    current: &ArmamentSelection,
) -> ArmamentSelection {
    let weapon_id = if catalog
        .weapons
        .iter()
        .any(|w| w.weapon_id == *current.weapon_id)
    {
        current.weapon_id.clone()
    } else {
        catalog
            .weapons
            .first()
            .map(|w| w.weapon_id.to_string())
            .unwrap_or_default()
    };
    let shells = shell_options(catalog, &weapon_id);
    let shell_id = if shells.iter().any(|s| s.value == current.shell_id) {
        current.shell_id.clone()
    } else {
        shells.first().map(|s| s.value.clone()).unwrap_or_default()
    };
    let charge = match current.charge {
        ChargeChoice::Rings(rings)
            if find_shell(catalog, &shell_id).is_some_and(|s| s.charge(rings).is_some()) =>
        {
            ChargeChoice::Rings(rings)
        }
        _ => ChargeChoice::Recommended,
    };
    ArmamentSelection {
        weapon_id,
        shell_id,
        charge,
    }
}

/// Word for a shell role.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn role_label(role: ShellRole) -> &'static str {
    match role {
        ShellRole::He => "high explosive",
        ShellRole::Smoke => "smoke",
        ShellRole::Illumination => "illumination",
        ShellRole::Practice => "practice",
        ShellRole::Other => "other",
    }
}

/// One labelled picker over `options`, bound to `value`/`on_pick`.
#[cfg(target_arch = "wasm32")]
fn picker(
    label: &'static str,
    input_name: &'static str,
    options: impl Fn() -> Vec<PickerOption> + Send + Sync + 'static,
    value: impl Fn() -> String + Send + Sync + 'static,
    on_pick: impl Fn(String) + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <label class="text-sm">
            {label}
            <select
                prop:value=value
                on:change=move |ev| on_pick(event_target_value(&ev))
                class=INPUT_CLASS
                data-mortar-input=input_name
            >
                {move || {
                    options()
                        .into_iter()
                        .map(|o| view! { <option value=o.value>{o.label}</option> })
                        .collect_view()
                }}
            </select>
        </label>
    }
}

/// The weapon, shell and charge pickers. Each pick is reconciled against the catalog, so a new
/// weapon lands on a shell it fires and a new shell on a charge it has.
#[cfg(target_arch = "wasm32")]
pub(crate) fn weapon_and_shell_inputs(
    catalog: RwSignal<Option<Arc<BallisticsCatalog>>>,
    selection: RwSignal<ArmamentSelection>,
) -> impl IntoView {
    let pick = move |edit: &dyn Fn(&mut ArmamentSelection)| {
        let Some(catalog) = catalog.get_untracked() else {
            return;
        };
        let mut next = selection.get_untracked();
        edit(&mut next);
        selection.set(reconcile_selection(&catalog, &next));
    };
    let with_catalog = move |f: fn(&BallisticsCatalog, &ArmamentSelection) -> Vec<PickerOption>| {
        catalog
            .get()
            .map(|c| selection.with(|s| f(&c, s)))
            .unwrap_or_default()
    };
    view! {
        {picker(
            "Weapon",
            "weapon",
            move || with_catalog(|c, _| weapon_options(c)),
            move || selection.with(|s| s.weapon_id.clone()),
            move |v| pick(&|s| s.weapon_id = v.clone()),
        )}
        {picker(
            "Shell",
            "shell",
            move || with_catalog(|c, s| shell_options(c, &s.weapon_id)),
            move || selection.with(|s| s.shell_id.clone()),
            move |v| pick(&|s| s.shell_id = v.clone()),
        )}
        {picker(
            "Charge",
            "charge",
            move || with_catalog(|c, s| charge_options(c, &s.shell_id)),
            move || selection.with(|s| charge_value(s.charge)),
            move |v| pick(&|s| s.charge = charge_from_value(&v)),
        )}
    }
}
