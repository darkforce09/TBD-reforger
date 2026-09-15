//! Role: export.
//! Position: `mission/compiler/payload` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Value;

/// The wire shape of `POST /missions/:id/versions`, written down **exactly once**.
#[derive(serde::Serialize)]
pub(super) struct VersionBody<'a> {
    /// Semver.
    pub(super) semver: &'a str,
    /// Editor notes.
    pub(super) editor_notes: &'a str,
    /// Payload.
    pub(super) payload: &'a Value,
}

/// Materialises a `Value`, which means it **clones the whole payload tree** under `"payload"`. That is fine for the editor's Save, which compiles its payload locally and owns it anyway; it is not fine for the browser document upload, which already holds a parsed tree — that door uses [`version_body_to_writer`] instead.
#[must_use]
pub fn version_body(semver: &str, editor_notes: &str, payload: &Value) -> Value {
    serde_json::to_value(VersionBody {
        semver,
        editor_notes,
        payload,
    })
    .unwrap_or_else(|_| Value::Null)
}

/// [`version_body`]'s JSON, serialised straight into `w` — **without building the `Value` first**.
pub fn version_body_to_writer<W: std::io::Write>(
    w: W,
    semver: &str,
    editor_notes: &str,
    payload: &Value,
) -> serde_json::Result<()> {
    serde_json::to_writer(
        w,
        &VersionBody {
            semver,
            editor_notes,
            payload,
        },
    )
}
