//! Mission Saving browser commands.
use super::*;

/// Save a new immutable version (React `saveVersion`): compile with `orbat` omitted (the server
/// re-derives), POST `{semver, editor_notes, payload}` to `/missions/:id/versions`, and reflect the
/// outcome in `status`. 409 = dup semver, 413 = too large, 401 = not signed in.
///
/// a 400 from `create_version` carries the *list* of things wrong with the payload
/// (schema violations plus wire-safety findings). `findings` takes per-problem lines so the dialog can name them; the
/// headline stays short because `status` is also rendered in the top strip.
pub fn save_now(
    semver: String,
    notes: String,
    status: RwSignal<String>,
    findings: RwSignal<Vec<String>>,
) {
    findings.set(Vec::new());
    let Some(snap) = snapshot() else {
        status.set("Editor not ready".to_string());
        return;
    };
    // ══════  REFUSE a document with duplicate slot ids ══════
    //
    // The shared duplicate-slot operation checks the live document before compiling or posting.
    //
    // It sits BEFORE `compile_payload` deliberately: compiling and POSTing a document we
    // already know the server will reject spends a round trip to learn what is knowable here,
    // and the server's 400 does not name the squad.
    //
    // DIVERGENCE, DELIBERATE AND UNRESOLVED (see the slice report): the UPLOAD path has a
    // private near-twin, `check_duplicate_slot_ids_in_payload` in `library/mission_library.rs`
    // (defined :1519, called :1565), which reads `payload.editor.squads[]` JSON. The two do
    // NOT agree  the engine's `data::store::operations::slot_ids` gates each id on
    // `doc.slot_exists(id)` and the library version does not, so a payload carrying a
    // DANGLING duplicate id is refused on upload and passes here. Collapsing them onto this
    // function is the right repair; `mission_library.rs` is outside 's owns, so the
    // divergence is recorded rather than silently halved. Do not "fix" one side alone  that
    // would make them disagree in a NEW way without anything failing.
    let dups = live_duplicate_slot_ids();
    if !dups.is_empty() {
        let (head, rows) = super::super::duplicate_slot_id_report(&dups);
        status.set(head);
        findings.set(rows);
        return;
    }
    let payload = compile_payload(&snap.small, &snap.slots, false);
    let body = version_body(&semver, &notes, &payload);
    let auth = snap.auth;
    let path = format!("/missions/{}/versions", snap.mission_id);
    let mission_id = snap.mission_id.clone();
    status.set(format!("Saving v{semver}…"));
    spawn_local(async move {
        match crate::v2::core::api::client::api_post::<serde_json::Value>(auth, &path, body).await {
            Ok(_) => {
                status.set(format!("Saved v{semver}"));
                // the saved version is now what local derives from: clear the dirty
                // flag and update the current-semver signal so a later Export/adopt uses it.
                // ( removed the `editor_session::mark_adopted` call that sat here:
                // had already emptied it, and  replaced the semver marker it once wrote
                // with the content test in `mission_hydrate::classify_local`.)
                crate::v2::apps::editor::bridge::document_host::history::set_dirty(false);
                //  fix  expire the conflict backup pair. This 201 is the one moment those
                // whole-document IDB records stop being anybody's last copy, and nothing else ever
                // deleted them: they accumulated one doc per mission ever conflicted, forever, while
                // `__missionBackup.has()` kept offering a weeks-old document that a restore would
                // swap over current work. Rationale in `mission_hydrate::clear_local_backups`.
                crate::v2::apps::editor::shell::hydrate::clear_local_backups(&mission_id);
                if let Some(sig) = semver_signal() {
                    sig.set(Some(semver.clone()));
                }
            }
            Err((409, _)) => status.set(format!("Version {semver} already exists")),
            Err((413, _)) => status.set("Payload too large".to_string()),
            Err((401, _)) => status.set("Sign in to save".to_string()),
            Err((s, msg)) => {
                let (head, rows) = crate::v2::core::api::client::split_error_lines(msg.as_deref());
                let head = head.filter(|h| !h.is_empty());
                status.set(match (&head, rows.len()) {
                    (Some(h), 0) => format!("Save rejected ({s}): {h}"),
                    (Some(h), n) => format!("Save rejected ({s}): {h} — {n} problem(s) below"),
                    (None, _) => format!("Save failed ({s})"),
                });
                findings.set(rows);
            }
        }
    });
}

/// Current wall-clock ISO-8601 (`new Date().toISOString()`)  the one clock read, kept out of the
/// pure core (which takes `exported_at` as a param, so the smoke can pin it).
pub(super) fn js_date_iso() -> String {
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default()
}

/// The `Blob → URL.createObjectURL → <a download> → click → revokeObjectURL` download dance
/// (mirrors the React `exportJson` DOM path).
pub(crate) fn download_json(filename: &str, contents: &str) -> Result<(), JsValue> {
    let win = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = win
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(contents));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("application/json");
    let blob = web_sys::Blob::new_with_str_sequence_and_options(parts.as_ref(), &opts)?;

    let url = web_sys::Url::create_object_url_with_blob(&blob)?;

    let anchor = document
        .create_element("a")?
        .dyn_into::<web_sys::HtmlAnchorElement>()?;
    anchor.set_href(&url);
    anchor.set_download(filename);
    let el: &web_sys::HtmlElement = anchor.as_ref();
    el.click();

    web_sys::Url::revoke_object_url(&url)?;
    Ok(())
}

/// one row in the "Merge Mission…" picker: a mission the author can merge
/// FROM. `id` feeds [`merge_mission_now`]; `title` is the label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionPick {
    /// The mission id (`GET /missions/:id`).
    pub id: String,
    /// The mission's display title.
    pub title: String,
}
