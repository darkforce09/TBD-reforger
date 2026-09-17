//! Fetches server mission state and resolves local conflicts.
use super::*;
/// `GET /api/v1/missions/:id`, measured, with [`crate::v2::core::api::client::api_get`] behind it.
///
/// The mission document is the boot bar's first segment and the API sends a `content-length` for
/// it, so it is determinate for the same reason the DEM is: budget from the header, progress from
/// the body's `ReadableStream`. What it is *not* is a second copy of the auth contract. The token is
/// injected and the body is read here; **anything that is not a 2xx  including the 401 that means
/// the access token expired  falls through to `api_get`**, which owns the single-flight refresh and
/// the one retry (`client.rs`). So this adds a fast path and cannot add a second place that spends a
/// refresh token; the worst case is one wasted GET before the real one.
///
/// A response with no `content-length` reports no budget and the segment simply stays at 0 until it
/// is finished  the bar under-claims rather than invents.
async fn get_mission_measured(
    auth: AuthStore,
    path: &str,
    report: &dyn Fn(website_map_engine::streaming::bridge::progress::BootEvent),
) -> Result<MissionDetail, crate::v2::core::api::client::ApiErr> {
    use wasm_bindgen::JsCast;
    use website_map_engine::streaming::bridge::progress::BootEvent;
    use website_map_engine::streaming::bridge::progress::BootSeg;
    use website_map_engine::streaming::bridge::progress::STREAM_REPORT_BYTES;

    let measured = async {
        let token = auth.access_token.get_untracked()?;
        let resp = gloo_net::http::RequestBuilder::new(&format!("/api/v1{path}"))
            .method(gloo_net::http::Method::GET)
            .credentials(web_sys::RequestCredentials::Include)
            .header("Authorization", &format!("Bearer {token}"))
            .build()
            .ok()?
            .send()
            .await
            .ok()?;
        if !(200..300).contains(&resp.status()) {
            return None;
        }
        let budget = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        report(BootEvent::Budget(BootSeg::Mission, budget));
        let body = resp.body()?;
        let reader: web_sys::ReadableStreamDefaultReader = body.get_reader().unchecked_into();
        let mut out: Vec<u8> = Vec::with_capacity(usize::try_from(budget).unwrap_or(0));
        let mut unreported: u64 = 0;
        loop {
            let chunk = wasm_bindgen_futures::JsFuture::from(reader.read())
                .await
                .ok()?;
            let done = js_sys::Reflect::get(&chunk, &"done".into())
                .ok()
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            if done {
                break;
            }
            let arr: js_sys::Uint8Array = js_sys::Reflect::get(&chunk, &"value".into())
                .ok()?
                .unchecked_into();
            let at = out.len();
            out.resize(at + arr.length() as usize, 0);
            arr.copy_to(&mut out[at..]);
            unreported += u64::from(arr.length());
            if unreported >= STREAM_REPORT_BYTES {
                report(BootEvent::Done(BootSeg::Mission, unreported));
                unreported = 0;
            }
        }
        if unreported > 0 {
            report(BootEvent::Done(BootSeg::Mission, unreported));
        }
        serde_json::from_slice::<MissionDetail>(&out).ok()
    }
    .await;
    match measured {
        Some(d) => Ok(d),
        None => crate::v2::core::api::client::api_get::<MissionDetail>(auth, path).await,
    }
}

/// Fetch `GET /missions/:id` and reconcile it with the just-loaded local doc:
///  * new mission (empty server payload) → apply the row terrain only;
///  * no local content  no IDB record, or one that decodes to an empty document → hydrate the
///    server payload, mark adopted, refresh;
///  * local content a hydrate of the server payload would reproduce exactly → trust local silently;
///  * local content that genuinely differs → set `conflict` so the UI can prompt.
///
/// `loaded_from_idb` is the persist layer's flag. On any non-404 failure it leaves the doc as-is
/// (local-only)  the caller shows no blocking error (the editor is usable on the local copy).
///
/// `report` carries the mission segment of the boot bar. The document GET is the one part
/// of this function with a number on it, and it is not a small one: the editor's own payload runs
/// from ~700 B on a fresh mission to the ~142 MB  measured at 367k slots, so a boot bar that
/// treated it as a rounding error would sit full while the operator waited on it. See
/// [`get_mission_measured`].
pub async fn hydrate_from_server(
    doc: DocHandle,
    id: String,
    auth: AuthStore,
    loaded_from_idb: bool,
    current_semver: RwSignal<Option<String>>,
    conflict: RwSignal<Option<crate::v2::apps::editor::mission_editor::ConflictInfo>>,
    report: website_map_engine::streaming::bridge::progress::ProgressFn,
) {
    // The recovery surface is bound on every editor boot, not only when a conflict fires: after a
    // reload the in-session snapshot is gone and the stored record is the only copy, and that
    // reload is exactly when someone reaches for it. It also re-binds the live-editor identity
    // every boot  `doc` is the very handle the mount created — which is what makes the
    // cross-mission refusal in `restore_snapshot` exact rather than best-effort.
    register_mission_backup(id.clone(), &doc);
    // The residue purge, and three properties of THIS position, all three of them the point:
    //   * **before `is_uuid`** a local-only id returns two lines down, and the residue is one
    //     global key set per browser rather than anything to do with which mission was opened.
    //   * **before the fetch** so a network failure or an expired session cannot skip it, which
    //     is the open that no other path reaches.
    //   * **before every branch** so no branch added later can miss it.
    //
    // **Do not move this below a branch, a `return`, or the fetch.** Nothing else clears the
    // residue, so a boot this line misses is a browser that keeps it.
    crate::v2::apps::editor::shell::session::purge_legacy_markers();
    if !is_uuid(&id) {
        return;
    }
    let path = format!("/missions/{id}");
    let detail = match get_mission_measured(auth, &path, report.as_ref()).await {
        Ok(d) => d,
        Err((404, _)) => return, // ad-hoc/local-only id — stay local, silently
        Err(_) => {
            crate::v2::core::ui::toast::use_toasts()
                .error("Could not load the saved version — editing your local copy.");
            return;
        }
    };

    // Hand the row to Export before any branch below can return. This is the ONLY place the editor
    // ever sees the author and the player cap, and the compiled export is built from them; every
    // path past here  fresh mission, warm reopen, conflict prompt — is a state in which the author
    // may still hit Export. Recording it once here rather than per-branch is what makes the
    // recorded row's `None` mean exactly what it claims: the row never arrived, not "it arrived
    // down a branch nobody wired".
    crate::v2::apps::editor::shell::document_commands::set_row_meta(&detail);

    let row = row_meta_from_detail(&detail);
    let version = detail.current_version.as_ref();
    let semver = version.map(|v| v.semver.clone());
    current_semver.set(semver.clone());

    // The editor superset lives in `current_version.json_payload`; empty `{}` = a fresh mission.
    let payload = version.map(|v| &v.json_payload);
    let is_empty = payload
        .map(|p| p.as_object().is_none_or(serde_json::Map::is_empty))
        .unwrap_or(true);

    if is_empty {
        // A real mission with no saved version yet. The editor mounts on a fixture seed, so on the
        // FIRST open  no local record — the seed is cleared to match what a save would mean: a
        // save must not round-trip fixture data. A warm reopen keeps the operator's local work.
        if !loaded_from_idb {
            adopt_payload(&doc, "{}", &row, Adopt::Init, &after_local_edit);
            crate::v2::apps::editor::bridge::document_host::history::set_dirty(false);
        } else {
            apply_row_meta_only(&doc, &row);
        }
        return;
    }
    let server = payload.unwrap();
    let payload_json = serde_json::to_string(server).unwrap_or_default();

    if loaded_from_idb {
        // A warm reopen. The decision is what the two documents CONTAIN, and the engine owns it.
        let Some(local) = classify_local_draft(&doc, server, &payload_json) else {
            // No document to classify  the editor unmounted mid-boot and cleared the `Option`.
            // Adopting and prompting would both act on something that is gone.
            return;
        };
        match local {
            // Nothing authored locally: the record decoded to an empty document. That is the cold
            // boot's situation, so it takes the cold boot's treatment  an adopt that is not an
            // undo step (the first undo would otherwise restore an empty document) and no snapshot,
            // because there is nothing to lose.
            LocalDraftVerdict::Empty => {
                adopt_payload(&doc, &payload_json, &row, Adopt::Init, &after_local_edit);
                crate::v2::apps::editor::bridge::document_host::history::set_dirty(false);
            }
            // Local IS the server's document. Nothing to choose between, so nothing to ask —
            // correcting `dirty` is the whole of this branch's work.
            //
            // That clean flag is EARNED rather than assumed. A restore from the local record is
            // marked dirty on arrival because nothing on that path can prove the restored blob was
            // ever saved; here the two documents have been compared and found equal, which is that
            // proof, so a save-then-reopen no longer shows an unsaved dot over a zero delta.
            LocalDraftVerdict::MatchesServer => {
                crate::v2::apps::editor::bridge::document_host::history::set_dirty(false);
            }
            // Two different documents  ask. The price of asking on every real difference is that
            // reopening a tab holding unsaved edits against the current version prompts. That is
            // deliberate: "Keep local" is one click and "Load server" is reversible twice over,
            // while a document silently replaced by one it never derived from is neither.
            LocalDraftVerdict::Diverged => {
                // Describe BOTH options before asking. The counts are the same numbers the
                // classification just compared, so the prompt cannot disagree with the decision
                // that raised it, and the instants are measured rather than guessed: the local one
                // is the write stamp the draft save leaves on the record it actually landed (absent
                // means this browser has no draft stamp, which is the honest answer rather than
                // "now"), and the server one is the version row's own creation time.
                let local_objects = doc.borrow().as_ref().map_or(0, MissionDocCore::slot_count);
                let local_saved = crate::v2::apps::editor::shell::persist::draft_written_at(&id)
                    .map_or_else(
                        || "not recorded on this browser".to_string(),
                        |at| tab_lock::ago(js_sys::Date::now(), at),
                    );
                let server_saved = version.map_or_else(
                    || tab_lock::short_utc(&detail.updated_at),
                    |v| tab_lock::short_utc(&v.created_at),
                );
                conflict.set(Some(
                    crate::v2::apps::editor::mission_editor::ConflictInfo {
                        local_objects,
                        server_objects: server_slot_count(server),
                        local_saved,
                        server_saved,
                        payload_json,
                        semver,
                    },
                ));
            }
        }
    } else {
        // Empty local → adopt the server payload (replaces the seed). Cold doc: INIT, no snapshot.
        adopt_payload(&doc, &payload_json, &row, Adopt::Init, &after_local_edit);
        crate::v2::apps::editor::bridge::document_host::history::set_dirty(false);
    }
}

/// The "Load server" resolution: adopt the offered payload, mark clean, clear the prompt.
///
/// This is the only adopt that runs over *live local work*, so it is the only one that takes a
/// [`snapshot_local`] and the only one that is an undo step. Both happen before the prompt signal is
/// cleared, so a failure to encode cannot leave the dialog gone AND the work unrecoverable.
pub fn resolve_conflict_server(
    id: String,
    conflict: RwSignal<Option<crate::v2::apps::editor::mission_editor::ConflictInfo>>,
) {
    if let (Some(c), Some(doc)) = (
        conflict.get_untracked(),
        crate::v2::apps::editor::bridge::document_host::history::doc_handle(),
    ) {
        // A new adopt opens a new restore cycle, so the counterpart slot  which holds whatever
        // document the PREVIOUS restore displaced  is now stale: `undoRestore()` would put a server
        // version from a cycle ago over current work. Drop it before the new pair is written. Safe
        // to delete and only this: `pre-restore` always holds an *adopted server* document, which is
        // one refetch away; `pre-adopt` is local work that exists nowhere else and is never dropped
        // here.
        forget_snapshot(&id, SnapshotSlot::PreRestore);
        // Capture the WHOLE local document before `hydrate` clears it. Synchronous encode (so the
        // bytes are pre-mutation by construction), deferred IDB write, own record key.
        let saved = snapshot_local(&doc, &id, SnapshotSlot::PreAdopt);
        // The payload carries its own map.terrain; the compile drops the title, so leave the
        // existing title untouched (row meta isn't refetched here).
        adopt_payload(
            &doc,
            &c.payload_json,
            &RowMeta::default(),
            Adopt::Undoable,
            &after_local_edit,
        );
        crate::v2::apps::editor::bridge::document_host::history::set_dirty(false);
        // Tell the user the door swings both ways  the modal can't (it is gone by the next line),
        // and an undo nobody knows about is not a recovery.
        notify(&match saved {
            Some(n) => format!(
                "Loaded the server version. Your local copy ({n} objects) was backed up — press Ctrl/Cmd+Z to put it back."
            ),
            None => "Loaded the server version. Press Ctrl/Cmd+Z to undo.".to_string(),
        });
    }
    conflict.set(None);
}

/// The "Keep local" resolution (React `resolveConflict('local')`): local knowingly diverges, so mark
/// it dirty. Clears the conflict signal.
///
/// `_mission_id` preserves the conflict-dialog callback signature; the decision uses the live
/// document and conflict signal.
pub fn resolve_conflict_local(
    _mission_id: String,
    conflict: RwSignal<Option<crate::v2::apps::editor::mission_editor::ConflictInfo>>,
) {
    crate::v2::apps::editor::bridge::document_host::history::set_dirty(true);
    conflict.set(None);
}

/// The mission row, as the editor's document wants it. The row arrives as the API's wire shape and
/// this is the one place that shape is read; `briefing` is the library blurb string on the row, not
/// the per-faction briefing object the payload carries.
fn row_meta_from_detail(d: &MissionDetail) -> RowMeta {
    RowMeta {
        title: d.title.clone(),
        terrain: d.terrain.clone(),
        time_of_day: d.time_of_day.clone(),
        weather: d.weather.clone(),
        briefing: d.briefing.clone().unwrap_or_default(),
    }
}
