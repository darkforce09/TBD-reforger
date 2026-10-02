//! Conflict dialog for editor overlays.
use super::*;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::shell::hydrate as mission_hydrate;

/// Local and server revision details shown during a save conflict.
#[derive(Clone)]
pub struct ConflictInfo {
    pub payload_json: String,
    pub semver: Option<String>,
    pub local_objects: usize,
    pub server_objects: usize,
    pub local_saved: String,
    pub server_saved: String,
}

/// Offers local or server revision resolution for a conflict.
#[component]
pub(crate) fn ConflictDialog(
    conflict: RwSignal<Option<ConflictInfo>>,
    conflict_id: String,
) -> impl IntoView {
    let id = StoredValue::new(conflict_id);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = id;
    move || {
        conflict.get().map(|c| {
            let _ = &c;
            #[cfg(target_arch = "wasm32")]
            let (id_server, id_local) = (id.get_value(), id.get_value());
            let semver_label = c
                .semver
                .clone()
                .map(|s| format!("Saved version v{s}"))
                .unwrap_or_else(|| "A saved version".to_string());
            view! {
                <div class="fixed inset-0 z-[60] bg-black/50 backdrop-blur-sm"></div>
                <div class="glass fixed top-1/2 left-1/2 z-[60] flex w-[92vw] max-w-md -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none">
                    <div class="border-b border-outline-variant/30 px-6 py-4">
                        <h2 class="text-headline-sm text-on-surface">"Unsaved local changes"</h2>
                        <p class="mt-1 text-label-md text-on-surface-variant">
                            {semver_label}
                            " on the server differs from your local copy. Which version should win?"
                        </p>
                        <dl class="mt-3 grid grid-cols-2 gap-x-4 gap-y-1 text-label-sm">
                            <dt class="font-medium text-on-surface">"Your local copy"</dt>
                            <dt class="font-medium text-error">"Server version"</dt>
                            <dd class="text-on-surface-variant">{format!("{} objects · written {}", c.local_objects, c.local_saved)}</dd>
                            <dd class="text-on-surface-variant">{format!("{} objects · saved {}", c.server_objects, c.server_saved)}</dd>
                        </dl>
                        <p class="mt-3 text-label-sm text-error">
                            "Loading the server version will discard your local copy. One Ctrl/Cmd+Z puts it back."
                        </p>
                    </div>
                    <div class="flex justify-end gap-2 px-6 py-4">
                        <button
                            type="button"
                            aria-label="Keep local copy"
                            class="rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10"
                            on:click=move |_| {
                                #[cfg(target_arch = "wasm32")]
                                mission_hydrate::resolve_conflict_local(
                                    id_local.clone(),
                                    conflict,
                                );
                            }
                        >
                            "Keep local copy"
                        </button>
                        <button
                            type="button"
                            aria-label="Load server version — discards your local copy"
                            class="rounded-lg bg-error/15 px-4 py-2 text-label-md font-medium text-error transition-colors hover:bg-error/25"
                            on:click=move |_| {
                                #[cfg(target_arch = "wasm32")]
                                mission_hydrate::resolve_conflict_server(
                                    id_server.clone(),
                                    conflict,
                                );
                            }
                        >
                            "Load server version"
                        </button>
                    </div>
                </div>
            }
        })
    }
}
