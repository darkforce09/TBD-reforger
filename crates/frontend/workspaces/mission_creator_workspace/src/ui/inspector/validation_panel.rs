//! Validation panel model and exports.

use leptos::prelude::*;

use mission_validation::Finding;
use mission_validation::Primitive;
use mission_validation::RuleId;
use mission_validation::Severity;
use mission_validation::SubjectId;

/// Trailing delay between document changes and validation passes.
pub const REEVAL_DEBOUNCE_MS: f64 = 250.0;

/// Polling interval while the initial document payload becomes available.
#[cfg(target_arch = "wasm32")]
pub const INITIAL_EVAL_TICK_MS: u64 = 50;
/// Maximum number of initial payload polls.
pub const INITIAL_EVAL_MAX_TICKS: u32 = 32;

/// Owned finding data rendered by the validation list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelFinding {
    /// The rule that raised the finding.
    pub rule_id: RuleId,
    /// How severe the finding is.
    pub severity: Severity,
    /// The check the rule ran.
    pub primitive: Primitive,
    /// The operator-facing message.
    pub message: String,
    /// The subject the finding names, as shown.
    pub subject: String,
    /// The subject's id, when the finding can select it.
    pub subject_id: Option<SubjectId>,
}

impl PanelFinding {
    /// Copies an engine finding into a panel row.
    #[must_use]
    pub fn from_finding(f: &Finding) -> Self {
        Self {
            rule_id: f.rule_id.clone(),
            severity: f.severity,
            primitive: f.primitive,
            message: f.message.clone(),
            subject: f.subject.clone(),
            subject_id: f.subject_id.clone(),
        }
    }

    /// Reports whether this finding has a subject identifier.
    #[cfg(test)]
    #[must_use]
    pub fn is_selectable(&self) -> bool {
        self.subject_id
            .as_ref()
            .is_some_and(|s| !s.as_str().is_empty())
    }
}

/// Counts findings by severity for the toolbar summary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rollup {
    /// Error findings.
    pub errors: usize,
    /// Warning findings.
    pub warnings: usize,
    /// Info findings.
    pub infos: usize,
}

impl Rollup {
    /// Counts a set of findings by severity.
    #[must_use]
    pub fn of(findings: &[PanelFinding]) -> Self {
        let mut r = Rollup::default();
        for f in findings {
            match f.severity {
                Severity::Error => r.errors += 1,
                Severity::Warning => r.warnings += 1,
                Severity::Info => r.infos += 1,
            }
        }
        r
    }

    /// Returns the total number of findings.
    #[must_use]
    pub fn total(self) -> usize {
        self.errors + self.warnings + self.infos
    }

    /// Reports whether the rollup has no findings.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.total() == 0
    }

    /// Reports whether the rollup includes an error.
    #[must_use]
    pub fn has_blocking(self) -> bool {
        self.errors > 0
    }

    /// Formats the one-line severity summary.
    #[must_use]
    pub fn chip_text(self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.errors > 0 {
            parts.push(count_label(self.errors, "error"));
        }
        if self.warnings > 0 {
            parts.push(count_label(self.warnings, "warning"));
        }
        if self.infos > 0 {
            parts.push(count_label(self.infos, "info"));
        }
        parts.join(" · ")
    }
}

#[must_use]
fn count_label(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// Findings grouped under one validation rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleGroup {
    /// The rule the group collects.
    pub rule_id: RuleId,
    /// The rule's severity.
    pub severity: Severity,
    /// The rule's findings, in report order.
    pub findings: Vec<PanelFinding>,
}

impl RuleGroup {
    /// Returns the number of findings in this rule group.
    #[must_use]
    pub fn count(&self) -> usize {
        self.findings.len()
    }
}

#[must_use]
fn severity_rank(s: Severity) -> u8 {
    match s {
        Severity::Error => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    }
}

/// Groups findings by rule in severity order.
#[must_use]
pub fn group_by_rule(findings: &[PanelFinding]) -> Vec<RuleGroup> {
    let mut groups: Vec<RuleGroup> = Vec::new();
    for f in findings {
        if let Some(g) = groups.iter_mut().find(|g| g.rule_id == f.rule_id) {
            if severity_rank(f.severity) < severity_rank(g.severity) {
                g.severity = f.severity;
            }
            g.findings.push(f.clone());
        } else {
            groups.push(RuleGroup {
                rule_id: f.rule_id.clone(),
                severity: f.severity,
                findings: vec![f.clone()],
            });
        }
    }
    groups.sort_by_key(|g| severity_rank(g.severity));
    groups
}

/// Severity label and description shown in the legend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LadderRung {
    /// The severity the rung explains.
    pub severity: Severity,
    /// The severity's label.
    pub label: &'static str,
    /// What the severity means for the mission.
    pub meaning: &'static str,
}

/// Legend entries in descending severity order.
pub const SEVERITY_LADDER: [LadderRung; 3] = [
    LadderRung {
        severity: Severity::Error,
        label: "Error",
        meaning: "blocks — the mission will not compile or spawn correctly until fixed",
    },
    LadderRung {
        severity: Severity::Warning,
        label: "Warning",
        meaning: "advisory — likely a mistake (fairness, identity, a soft ceiling), but not blocking",
    },
    LadderRung {
        severity: Severity::Info,
        label: "Info",
        meaning: "a note — informational, no action required",
    },
];

/// Returns the display label for a severity.
#[must_use]
pub fn severity_tag(s: Severity) -> &'static str {
    s.as_str()
}

/// Tracks a trailing validation delay after document changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Debouncer {
    window_ms: f64,
    last_bump: Option<f64>,
}

impl Debouncer {
    /// Creates a debouncer with the specified delay.
    #[must_use]
    pub fn new(window_ms: f64) -> Self {
        Self {
            window_ms,
            last_bump: None,
        }
    }

    /// Records a document change at the supplied time.
    pub fn bump(&mut self, now: f64) -> bool {
        self.last_bump = Some(now);
        true
    }

    /// Reports whether the trailing delay has elapsed.
    #[must_use]
    pub fn should_fire(&self, now: f64) -> bool {
        match self.last_bump {
            Some(t) => now - t >= self.window_ms,
            None => false,
        }
    }

    /// Consumes a pending validation pass.
    pub fn take_fire(&mut self) -> bool {
        let had = self.last_bump.is_some();
        self.last_bump = None;
        had
    }

    /// Reports whether a validation pass is pending.
    #[cfg(test)]
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.last_bump.is_some()
    }
}

mod findings_dropdown;
mod payload_source_and_selection_routes;
mod validation_evaluation_and_findings;
pub mod validation_runtime;

#[cfg(any(target_arch = "wasm32", test))]
pub use findings_dropdown::findings_dropdown;
#[cfg(test)]
use findings_dropdown::row_cursor_class;
use mission_creator_state::seam_registration::install_seam;
#[cfg(target_arch = "wasm32")]
pub use payload_source_and_selection_routes::known_asset_ids_from_registry;
pub use payload_source_and_selection_routes::{
    PayloadSource, finding_is_routable, read_payload_source, register_payload_source,
    register_route_probe, register_select_by_id, route_select_by_subject_id, subject_id_routes,
};
pub use validation_evaluation_and_findings::{
    chip_findings, clear_compile_findings, evaluate_now, register_compile_findings_publisher,
    register_panel_sink,
};
#[cfg(test)]
pub(crate) use validation_evaluation_and_findings::{
    compile_findings, evaluate_source, publish_compile_findings,
};
#[cfg(target_arch = "wasm32")]
pub use validation_runtime::ValidationPanel;

#[cfg(test)]
#[path = "tests/validation_panel/finding_rollup_and_debounce.rs"]
mod finding_rollup_and_debounce_tests;
#[cfg(test)]
#[path = "tests/validation_panel/finding_route_probe.rs"]
mod finding_route_probe_tests;
#[cfg(test)]
#[path = "tests/validation_panel/mission_switch_compile_findings.rs"]
mod mission_switch_compile_findings_tests;
