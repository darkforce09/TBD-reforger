//! One line for an error and every cause beneath it.
//!
//! **Role:** [`error_chain_text`] writes an error the way `anyhow` writes `{error:#}`: its own
//! message, then `: ` and each source in turn.
//! **Position:** the ticket crates' checks fold a failure into one finding line with it; their
//! error types keep each cause as a source, so the line names every step.
//! **Signals & state:** none; a pure function.
//! **Invariants:** each error in the chain appears once, outermost first.

use std::error::Error;

/// `error` and every source beneath it, joined with `: `.
pub fn error_chain_text(error: &(dyn Error + 'static)) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(next) = cause {
        text.push_str(": ");
        text.push_str(&next.to_string());
        cause = next.source();
    }
    text
}
