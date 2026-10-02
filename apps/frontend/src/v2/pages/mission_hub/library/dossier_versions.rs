//! The dossier's version rail and the "what is in this version" census.
//!
//! **Role:** turns one stored mission version into the timeline entry the dossier shows — the
//! semver, when and by whom it was saved, and a one-line summary of what it holds.
//! **Position:** a section of the mission dossier, rendered above the collaboration controls in
//! the library slide-over.
//! **Signals & state:** none — it renders a snapshot of the mission detail it is handed.
//! **Invariants:** the census is the payload comparison run against the empty document, so it can
//! never disagree with the upload preview. Only the version embedded in the mission detail can be
//! rendered: the API has no list-versions route, and the rail says so rather than implying the
//! earlier snapshots were lost.

use super::mission_diff::diff_mission_payloads;
use crate::v2::core::api::dto::MissionDetail;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;
use serde_json::Value;

/// "What is in this version" — `(label, rows)` for every non-empty collection.
///
/// This is the payload comparison with the empty document on the left, not a second row counter
/// that could drift from it.
pub(super) fn version_census(payload: &Value) -> Vec<(&'static str, usize)> {
    diff_mission_payloads(&Value::Null, payload)
        .collections
        .into_iter()
        .filter(|c| c.b_rows > 0)
        .map(|c| (c.label, c.b_rows))
        .collect()
}

/// Render a census as `"142 slots · 8 objectives · 12 markers"`.
pub(super) fn census_line(census: &[(&'static str, usize)]) -> String {
    census
        .iter()
        .map(|(label, n)| format!("{n} {}", label.to_lowercase()))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The dossier's version-history rail, or `None` when the mission has no stored version.
///
/// Renders the versions this page can genuinely obtain, which today is exactly one: the version
/// embedded in the mission detail. The closing line says so in the author's own terms rather than
/// leaving them to conclude their history was lost.
pub(super) fn version_history_section(m: &MissionDetail) -> Option<impl IntoView + use<>> {
    let v = m.current_version.as_ref()?;
    let semver = v.semver.clone();
    let saved = crate::v2::core::utils::datefmt::format_local_datetime(&v.created_at);
    // `created_by` holds a raw Discord snowflake, not a display name, and this page has no
    // directory to resolve one against. Name the author when the ids match; otherwise say nothing
    // rather than print an eighteen-digit number at the reader.
    let by = (v.created_by == m.author_id).then(|| format!(" by {}", m.author_name));
    let census = version_census(&v.json_payload);
    let contents = if census.is_empty() {
        // Reachable and true: a mission saved before the editor wrote anything stores an empty
        // payload. Saying so beats a blank line that reads as a loading bug.
        "This version stores no editor content.".to_string()
    } else {
        census_line(&census)
    };
    Some(view! {
        <section>
            <h3 class="mb-2 font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
                "Version history"
            </h3>
            <ol class="border-l border-white/10 pl-5">
                <li class="relative">
                    <span class="absolute top-1.5 -left-[23px] size-2.5 rounded-full bg-primary ring-4 ring-surface-container-high"></span>
                    <div class="flex flex-wrap items-center gap-2">
                        <span class="font-mono text-label-lg font-semibold text-on-surface">
                            {format!("v{semver}")}
                        </span>
                        <span class="rounded-full border border-primary/30 bg-primary/10 px-2 py-0.5 font-mono text-label-sm tracking-widest text-primary uppercase">
                            "Current"
                        </span>
                    </div>
                    <p class="mt-1 font-mono text-label-sm text-on-surface-variant">
                        {format!("Saved {saved}")}
                        {by}
                    </p>
                    <p class="mt-1 text-label-md text-on-surface-variant">{contents}</p>
                </li>
            </ol>
            <p class="mt-3 flex items-start gap-2 text-label-md text-on-surface-variant">
                <MaterialIcon name="history" class="mt-0.5 shrink-0 text-[16px]" />
                <span>
                    "Earlier versions of this mission are kept, but the library cannot list them yet — it can load only the current one, so there is no earlier snapshot to compare against."
                </span>
            </p>
        </section>
    })
}
