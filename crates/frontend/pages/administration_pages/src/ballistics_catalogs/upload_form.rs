//! The upload form: the catalog and its calibration bundle, picked and sent as one request.
//!
//! **Role:** the two file pickers, the send control, and the multipart `POST` that carries the
//! picked files under the parts [`CATALOG_PART`] and [`CALIBRATION_PART`].
//! **Position:** the left column of the `/admin/ballistics-catalogs` route; writes the outcome the
//! validation report shows, and bumps the list's reload counter when a version is stored.
//! **Signals & state:** [`UploadDesk`] holds the picked files' names and sizes, the browser file
//! handles, the in-flight flag and the latest outcome.
//! **Invariants:** a send starts only when [`upload_blocker`] finds nothing and no send is in
//! flight; picking a file clears the previous outcome, so a report never describes files other than
//! the ones picked. The send runs in the browser build only, as does the form.

#[cfg(target_arch = "wasm32")]
use super::view_model::{
    CALIBRATION_PART, CATALOG_PART, PickedFile, UploadOutcome, file_size_label, upload_blocker,
};
#[cfg(target_arch = "wasm32")]
use frontend_session::AuthStore;
#[cfg(target_arch = "wasm32")]
use frontend_ui::MaterialIcon;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The upload form's state.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub(super) struct UploadDesk {
    /// The picked catalog, as the form shows it.
    pub catalog: RwSignal<Option<PickedFile>>,
    /// The picked calibration bundle, as the form shows it.
    pub calibration: RwSignal<Option<PickedFile>>,
    /// The browser handle of the picked catalog.
    catalog_file: StoredValue<Option<web_sys::File>, LocalStorage>,
    /// The browser handle of the picked calibration bundle.
    calibration_file: StoredValue<Option<web_sys::File>, LocalStorage>,
    /// Whether a send is in flight.
    pub busy: RwSignal<bool>,
    /// The latest send's outcome; `None` before a send and after a new pick.
    pub outcome: RwSignal<Option<UploadOutcome>>,
}

#[cfg(target_arch = "wasm32")]
impl UploadDesk {
    /// An empty form: nothing picked, nothing sent.
    pub(super) fn new() -> Self {
        Self {
            catalog: RwSignal::new(None),
            calibration: RwSignal::new(None),
            catalog_file: StoredValue::new_local(None),
            calibration_file: StoredValue::new_local(None),
            busy: RwSignal::new(false),
            outcome: RwSignal::new(None),
        }
    }
}

/// Which of the two parts a picker fills.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UploadPart {
    Catalog,
    Calibration,
}

/// The form: two pickers, the reason it cannot be sent yet, and the send control.
#[cfg(target_arch = "wasm32")]
pub(super) fn upload_form(
    store: AuthStore,
    desk: UploadDesk,
    reload: RwSignal<u32>,
) -> impl IntoView {
    let blocker =
        move || upload_blocker(desk.catalog.get().as_ref(), desk.calibration.get().as_ref());
    let disabled = move || desk.busy.get() || blocker().is_some();
    let on_send = move |_| {
        if desk.busy.get_untracked()
            || upload_blocker(
                desk.catalog.get_untracked().as_ref(),
                desk.calibration.get_untracked().as_ref(),
            )
            .is_some()
        {
            return;
        }
        desk.busy.set(true);
        desk.outcome.set(None);
        send_upload(store, desk, reload);
    };
    view! {
        <section
            class="space-y-4 rounded-xl border border-white/10 bg-white/[0.02] p-5"
            data-testid="ballistics-catalog-upload"
        >
            <h2 class="text-label-md font-semibold tracking-wide text-on-surface uppercase">
                "Upload a catalog version"
            </h2>
            <p class="text-label-sm text-on-surface-variant">
                "The catalog is flown against its calibration bundle before it is stored; a version \
                 that misses any calibration case is refused with every failed case listed."
            </p>
            {file_picker(desk, UploadPart::Catalog)}
            {file_picker(desk, UploadPart::Calibration)}
            <p class="min-h-[1.25rem] text-label-sm text-on-surface-variant">
                {move || blocker().unwrap_or_default()}
            </p>
            <button
                type="button"
                class="inline-flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-label-md font-semibold text-on-primary transition-opacity disabled:cursor-not-allowed disabled:opacity-40"
                prop:disabled=disabled
                on:click=on_send
            >
                <MaterialIcon name="upload_file" class="text-[18px]" />
                {move || if desk.busy.get() { "Validating…" } else { "Validate and store" }}
            </button>
        </section>
    }
}

/// One labelled picker, with the picked file's name and size below it.
#[cfg(target_arch = "wasm32")]
fn file_picker(desk: UploadDesk, part: UploadPart) -> impl IntoView {
    let (label, part_name, picked) = match part {
        UploadPart::Catalog => ("Catalog document", CATALOG_PART, desk.catalog),
        UploadPart::Calibration => ("Calibration bundle", CALIBRATION_PART, desk.calibration),
    };
    let on_change = move |ev: leptos::ev::Event| {
        let input: web_sys::HtmlInputElement = event_target(&ev);
        let file = input.files().and_then(|list| list.item(0));
        picked.set(file.as_ref().map(|file| PickedFile {
            name: file.name(),
            size_bytes: file.size(),
        }));
        match part {
            UploadPart::Catalog => desk.catalog_file.set_value(file),
            UploadPart::Calibration => desk.calibration_file.set_value(file),
        }
        desk.outcome.set(None);
    };
    view! {
        <label class="block space-y-1">
            <span class="font-mono text-code-md tracking-wider text-on-surface-variant/70 uppercase">
                {label}
            </span>
            <input
                type="file"
                accept=".json,application/json"
                name=part_name
                class="block w-full text-label-md text-on-surface-variant file:mr-3 file:rounded-md file:border file:border-outline-variant/40 file:bg-surface-container file:px-3 file:py-1.5 file:text-label-md file:text-on-surface"
                on:change=on_change
            />
            <span class="block truncate font-mono text-code-md text-outline">
                {move || {
                    picked
                        .get()
                        .map(|file| format!("{} · {}", file.name, file_size_label(file.size_bytes)))
                        .unwrap_or_else(|| "Nothing picked".into())
                }}
            </span>
        </label>
    }
}

/// Send both picked files as one multipart `POST`, record the outcome, and reload the list when
/// a version is stored.
#[cfg(target_arch = "wasm32")]
fn send_upload(store: AuthStore, desk: UploadDesk, reload: RwSignal<u32>) {
    {
        use super::view_model::{BALLISTICS_CATALOGS_PATH, upload_outcome};
        use frontend_api_dtos::ballistics_catalogs::CatalogUploadReport;
        use frontend_transport::client::api_post_form_keeping_refusal;

        let (Some(catalog), Some(calibration)) = (
            desk.catalog_file.get_value(),
            desk.calibration_file.get_value(),
        ) else {
            desk.busy.set(false);
            return;
        };
        leptos::task::spawn_local(async move {
            let outcome = match upload_form_data(&catalog, &calibration) {
                Some(form) => upload_outcome(
                    api_post_form_keeping_refusal::<CatalogUploadReport>(
                        store,
                        BALLISTICS_CATALOGS_PATH,
                        form,
                    )
                    .await,
                ),
                None => UploadOutcome::Unreachable,
            };
            if matches!(outcome, UploadOutcome::Accepted(_)) {
                let _ = reload.try_update(|generation| *generation += 1);
            }
            let _ = desk.outcome.try_set(Some(outcome));
            let _ = desk.busy.try_set(false);
        });
    }
}

/// The multipart body: the catalog under [`CATALOG_PART`], the bundle under [`CALIBRATION_PART`],
/// each with its file name. `None` when the browser cannot build it.
#[cfg(target_arch = "wasm32")]
fn upload_form_data(
    catalog: &web_sys::File,
    calibration: &web_sys::File,
) -> Option<web_sys::FormData> {
    let form = web_sys::FormData::new().ok()?;
    form.append_with_blob_and_filename(CATALOG_PART, catalog.as_ref(), &catalog.name())
        .ok()?;
    form.append_with_blob_and_filename(CALIBRATION_PART, calibration.as_ref(), &calibration.name())
        .ok()?;
    Some(form)
}
