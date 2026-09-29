//! The numbered list of real actions the operator approves before a procedure runs, and after a
//! stopped run.
//!
//! **Role:** the [`PlannedAction`] value each procedure supplies, and the rendering of
//! `staging action-list <procedure> [--cases] [--recovery]`.
//!
//! **Position:** filled by `fleet_procedure/`, `discord_procedure/` and `load_procedure/`;
//! printed by `dispatch.rs`.
//!
//! **Signals & state:** none; values and pure rendering.
//!
//! **Invariants:** every action is numbered from 1 in the order it happens; an action lists the
//! exact commands it runs, so anything off the list is visibly unapproved; an empty list renders
//! as a line saying the procedure declares none, never as nothing.

use std::fmt::Write as _;

use crate::commands::staging::procedure_runner::step::DeclaredCase;

/// One real action: what happens, who does it, and the exact commands or clicks it involves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedAction {
    /// Who performs it: `harness`, `orchestrator (browser)`, `operator`.
    pub actor: &'static str,
    /// One line saying what happens.
    pub summary: String,
    /// The exact commands, clicks or effects, one per line.
    pub details: Vec<String>,
}

impl PlannedAction {
    /// An action with no detail lines.
    pub(crate) fn new(actor: &'static str, summary: impl Into<String>) -> Self {
        Self {
            actor,
            summary: summary.into(),
            details: Vec::new(),
        }
    }

    /// This action with one more detail line.
    pub(crate) fn detail(mut self, line: impl Into<String>) -> Self {
        self.details.push(line.into());
        self
    }
}

/// `<title>`, then `  N. [actor] summary` with each detail indented under it.
pub(crate) fn render(title: &str, actions: &[PlannedAction]) -> String {
    let mut text = format!("{title}\n");
    if actions.is_empty() {
        text.push_str("  (this procedure declares no actions)\n");
    }
    for (index, action) in actions.iter().enumerate() {
        let _ = writeln!(
            text,
            "  {}. [{}] {}",
            index + 1,
            action.actor,
            action.summary
        );
        for detail in &action.details {
            let _ = writeln!(text, "       {detail}");
        }
    }
    text
}

/// `<check>: <n> declared cases`, then one `case <check>_<name>` line each, with `NOT RUN
/// (missing: …)` after a case whose dependency the environment lacks.
pub(crate) fn render_cases(check: &str, cases: &[DeclaredCase]) -> String {
    let mut text = format!("{check}: {} declared cases\n", cases.len());
    for case in cases {
        let _ = write!(text, "  case {check}_{}", case.name.as_str());
        match &case.unavailable_dependency {
            Some(missing) => {
                let _ = writeln!(text, " ... NOT RUN (missing: {missing})");
            }
            None => text.push('\n'),
        }
    }
    text
}
