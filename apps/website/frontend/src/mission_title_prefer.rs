//! T-522 / T-505 — prefer non-blank payload title over a stale missions-row title.
//!
//! Pure helper extracted from `mission_hydrate` so Class-R runs on native
//! `cargo test -p website-frontend` (cold gate). The live hydrate glue stays
//! `#[cfg(target_arch = "wasm32")]`; without this module a prefer→`&row.title`
//! regression stayed green on CI.

/// Non-blank trimmed top-level `title` from a compiled payload (T-375 wire emit).
///
/// Prefer this over the mission-row title when adopting: hydrate loads it into meta, but
/// a subsequent `apply_row_meta` with a stale row would otherwise stomp it. Whitespace-only is not a
/// title (same spirit as `eden_chrome` / `compile_payload`).
pub(crate) fn payload_title_nonblank(payload_json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(payload_json).ok()?;
    v.get("title")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Prefer-payload rule `adopt_payload` must use: non-blank payload title, else row title.
pub(crate) fn prefer_payload_title(payload_json: &str, row_title: &str) -> String {
    payload_title_nonblank(payload_json).unwrap_or_else(|| row_title.trim().to_string())
}

#[cfg(test)]
mod t505_tests {
    use super::{payload_title_nonblank, prefer_payload_title};

    /// T-505 Class-R: prefer helper must keep authored title when the row is stale.
    ///
    /// RED: change `prefer_payload_title` to always return `row_title.trim()` (or drop prefer).
    #[test]
    fn prefer_payload_keeps_authored_over_stale_row() {
        let payload = r#"{"title":"  Authored Bridgehead  ","editor":{}}"#;
        assert_eq!(
            prefer_payload_title(payload, "Stale Library Title"),
            "Authored Bridgehead"
        );
        assert_eq!(
            prefer_payload_title(r#"{"title":"   "}"#, "  Row Title  "),
            "Row Title"
        );
        assert_eq!(
            prefer_payload_title(r#"{"editor":{}}"#, "Row Only"),
            "Row Only"
        );
    }

    #[test]
    fn payload_title_nonblank_trim() {
        assert_eq!(
            payload_title_nonblank(r#"{"title":"  Authored  "}"#).as_deref(),
            Some("Authored")
        );
        assert_eq!(payload_title_nonblank(r#"{"title":"  "}"#), None);
        assert_eq!(payload_title_nonblank(r#"{"editor":{}}"#), None);
    }

    /// T-505 Class-R: `adopt_payload` in mission_hydrate.rs must call the prefer helper.
    ///
    /// RED: pass `&row.title` straight into `apply_row_meta` (or drop `prefer_payload_title` /
    /// `payload_title_nonblank` from the adopt body).
    #[test]
    fn adopt_payload_wires_prefer_helper() {
        const SRC: &str = include_str!("mission_hydrate.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .unwrap_or(SRC);
        let adopt = production
            .split("fn adopt_payload(")
            .nth(1)
            .and_then(|s| s.split("\nfn ").next())
            .expect("adopt_payload body");
        assert!(
            adopt.contains("prefer_payload_title(")
                || adopt.contains("payload_title_nonblank(payload_json)"),
            "adopt_payload must prefer via prefer_payload_title / payload_title_nonblank; got:\n{adopt}"
        );
        assert!(
            !adopt.contains("&row.title,"),
            "adopt_payload must not pass &row.title straight into apply_row_meta (stomp); got:\n{adopt}"
        );
    }
}
