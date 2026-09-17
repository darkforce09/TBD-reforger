//! Attributes modal identity tab behavior.

use super::*;

/// Renders role, tag, type, description, and squad controls.
#[cfg(target_arch = "wasm32")]
pub(super) fn identity_tab(
    targets: StoredValue<Vec<String>>,
    attrs: StoredValue<engine_ops::SlotAttrs>,
    is_multi: bool,
    diff: engine_ops::AttrDiff,
    opts: MultiOpts,
    registry_items: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
) -> impl IntoView {
    let a = attrs.get_value();
    let g = |differs: bool, latch| Gate::maybe(is_multi && differs, latch);
    let _ = is_multi;
    view! {
        <div class="flex flex-col gap-4">
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
