//! The lines a recorded run prints while it waits: which step it awaits, where a browser read goes,
//! and how each awaited effect ended.
//!
//! **Role:** formats the `AWAIT` line of a step, the browser inbox hint, and one outcome line per
//! effect.
//!
//! **Position:** written by `procedure_runner/runner.rs` to the run's output, which the
//! orchestrator reads while the harness runs in the background.
//!
//! **Signals & state:** none; pure formatting.
//!
//! **Invariants:** an `AWAIT` line is one line (`AWAIT <step>: <instruction>`), so the
//! orchestrator can find it by prefix; the harness prints it and never reads an answer.

use std::path::Path;

/// `AWAIT <step>: <instruction>`, with any line break in the instruction folded to a space.
pub(crate) fn await_line(step: &str, instruction: &str) -> String {
    let instruction: String = instruction
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    format!("AWAIT {step}: {instruction}")
}

/// `  browser inbox: save the page read as <path>` for a step that reads the browser.
pub(crate) fn inbox_hint(path: &Path) -> String {
    format!(
        "  browser inbox: save the Chrome tool's output with its capture time as {}",
        path.display()
    )
}

/// `  <step>.<effect> ok (<summary>)` or `  <step>.<effect> FAILED (<why>)`.
pub(crate) fn effect_line(step: &str, effect: &str, outcome: Result<&str, &str>) -> String {
    match outcome {
        Ok(summary) => format!("  {step}.{effect} ok ({summary})"),
        Err(why) => format!("  {step}.{effect} FAILED ({why})"),
    }
}
