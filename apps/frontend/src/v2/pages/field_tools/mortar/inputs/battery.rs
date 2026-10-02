//! The battery: one or more labelled guns, each a position on the map.
//!
//! **Role:** the gun drafts, adding and removing guns, resolving every gun into the save body's
//! [`FireMissionGunPosition`], and the rows that edit them.
//! **Position:** the gun half of the inputs card; each gun reuses the position draft and its
//! fields from the sibling `positions` module, and the solve bridge solves every resolved gun
//! independently onto the one target.
//! **Signals & state:** the view reads and writes the page's gun list signal; each draft carries
//! a stable key the rows are keyed by.
//! **Invariants:** a battery has between one and [`MAX_GUNS`] guns; keys are unique within the list
//! (a new gun takes one above the highest); a new gun gets the lowest free "Gun n" label and inherits the last
//! gun's height source; labels are non-empty and unique once trimmed.

use super::positions::{
    position_error_message, position_fields, resolve_position, MortarTerrain, PositionDraft,
    PositionError,
};
use super::INPUT_CLASS;
use leptos::prelude::*;
use map_engine::data::scenario::ballistics::fire_mission::FireMissionGunPosition;

/// Most guns one battery may hold: the contract's `FireMissionSave.guns` `maxItems`, which the
/// save route enforces too.
pub(crate) const MAX_GUNS: usize = 12;

/// One gun as typed.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GunDraft {
    /// Stable identity of the row.
    pub(crate) key: u32,
    /// Label shown and saved, e.g. "Gun 1".
    pub(crate) label: String,
    /// Where the gun stands.
    pub(crate) position: PositionDraft,
}

/// A battery of one gun, "Gun 1".
pub(crate) fn default_battery() -> Vec<GunDraft> {
    vec![GunDraft {
        key: 0,
        label: "Gun 1".to_string(),
        position: PositionDraft::default(),
    }]
}

/// The lowest "Gun n" label no gun of `guns` carries.
pub(crate) fn next_gun_label(guns: &[GunDraft]) -> String {
    (1..)
        .map(|n| format!("Gun {n}"))
        .find(|label| guns.iter().all(|g| g.label.trim() != label))
        .unwrap_or_default()
}

/// Appends a gun; `false` when the battery is full.
pub(crate) fn add_gun(guns: &mut Vec<GunDraft>) -> bool {
    if guns.len() >= MAX_GUNS {
        return false;
    }
    let key = guns.iter().map(|g| g.key + 1).max().unwrap_or(0);
    let position = PositionDraft {
        height_choice: guns
            .last()
            .map(|g| g.position.height_choice)
            .unwrap_or(PositionDraft::default().height_choice),
        ..PositionDraft::default()
    };
    guns.push(GunDraft {
        key,
        label: next_gun_label(guns),
        position,
    });
    true
}

/// Removes the gun keyed `key`; `false` when it is the last gun or no gun has that key.
pub(crate) fn remove_gun(guns: &mut Vec<GunDraft>, key: u32) -> bool {
    if guns.len() <= 1 {
        return false;
    }
    let before = guns.len();
    guns.retain(|g| g.key != key);
    guns.len() != before
}

/// Why a battery does not resolve.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BatteryError {
    /// A gun's label is empty.
    EmptyLabel {
        /// Zero-based index of the gun.
        index: usize,
    },
    /// Two guns carry the same label.
    DuplicateLabel(String),
    /// A gun's position does not resolve.
    Position {
        /// The gun's label.
        label: String,
        /// Why.
        error: PositionError,
    },
}

/// The sentence the page shows for a battery error.
pub(crate) fn battery_error_message(error: &BatteryError) -> String {
    match error {
        BatteryError::EmptyLabel { index } => format!("Gun {} needs a label.", index + 1),
        BatteryError::DuplicateLabel(label) => format!("Two guns are labelled \"{label}\"."),
        BatteryError::Position { label, error } => {
            format!("{label}: {}", position_error_message(error))
        }
    }
}

/// Resolves every gun on `terrain`, in battery order.
///
/// # Errors
///
/// Every [`BatteryError`] found, in battery order; the list is never empty on `Err`.
pub(crate) fn resolve_battery(
    guns: &[GunDraft],
    terrain: MortarTerrain,
    height_at: impl Fn(f64, f64) -> Option<f64>,
) -> Result<Vec<FireMissionGunPosition>, Vec<BatteryError>> {
    let mut errors = Vec::new();
    let mut resolved = Vec::with_capacity(guns.len());
    for (index, gun) in guns.iter().enumerate() {
        let label = gun.label.trim().to_string();
        if label.is_empty() {
            errors.push(BatteryError::EmptyLabel { index });
        } else if guns[..index].iter().any(|g| g.label.trim() == label) {
            errors.push(BatteryError::DuplicateLabel(label.clone()));
        }
        match resolve_position(&gun.position, terrain, &height_at) {
            Ok(point) => resolved.push(FireMissionGunPosition {
                label,
                x: point.x,
                y: point.y,
                height_m: point.height_m,
                height_source: point.height_source,
            }),
            Err(error) => errors.push(BatteryError::Position {
                label: if label.is_empty() {
                    format!("Gun {}", index + 1)
                } else {
                    label
                },
                error,
            }),
        }
    }
    if errors.is_empty() {
        Ok(resolved)
    } else {
        Err(errors)
    }
}

/// The gun rows with their add and remove buttons.
pub(crate) fn battery_inputs(
    guns: RwSignal<Vec<GunDraft>>,
    terrain: RwSignal<MortarTerrain>,
) -> impl IntoView {
    let update = move |key: u32, edit: &dyn Fn(&mut GunDraft)| {
        guns.update(|all| {
            if let Some(gun) = all.iter_mut().find(|g| g.key == key) {
                edit(gun);
            }
        });
    };
    view! {
        <div class="flex flex-col gap-3" data-mortar-input="battery">
            <For
                each=move || guns.get()
                key=|gun| gun.key
                children=move |gun| {
                    let key = gun.key;
                    let draft = Signal::derive(move || {
                        guns.with(|all| {
                            all.iter()
                                .find(|g| g.key == key)
                                .map(|g| g.position.clone())
                                .unwrap_or_default()
                        })
                    });
                    view! {
                        <div class="flex flex-col gap-2 rounded-lg border border-border-subtle p-3">
                            <div class="flex items-end gap-2">
                                <label class="flex-1 text-sm">
                                    "Gun label"
                                    <input
                                        type="text"
                                        prop:value=move || {
                                            guns.with(|all| {
                                                all.iter()
                                                    .find(|g| g.key == key)
                                                    .map(|g| g.label.clone())
                                                    .unwrap_or_default()
                                            })
                                        }
                                        on:input=move |ev| {
                                            let text = event_target_value(&ev);
                                            update(key, &|g| g.label = text.clone());
                                        }
                                        class=INPUT_CLASS
                                    />
                                </label>
                                <button
                                    type="button"
                                    class="rounded-lg border border-border-subtle px-3 py-2 text-sm disabled:opacity-50"
                                    prop:disabled=move || guns.with(|all| all.len() <= 1)
                                    on:click=move |_| {
                                        guns.update(|all| {
                                            remove_gun(all, key);
                                        })
                                    }
                                >
                                    "Remove"
                                </button>
                            </div>
                            {position_fields(
                                "Gun",
                                draft,
                                move |next| update(key, &|g| g.position = next.clone()),
                                terrain,
                            )}
                        </div>
                    }
                }
            />
            <button
                type="button"
                class="self-start rounded-lg border border-border-subtle px-3 py-2 text-sm disabled:opacity-50"
                prop:disabled=move || guns.with(|all| all.len() >= MAX_GUNS)
                on:click=move |_| {
                    guns.update(|all| {
                        add_gun(all);
                    })
                }
            >
                "Add gun"
            </button>
        </div>
    }
}
