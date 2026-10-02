//! Mission Merge browser commands.
use super::*;

/// the author's OTHER missions, for the "Merge Mission…" picker.
///
/// Reuses the SPA's own list client (`GET /missions?scope=mine`, the same call
/// `missions::MissionLibraryPage` makes) through [`crate::v2::core::api::client::api_get`], which owns the
/// single-flight refresh  so this adds no second auth path. The CURRENT mission is filtered out
/// (you cannot merge a mission into itself). Titles come straight off the `MissionCard` rows.
///
/// # Errors
/// A display string when the list request fails (offline / 401 / server error).
pub async fn other_missions(auth: AuthStore, exclude_id: &str) -> Result<Vec<MissionPick>, String> {
    use crate::v2::core::api::dto::{MissionCard, Paginated};
    match crate::v2::core::api::client::api_get::<Paginated<MissionCard>>(
        auth,
        "/missions?scope=mine",
    )
    .await
    {
        Ok(page) => Ok(page
            .data
            .into_iter()
            .filter(|c| c.id != exclude_id)
            .map(|c| MissionPick {
                id: c.id,
                title: c.title,
            })
            .collect()),
        Err((401, _)) => Err("Sign in to list your missions.".to_string()),
        Err((s, msg)) => Err(match msg {
            Some(m) if !m.is_empty() => format!("Could not load your missions ({s}): {m}"),
            _ => format!("Could not load your missions ({s})."),
        }),
    }
}

/// merge another mission (`source_id`) into the CURRENT document.
///
/// Fetches the source's latest payload (`GET /missions/:id` → `current_version.json_payload`, the
/// same superset [`crate::v2::apps::editor::shell::hydrate`] loads), runs [`MissionDocCore::merge_mission_payload_json`]
/// on the hosted doc, and reports the outcome via toasts: a counts line plus, when the merge
/// tolerated malformed rows, an error toast listing each skipped row (the  totality contract
/// made visible to the author). `offset` is the optional template placement delta.
///
/// The whole merge is one undo step (the core opens one txn), so a mistaken merge is one Ctrl+Z.
/// The borrow of the hosted `MissionDocCore` is taken and released synchronously AFTER the
/// `.await`  never held across the yield (the module's borrow-safety contract).
pub fn merge_mission_now(
    source_id: String,
    offset: Option<(f64, f64)>,
    toasts: crate::v2::core::ui::toast::Toasts,
) {
    let Some((doc, auth)) =
        EDITOR_CTX.with(|c| c.borrow().as_ref().map(|ctx| (ctx.doc.clone(), ctx.auth)))
    else {
        toasts.error("Editor not ready.");
        return;
    };
    let path = format!("/missions/{source_id}");
    spawn_local(async move {
        let detail = match crate::v2::core::api::client::api_get::<
            crate::v2::core::api::dto::MissionDetail,
        >(auth, &path)
        .await
        {
            Ok(d) => d,
            Err((401, _)) => {
                toasts.error("Sign in to merge a mission.");
                return;
            }
            Err((404, _)) => {
                toasts.error("That mission no longer exists.");
                return;
            }
            Err((s, _)) => {
                toasts.error(format!("Could not load the mission to merge ({s})."));
                return;
            }
        };
        // The editor superset lives in `current_version.json_payload`; an empty `{}` (a
        // never-saved source) has nothing to merge  say so rather than run an empty merge.
        let payload = detail.current_version.as_ref().map(|v| &v.json_payload);
        let is_empty = payload.is_none_or(|p| p.as_object().is_none_or(serde_json::Map::is_empty));
        if is_empty {
            toasts.message(format!(
                "\"{}\" has no saved content to merge yet.",
                detail.title
            ));
            return;
        }
        let payload_json = payload
            .map(std::string::ToString::to_string)
            .unwrap_or_default();

        // Borrow the doc only now (post-await), run the merge, drop the borrow before toasting.
        let report_json = {
            let borrow = doc.borrow();
            let Some(core) = borrow.as_ref() else {
                toasts.error("Editor not ready.");
                return;
            };
            core.merge_mission_payload_json(&payload_json, offset)
        };
        // A merge is a document mutation, so it must run the SAME post-mutation tail every editor
        // mutator ends on (`editor_ops` mutators all call this): materialize → prune the selection
        // → rebind the engine slot/vehicle glyphs so the merged rows reach the GPU (they are
        // invisible on the map otherwise) → bump `doc_ver` (which drives the validation panel's
        // re-check and the attributes re-read) → set dirty → schedule the IDB persist → refresh the
        // HUD counts. `set_dirty(true)` alone (the prior code) did only the last-but-two of those,
        // so a wired merge reported success by toast while the map showed nothing. The `EDITOR_CTX`
        // doc borrow was dropped at the end of the block above; `after_local_edit` takes its own
        // `HISTORY_CTX` borrow, so this is not held across the earlier `.await`.
        crate::v2::apps::editor::bridge::document_host::history::after_local_edit();

        let (summary, skipped) = format_merge_report(&report_json);
        toasts.success(format!("{summary} (Ctrl+Z to undo.)"));
        if !skipped.is_empty() {
            let head = if skipped.len() == 1 {
                "1 row was skipped:".to_string()
            } else {
                format!("{} rows were skipped:", skipped.len())
            };
            toasts.error(format!("{head} {}", skipped.join("; ")));
        }
    });
}
