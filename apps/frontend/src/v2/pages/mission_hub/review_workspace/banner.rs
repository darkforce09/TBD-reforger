//! The persistent banner over a review workspace: which artifact and version it shows, that nothing
//! is saved, and what the artifact's compile reported.
//!
//! **Role:** names the artifact under review by its short digest and the version it compiled from,
//! states that the workspace writes nothing, and lists the compile findings with each one's rule,
//! severity, subject and message.
//! **Position:** fixed under the editor's command strip, above its chrome and below its dialogs.
//! **Signals & state:** none; the findings list folds with the browser's own disclosure control.
//! **Invariants:** the banner stays for as long as the workspace is open — the editor looks the
//! same as the Mission Creator, and this is what tells a reviewer they are not editing the mission.

use crate::v2::core::api::dto::ReviewWorkspace;
use crate::v2::core::api::endpoints::encode_path_segment;
use crate::v2::core::ui::MaterialIcon;
use crate::v2::core::utils::utc_timestamp::utc_label;
use crate::v2::pages::mission_hub::mission_review::artifact_provenance_view::diagnostics_list;
use crate::v2::pages::mission_hub::mission_review::review_wording::short_digest;
use leptos::prelude::*;

/// The banner's headline: which artifact the workspace shows, and from which version.
pub(crate) fn review_workspace_title(artifact_digest: &str, semver: &str) -> String {
    format!(
        "Review workspace of artifact {}, version {semver}",
        short_digest(artifact_digest)
    )
}

/// What the workspace does with an edit.
pub(crate) const NOTHING_IS_SAVED: &str = "Read-only: nothing here is saved — no draft, no \
                                           version, no submission, no mission settings. Edits stay \
                                           in this tab and are discarded when it closes.";

/// The findings summary line.
pub(crate) fn findings_summary(count: usize) -> String {
    match count {
        0 => "The compile reported no findings".to_string(),
        1 => "The compile reported 1 finding".to_string(),
        n => format!("The compile reported {n} findings"),
    }
}

/// The banner over one review workspace.
pub(crate) fn review_banner(workspace: &ReviewWorkspace) -> impl IntoView {
    let artifact = &workspace.artifact;
    let title = review_workspace_title(&artifact.artifact_digest, &workspace.version.semver);
    let compiled = format!(
        "{} · compiled {} · document {}",
        artifact.metadata.title,
        utc_label(&artifact.created_at),
        short_digest(&artifact.document_sha256)
    );
    let findings = findings_summary(artifact.diagnostics.len());
    let has_findings = !artifact.diagnostics.is_empty();
    let mission_href = format!("/missions/{}", encode_path_segment(&artifact.mission_id));
    view! {
        <div class="pointer-events-none fixed inset-x-0 top-14 z-40 flex justify-center px-4">
            <div
                role="status"
                data-testid="review-workspace-banner"
                class="pointer-events-auto w-full max-w-2xl rounded-xl border border-tertiary/40 bg-surface-container/95 p-4 text-sm shadow-2xl"
            >
                <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                        <p class="flex items-center gap-2 font-semibold text-on-surface">
                            <MaterialIcon name="visibility" class="text-[18px] text-tertiary" />
                            {title}
                        </p>
                        <p class="mt-1 text-xs text-on-surface-variant">{NOTHING_IS_SAVED}</p>
                        <p class="mt-1 font-mono text-code-md text-outline">{compiled}</p>
                    </div>
                    <a
                        href=mission_href
                        class="shrink-0 rounded-full border border-white/10 px-3 py-1 text-xs text-primary transition hover:bg-white/5"
                    >
                        "Back to the mission"
                    </a>
                </div>
                <details class="mt-3" open=has_findings>
                    <summary class="cursor-pointer text-xs font-medium text-on-surface">
                        {findings}
                    </summary>
                    <div class="custom-scrollbar mt-2 max-h-56 overflow-y-auto">
                        {diagnostics_list(&artifact.diagnostics)}
                    </div>
                </details>
            </div>
        </div>
    }
}
