//! Role: merge json.
//! Position: `doc/store` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::MergeOpts;
use super::MergeReport;
use super::MissionDocCore;

impl MissionDocCore {
    /// Merge mission payload json using the supplied domain data.
    #[must_use]
    pub fn merge_mission_payload_json(
        &self,
        payload_json: &str,
        offset: Option<(f64, f64)>,
    ) -> String {
        let report = match serde_json::from_str::<serde_json::Value>(payload_json) {
            Ok(payload) => self.merge_mission_payload(&payload, MergeOpts { offset }),
            Err(e) => {
                let mut report = MergeReport::default();
                report.skipped.push((
                    "payload".into(),
                    String::new(),
                    format!("invalid JSON: {e}"),
                ));
                report
            }
        };
        report.to_json_string()
    }
}
