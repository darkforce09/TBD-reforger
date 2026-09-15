//! The dossier panel that uploads a mission document as the next stored version.
//!
//! **Role:** owns the picked file, the parsed payload and the suggested version number, opens the
//! file picker, posts the document, and renders the status line, the server's findings and the
//! preview of what the document would change.
//! **Position:** a section of the mission dossier, directly under the version rail, shown only to
//! someone the versions route would actually accept a write from.
//! **Signals & state:** `up_semver`, `up_name`, `up_size`, `up_doc`, `up_busy`, `up_status`,
//! `up_findings` and `up_preview` are page-local; `current_payload` holds the mission's stored
//! payload for the preview comparison. Reads the session store and the toast queue from context.
//! **Invariants:** the whole pipeline is browser-only — natively the closures compile but never
//! run. The document is read by reference at every step: it is the one value in this application
//! that reaches hundreds of megabytes, and each extra live copy multiplies the tab's peak.

// Every helper below is reached only from the browser-only closures, so the imports carry the
// same gate the closures do.
#[cfg(target_arch = "wasm32")]
use super::dossier_upload::{
    diff_summary_lines, next_semver, oversize_refusal, parse_uploaded_document, upload_failure,
};
#[cfg(target_arch = "wasm32")]
use super::dossier_versions::{census_line, version_census};
#[cfg(target_arch = "wasm32")]
use super::mission_diff::diff_mission_payloads;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;
use serde_json::Value;

/// The upload panel, or nothing when the viewer cannot write a version to this mission.
///
/// `initial_semver` is the suggested next version, computed by the caller before the stored
/// payload was moved out of the mission detail; `current_payload` is that payload, kept for the
/// preview comparison. `changed` re-reads the dossier and the card grid after a successful post.
pub(super) fn upload_panel(
    can_edit: bool,
    id_sv: StoredValue<String>,
    changed: Callback<()>,
    initial_semver: String,
    current_payload: Option<Value>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    // These feed only the wasm-gated closures below.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, id_sv, &changed);
    let up_semver = RwSignal::new(initial_semver);
    let current_payload = StoredValue::new(current_payload);
    // The picked file's name, which also becomes the provenance note on the stored version.
    let up_name = RwSignal::new(Option::<String>::None);
    let up_size = RwSignal::new(0usize);
    // The parsed, envelope-unwrapped editor payload, held so a rejected version number can
    // be retried without making the author pick and parse the file a second time.
    let up_doc = RwSignal::new(Option::<Value>::None);
    let up_busy = RwSignal::new(false);
    let up_status = RwSignal::new(String::new());
    // The server's findings, rendered as a persistent list rather than a toast: a schema finding
    // is a work item an author reads while editing the document, and a notice that vanishes after
    // a few seconds loses the only explanation they will ever be given.
    let up_findings = RwSignal::new(Vec::<String>::new());
    let up_preview = RwSignal::new(Vec::<String>::new());
    // The upload pipeline is browser-only — file picker, then a read, then a post — so on
    // the native build these three are read by nothing. The pure functions behind them are
    // what the test suite exercises.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (current_payload, up_name, up_size);

    // Reading and parsing happen at pick time rather than at upload time for two reasons: the
    // author is told the file is unusable before they have chosen a version number, and the
    // preview below can say what the document would do while there is still time not to.
    let pick_document = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::closure::Closure;
            use wasm_bindgen::JsCast;

            if up_busy.get_untracked() {
                return;
            }
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let Some(document) = web_sys::window().and_then(|w| w.document()) else {
                toasts.error("Could not open the file picker");
                return;
            };
            let Ok(input) = document
                .create_element("input")
                .map_err(|_| ())
                .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().map_err(|_| ()))
            else {
                toasts.error("Could not open the file picker");
                return;
            };
            input.set_type("file");
            input.set_accept("application/json,.json");

            let input_for_cb = input.clone();
            let on_change = Closure::once(move |_ev: web_sys::Event| {
                let Some(file) = input_for_cb.files().and_then(|list| list.item(0)) else {
                    return;
                };
                let name = file.name();
                // The browser reports a file size as a JavaScript number. Clamp rather than cast
                // blind: a negative or NaN size must not wrap into a huge unsigned value and sail
                // past the budget check below.
                let size = file.size().max(0.0).min(usize::MAX as f64) as usize;
                up_findings.set(Vec::new());
                up_preview.set(Vec::new());
                up_doc.set(None);
                up_name.set(Some(name.clone()));
                up_size.set(size);
                // Refuse before reading. A tab that dies part-way through a read cannot tell
                // anybody why it died.
                if let Some(refusal) = oversize_refusal(size) {
                    up_status.set(refusal);
                    return;
                }
                up_status.set(format!("Reading {name}…"));
                leptos::task::spawn_local(async move {
                    // Reading a file hands back a promise: the browser reads off disk on its own
                    // thread and this task is suspended, so the tab stays interactive through the
                    // read. The parse that follows is synchronous, which is why the budget above
                    // is a hard gate rather than a warning.
                    let text = match wasm_bindgen_futures::JsFuture::from(file.text()).await {
                        Ok(v) => v.as_string().unwrap_or_default(),
                        Err(_) => {
                            up_status.set(format!("Could not read {name}."));
                            return;
                        }
                    };
                    match parse_uploaded_document(&text) {
                        Ok(doc) => {
                            let census = census_line(&version_census(&doc));
                            let census = if census.is_empty() {
                                "no editor content".to_string()
                            } else {
                                census
                            };
                            // The payload comparison, with a real second document on the other
                            // side of it for the first time.
                            let preview = current_payload.with_value(|cur| match cur {
                                Some(a) => {
                                    let d = diff_mission_payloads(a, &doc);
                                    if d.is_empty() {
                                        vec!["Identical to the current version — uploading it \
                                             would only add a version number."
                                            .to_string()]
                                    } else {
                                        diff_summary_lines(&d)
                                    }
                                }
                                None => Vec::new(),
                            });
                            up_preview.set(preview);
                            up_doc.set(Some(doc));
                            up_status.set(format!("{name} — {census}. Ready to upload."));
                        }
                        Err(why) => up_status.set(why),
                    }
                });
            });
            let _ = input
                .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
            // The listener has to outlive this frame: the picker is fire-and-forget, and the
            // closure is dropped by the browser once it has fired.
            on_change.forget();
            input.click();
        }
    };

    // The wire shape is built by the mission compiler, the same builder the editor's own Save
    // uses, so the two doors onto the versions route cannot drift into sending different JSON.
    let upload_document = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if up_busy.get_untracked() {
                return;
            }
            if up_doc.with_untracked(Option::is_none) {
                up_status.set("Choose a mission document first.".to_string());
                return;
            }
            let semver = up_semver.get_untracked().trim().to_string();
            if semver.is_empty() {
                up_status.set("A version number is required (e.g. 1.2.3).".to_string());
                return;
            }
            let notes = up_name
                .get_untracked()
                .map(|n| format!("Uploaded from {n}"))
                .unwrap_or_else(|| "Uploaded document".to_string());
            up_busy.set(true);
            up_findings.set(Vec::new());
            up_status.set(format!("Uploading v{semver}…"));
            // Serialise the request bytes straight out of the stored document, because the copies
            // this avoids are what set the size budget.
            //
            // `with_untracked` reads the signal by reference; `get_untracked()` would clone the
            // whole parsed tree just to hand it to the builder. The builder then walks that
            // borrowed tree once, and the raw post takes the finished `String`, so the request
            // never clones a body per attempt. One live tree at the fetch instead of four.
            //
            // The buffer is pre-sized from the picked file's size because growth doubling is
            // itself a transient copy of everything written so far, and at these sizes that
            // reallocation is the peak. Compact re-serialisation is never larger than the source
            // JSON in practice, and the buffer still grows correctly if it is.
            //
            // Read the size outside the borrow below: nothing else should touch a signal while
            // the document's storage is held open.
            let cap = up_size.get_untracked().saturating_add(1024);
            let body = up_doc.with_untracked(|slot| {
                let doc = slot.as_ref()?;
                let mut buf: Vec<u8> = Vec::with_capacity(cap);
                website_mission_core::mission::compile::version_body_to_writer(
                    &mut buf, &semver, &notes, doc,
                )
                .ok()?;
                String::from_utf8(buf).ok()
            });
            // Unreachable in practice, since a parsed value always serialises and the serialiser
            // always emits UTF-8 — but sending an empty body would earn a refusal the author
            // would read as "my document is broken", so say it in our own words instead.
            let Some(body) = body else {
                up_status.set(
                    "That document could not be prepared for upload — nothing was sent."
                        .to_string(),
                );
                up_busy.set(false);
                return;
            };
            let path = format!("/missions/{}/versions", id_sv.get_value());
            let toasts = crate::v2::core::ui::toast::use_toasts();
            leptos::task::spawn_local(async move {
                // The raw post, not the typed one: a successful write echoes the entire stored
                // payload back, so a typed post would parse a whole extra tree out of the
                // response that the success arm below immediately throws away. Failure bodies are
                // still read, and their findings still folded into the message.
                match crate::v2::core::api::client::api_post_raw(store, &path, body).await {
                    Ok(()) => {
                        up_status.set(format!(
                            "Uploaded v{semver} — it is now this mission's current version."
                        ));
                        up_doc.set(None);
                        up_name.set(None);
                        up_size.set(0);
                        up_preview.set(Vec::new());
                        up_semver.set(next_semver(Some(&semver)));
                        toasts.success("Mission document uploaded");
                        // Re-read the dossier and the card grid, so the rail shows the
                        // new tip rather than the version this panel just replaced.
                        changed.run(());
                    }
                    Err((status, msg)) => {
                        let (head, rows) = upload_failure(status, msg.as_deref(), &semver);
                        up_status.set(head);
                        up_findings.set(rows);
                    }
                }
                up_busy.set(false);
            });
        }
    };

    can_edit.then(|| {
        view! {
                            <section data-testid="mission-upload-section">
                                <h3 class="mb-2 font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
                                    "Upload mission document"
                                </h3>
                                <p class="mb-3 text-label-md text-on-surface-variant">
                                    "Accepts an exported mission file or a bare editor payload. The document is validated before it is stored — if it is rejected you get the list of what is wrong with it, not just a refusal."
                                </p>
                                <div class="flex flex-wrap items-center gap-2">
                                    <button
                                        type="button"
                                        data-testid="mission-upload-pick"
                                        on:click=pick_document
                                        prop:disabled=move || up_busy.get()
                                        class="flex items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10 disabled:opacity-60"
                                    >
                                        <MaterialIcon name="upload_file" class="text-[16px]" />
                                        "Choose document…"
                                    </button>
                                    <label class="text-label-md text-on-surface-variant" for="mission-upload-semver">
                                        "Version"
                                    </label>
                                    <input
                                        id="mission-upload-semver"
                                        type="text"
                                        data-testid="mission-upload-semver"
                                        placeholder="1.2.3"
                                        prop:value=move || up_semver.get()
                                        on:input=move |ev| up_semver.set(event_target_value(&ev))
                                        class="w-28 rounded-lg border border-white/10 bg-black/30 px-3 py-2 font-mono text-label-md text-on-surface outline-none transition-colors focus:border-primary/60"
                                    />
                                    <button
                                        type="button"
                                        data-testid="mission-upload-submit"
                                        on:click=upload_document
                                        prop:disabled=move || up_busy.get() || up_doc.with(Option::is_none)
                                        class="flex items-center gap-2 rounded-lg border border-primary/30 bg-primary/15 px-4 py-2 text-label-md font-semibold text-primary transition-colors hover:bg-primary/25 disabled:opacity-40"
                                    >
                                        <MaterialIcon name="cloud_upload" class="text-[16px]" />
                                        "Upload as new version"
                                    </button>
                                    {move || {
                                        let sz = up_size.get();
                                        (sz > 0).then(|| {
                                            view! {
                                                <span
                                                    data-testid="mission-upload-size"
                                                    class="font-mono text-label-md text-on-surface-variant"
                                                >
                                                    {format!("{} / 8.4 MB", crate::editor::mission_size::format_bytes(sz))}
                                                </span>
                                            }
                                        })
                                    }}
                                </div>

                                // The headline of whatever just happened: a read, a
                                // parse refusal, a size refusal, or the upload verdict.
                                {move || {
                                    let s = up_status.get();
                                    (!s.is_empty())
                                        .then(|| {
                                            view! {
                                                <p
                                                    data-testid="mission-upload-status"
                                                    class="mt-3 text-label-md text-on-surface"
                                                >
                                                    {s}
                                                </p>
                                            }
                                        })
                                }}

                                // The server's findings — the reason the document was refused, one
                                // work item per row — in their own labelled block, because losing
                                // them is the failure this panel is built to avoid.
                                {move || {
                                    let rows = up_findings.get();
                                    (!rows.is_empty())
                                        .then(|| {
                                            view! {
                                                <div
                                                    data-testid="mission-upload-findings"
                                                    class="mt-3 rounded-xl border border-error-alert/30 bg-error-alert/10 p-4"
                                                >
                                                    <p class="font-mono text-label-sm tracking-widest text-error-alert uppercase">
                                                        "What is wrong with this document"
                                                    </p>
                                                    <ul class="mt-2 space-y-1">
                                                        {rows
                                                            .into_iter()
                                                            .map(|r| {
                                                                view! {
                                                                    <li class="font-mono text-label-sm break-words text-on-surface">
                                                                        {r}
                                                                    </li>
                                                                }
                                                            })
                                                            .collect_view()}
                                                    </ul>
                                                </div>
                                            }
                                        })
                                }}

                                // What uploading this file would change, said before it
                                // is uploaded.
                                {move || {
                                    let rows = up_preview.get();
                                    (!rows.is_empty())
                                        .then(|| {
                                            view! {
                                                <div
                                                    data-testid="mission-upload-preview"
                                                    class="mt-3 rounded-xl border border-white/10 bg-white/5 p-4"
                                                >
                                                    <p class="font-mono text-label-sm tracking-widest text-on-surface-variant uppercase">
                                                        "Against the current version"
                                                    </p>
                                                    <ul class="mt-2 space-y-1">
                                                        {rows
                                                            .into_iter()
                                                            .map(|r| {
                                                                view! {
                                                                    <li class="text-label-md text-on-surface-variant">
                                                                        {r}
                                                                    </li>
                                                                }
                                                            })
                                                            .collect_view()}
                                                    </ul>
                                                </div>
                                            }
                                        })
                                }}
                            </section>
        }
    })
}
