//! Attributes modal spatial transform tab behavior.

use super::*;

/// Renders position, rotation, and stance controls.
#[cfg(target_arch = "wasm32")]
pub(super) fn transform_tab(
    targets: StoredValue<Vec<String>>,
    attrs: StoredValue<engine_ops::SlotAttrs>,
    is_multi: bool,
    diff: engine_ops::AttrDiff,
    opts: MultiOpts,
    locked_n: usize,
) -> impl IntoView {
    let a = attrs.get_value();
    let n = targets.get_value().len();
    let all_locked = n > 0 && locked_n == n;
    let g = move |differs: bool, latch| {
        let base = Gate::maybe(is_multi && differs, latch);
        if all_locked {
            base.refused()
        } else {
            base
        }
    };
    let stance_gate = Gate::maybe(is_multi && diff.stance, opts.stance);
    view! {
        <div class="flex flex-col gap-4">
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
                "Drag on the map or edit coordinates above. Z is sampled from terrain elevation (DEM); edit it here to override."
            </p>
        </div>
    }
}
