//! What the ballistics catalog screen shows, computed from what the API sent and what was picked.
//!
//! **Role:** the route and multipart part names of the catalog upload, whether a pair of picked
//! files may be sent, the classification of the upload's answer into an outcome with its headline,
//! and the rows of the stored-version list.
//! **Position:** read by the upload form, the validation report and the version list of the
//! `/admin/ballistics-catalogs` route; fed by the [`CatalogUploadReport`] and
//! [`BallisticsCatalogList`] wire shapes and by [`ApiRefusal`].
//! **Signals & state:** none; pure functions over their arguments.
//! **Invariants:** an upload is sendable only with both parts picked and both named `.json`. A
//! `422` whose `details` carries a `failures` array is a calibration refusal and keeps every
//! failure; any other refusal keeps the backend's sentence. The version list orders catalogs by
//! identifier and each catalog's versions newest first, and marks exactly one latest version per
//! catalog.

#[cfg(any(target_arch = "wasm32", test))]
use crate::foundation::transport::client::ApiRefusal;
#[cfg(any(target_arch = "wasm32", test))]
use crate::foundation::transport::dto::ballistics_catalogs::{
    BallisticsCatalogList, CalibrationFailure, CatalogUploadReport,
};
#[cfg(any(target_arch = "wasm32", test))]
use crate::foundation::utils::utc_timestamp::utc_label;
#[cfg(any(target_arch = "wasm32", test))]
use serde_json::Value;

/// The catalog collection route, below the API base: `GET` lists, `POST` uploads.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) const BALLISTICS_CATALOGS_PATH: &str = "/ballistics-catalogs";
/// The multipart part carrying the catalog document.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) const CATALOG_PART: &str = "catalog";
/// The multipart part carrying the calibration bundle the catalog is flown against.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) const CALIBRATION_PART: &str = "calibration";
/// The characters of a SHA-256 the version list shows.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) const SHORT_SHA_LENGTH: usize = 12;

/// One file an administrator picked: what the form shows about it.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PickedFile {
    /// The file name as the browser reports it.
    pub name: String,
    /// The size in bytes, as the browser reports it.
    pub size_bytes: f64,
}

/// Why the upload cannot be sent yet, or `None` when it can.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn upload_blocker(
    catalog: Option<&PickedFile>,
    calibration: Option<&PickedFile>,
) -> Option<String> {
    match (catalog, calibration) {
        (None, None) => Some("Choose the catalog and its calibration bundle.".into()),
        (None, Some(_)) => Some("Choose the catalog document.".into()),
        (Some(_), None) => Some("Choose the calibration bundle.".into()),
        (Some(catalog), Some(calibration)) => {
            if !is_json_name(&catalog.name) {
                Some(format!(
                    "The catalog \"{}\" is not a .json file.",
                    catalog.name
                ))
            } else if !is_json_name(&calibration.name) {
                Some(format!(
                    "The calibration bundle \"{}\" is not a .json file.",
                    calibration.name
                ))
            } else {
                None
            }
        }
    }
}

/// Whether a file name ends in `.json`, in any letter case.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn is_json_name(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".json")
}

/// A byte count in the unit that keeps it readable: bytes, KB or MB (powers of 1024).
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn file_size_label(size_bytes: f64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    if !size_bytes.is_finite() || size_bytes < 0.0 {
        "—".into()
    } else if size_bytes < KIB {
        format!("{} B", size_bytes as u64)
    } else if size_bytes < MIB {
        format!("{:.1} KB", size_bytes / KIB)
    } else {
        format!("{:.1} MB", size_bytes / MIB)
    }
}

/// What the upload answered.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq)]
pub(super) enum UploadOutcome {
    /// `201`: every calibration case passed and the version is stored.
    Accepted(CatalogUploadReport),
    /// `422` with the failed calibration cases: nothing is stored.
    CalibrationRefused(CatalogUploadReport),
    /// `409`: this version, or these exact catalog bytes, are already stored.
    Duplicate(String),
    /// Any other refusal (`400`, `413`, `415`, a `422` without failures, …) with its sentence.
    Rejected {
        /// The HTTP status.
        status: u16,
        /// The backend's sentence, or a fallback.
        message: String,
    },
    /// `401`: the session is over.
    SessionExpired,
    /// The request never reached the backend, or its answer could not be read.
    Unreachable,
}

/// How an outcome is coloured.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OutcomeTone {
    /// The version is stored.
    Success,
    /// The upload was refused for what it contains.
    Failure,
    /// The upload did not decide anything about the catalog.
    Warning,
}

/// Classify the upload's answer.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn upload_outcome(answer: Result<CatalogUploadReport, ApiRefusal>) -> UploadOutcome {
    let refusal = match answer {
        Ok(report) if report.accepted && report.failures.is_empty() => {
            return UploadOutcome::Accepted(report)
        }
        Ok(report) => return UploadOutcome::CalibrationRefused(report),
        Err(refusal) => refusal,
    };
    match refusal.status {
        0 => UploadOutcome::Unreachable,
        401 => UploadOutcome::SessionExpired,
        409 => {
            UploadOutcome::Duplicate(refusal.message_or("This catalog version is already stored"))
        }
        422 => match refusal.details.as_ref().and_then(report_from_details) {
            Some(report) => UploadOutcome::CalibrationRefused(report),
            None => UploadOutcome::Rejected {
                status: 422,
                message: refusal.message_or("The catalog or its calibration bundle is invalid"),
            },
        },
        status => UploadOutcome::Rejected {
            status,
            message: refusal.message_or("The upload was refused"),
        },
    }
}

/// The calibration report a `422` carries in its `details`: its `failures` array (required), and
/// its `cases` and `forward_samples_not_judged` counts (zero when absent). Extra keys, such as a
/// machine-readable `code`, are ignored.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn report_from_details(
    details: &serde_json::Map<String, Value>,
) -> Option<CatalogUploadReport> {
    let failures: Vec<CalibrationFailure> =
        serde_json::from_value(details.get("failures")?.clone()).ok()?;
    let count = |key: &str| {
        details
            .get(key)
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0)
    };
    Some(CatalogUploadReport {
        accepted: false,
        cases: count("cases"),
        failures,
        forward_samples_not_judged: count("forward_samples_not_judged"),
    })
}

/// The one-line headline of an outcome, and its tone.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn outcome_headline(outcome: &UploadOutcome) -> (OutcomeTone, String) {
    match outcome {
        UploadOutcome::Accepted(report) => (
            OutcomeTone::Success,
            format!(
                "Accepted — all {} calibration cases passed; the version is stored.",
                report.cases
            ),
        ),
        UploadOutcome::CalibrationRefused(report) => (
            OutcomeTone::Failure,
            format!(
                "Refused — {} of {} calibration cases failed; nothing was stored.",
                report.failures.len(),
                report.cases
            ),
        ),
        UploadOutcome::Duplicate(message) => {
            (OutcomeTone::Warning, format!("Already stored — {message}"))
        }
        UploadOutcome::Rejected { status, message } => (
            OutcomeTone::Failure,
            format!("Upload refused ({status}) — {message}"),
        ),
        UploadOutcome::SessionExpired => (
            OutcomeTone::Warning,
            "Your session has ended — sign in again, then upload.".into(),
        ),
        UploadOutcome::Unreachable => (
            OutcomeTone::Warning,
            "The upload did not reach the server — check the connection and try again.".into(),
        ),
    }
}

/// The outcome's stable name, carried on the report panel as `data-upload-outcome`.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn outcome_key(outcome: &UploadOutcome) -> &'static str {
    match outcome {
        UploadOutcome::Accepted(_) => "accepted",
        UploadOutcome::CalibrationRefused(_) => "calibration-refused",
        UploadOutcome::Duplicate(_) => "duplicate",
        UploadOutcome::Rejected { .. } => "rejected",
        UploadOutcome::SessionExpired => "session-expired",
        UploadOutcome::Unreachable => "unreachable",
    }
}

/// The failures an outcome lists: the failed calibration cases, or none.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn outcome_failures(outcome: &UploadOutcome) -> &[CalibrationFailure] {
    match outcome {
        UploadOutcome::Accepted(report) | UploadOutcome::CalibrationRefused(report) => {
            &report.failures
        }
        _ => &[],
    }
}

/// One row of the stored-version list.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq)]
pub(super) struct CatalogVersionRow {
    pub catalog_id: String,
    pub catalog_version: u32,
    pub title: String,
    pub game_build: String,
    /// The first [`SHORT_SHA_LENGTH`] characters of the catalog SHA-256.
    pub short_sha: String,
    /// The full catalog SHA-256, for the row's tooltip.
    pub catalog_sha256: String,
    /// The upload time as a UTC label.
    pub uploaded_label: String,
    /// Whether this is the highest stored version of its catalog.
    pub latest: bool,
}

/// The list's rows: catalogs by identifier, each catalog's versions newest first.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn version_rows(list: &BallisticsCatalogList) -> Vec<CatalogVersionRow> {
    let mut summaries: Vec<_> = list.data.iter().collect();
    summaries.sort_by(|a, b| {
        a.catalog_id
            .cmp(&b.catalog_id)
            .then(b.catalog_version.cmp(&a.catalog_version))
    });
    let mut rows = Vec::with_capacity(summaries.len());
    let mut previous_id: Option<&str> = None;
    for summary in summaries {
        let latest = previous_id != Some(summary.catalog_id.as_str());
        previous_id = Some(summary.catalog_id.as_str());
        rows.push(CatalogVersionRow {
            catalog_id: summary.catalog_id.clone(),
            catalog_version: summary.catalog_version,
            title: summary.title.clone(),
            game_build: summary.game_build.clone(),
            short_sha: summary
                .catalog_sha256
                .chars()
                .take(SHORT_SHA_LENGTH)
                .collect(),
            catalog_sha256: summary.catalog_sha256.clone(),
            uploaded_label: utc_label(&summary.uploaded_at),
            latest,
        });
    }
    rows
}

/// The list header's count: versions across how many catalogs.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn version_count_label(rows: &[CatalogVersionRow]) -> String {
    let catalogs = rows.iter().filter(|row| row.latest).count();
    let versions = rows.len();
    format!(
        "{versions} {} · {catalogs} {}",
        if versions == 1 { "version" } else { "versions" },
        if catalogs == 1 { "catalog" } else { "catalogs" }
    )
}
