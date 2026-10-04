//! An artifact's provenance and the findings its compile reported.
//!
//! **Role:** renders what an artifact was compiled from and what it is — the version, the compiler
//! and schema versions, the modpack and its version, the terrain, the compiled document's size and
//! SHA-256, and the artifact digest — and the list of compile findings with each one's rule,
//! severity, subject and message.
//! **Position:** inside the approvals drawer, above the decision; the findings list also heads the
//! read-only review workspace.
//! **Signals & state:** none; the artifact is handed in already read.
//! **Invariants:** digests are shown whole and selectable, because they are what a reviewer
//! compares; only the headline uses the short form. An artifact compiled with no current modpack
//! says so rather than showing an empty cell.

use crate::review_wording::{diagnostic_subject, document_size_label, severity_tone};
use frontend_api_dtos::ArtifactDiagnostic;
use frontend_api_dtos::MissionArtifact;
use frontend_ui::badge_class;
use frontend_ui::utc_timestamp::utc_label;
use leptos::prelude::*;

/// The provenance rows of one artifact, as `(label, value)` pairs, in reading order.
pub fn provenance_rows(artifact: &MissionArtifact, semver: &str) -> Vec<(&'static str, String)> {
    let modpack = match (&artifact.modpack_id, &artifact.modpack_version) {
        (Some(id), Some(version)) => format!("{id} · version {version}"),
        (Some(id), None) => id.to_string(),
        (None, _) => "None — no modpack was current at compile time".to_string(),
    };
    vec![
        ("Version", format!("v{semver}")),
        ("Compiled", utc_label(&artifact.created_at)),
        ("Compiler", artifact.compiler_version.clone()),
        ("Schema", artifact.schema_version.clone()),
        ("Modpack", modpack),
        ("Terrain", artifact.terrain.clone()),
        (
            "Document size",
            document_size_label(artifact.document_bytes),
        ),
        ("Document SHA-256", artifact.document_sha256.clone()),
        ("Artifact digest", artifact.artifact_digest.clone()),
    ]
}

/// The provenance of one artifact, and its compile findings.
pub fn artifact_provenance(artifact: &MissionArtifact, semver: &str) -> impl IntoView {
    view! {
        <section class="space-y-3" data-testid="artifact-provenance">
            <dl class="grid grid-cols-1 gap-x-4 gap-y-2 text-sm sm:grid-cols-[10rem_1fr]">
                {provenance_rows(artifact, semver)
                    .into_iter()
                    .map(|(label, value)| {
                        view! {
                            <dt class="font-mono text-label-sm tracking-wider text-outline uppercase">
                                {label}
                            </dt>
                            <dd class="min-w-0 break-all font-mono text-code-md text-on-surface select-all">
                                {value}
                            </dd>
                        }
                    })
                    .collect_view()}
            </dl>
            {diagnostics_list(&artifact.diagnostics)}
        </section>
    }
}

/// The findings a compile reported, or a line saying there were none.
pub fn diagnostics_list(diagnostics: &[ArtifactDiagnostic]) -> impl IntoView {
    if diagnostics.is_empty() {
        return view! {
            <p class="text-sm text-on-surface-variant">"The compile reported no findings."</p>
        }
        .into_any();
    }
    view! {
        <div>
            <p class="mb-2 text-sm text-on-surface">
                {format!("The compile reported {} finding(s):", diagnostics.len())}
            </p>
            <ul class="space-y-2" data-testid="artifact-diagnostics">
                {diagnostics
                    .iter()
                    .map(|finding| {
                        view! {
                            <li class="rounded-lg border border-white/10 bg-black/20 px-3 py-2 text-sm">
                                <p class="flex flex-wrap items-center gap-2">
                                    <span class=badge_class(severity_tone(&finding.severity))>
                                        {finding.severity.clone()}
                                    </span>
                                    <span class="font-mono text-code-md text-on-surface">
                                        {finding.rule_id.to_string()}
                                    </span>
                                    <span class="font-mono text-code-md text-on-surface-variant">
                                        {diagnostic_subject(finding)}
                                    </span>
                                </p>
                                <p class="mt-1 text-on-surface-variant">{finding.message.clone()}</p>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </div>
    }
    .into_any()
}
