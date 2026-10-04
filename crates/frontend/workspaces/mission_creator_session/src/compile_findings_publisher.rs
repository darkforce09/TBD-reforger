//! The registered publisher the compiled export hands its compile findings to.
//!
//! **Role:** carries the structured findings one compile produced from the Export Compiled
//! command to the surface that renders them, so the session's export reports a build without
//! naming the workspace's validation panel.
//! **Position:** part of the session layer. The workspace's validation panel registers its
//! publisher here (`ui::inspector::validation_panel::register_compile_findings_publisher`), and
//! the Mission Creator page runs that registration first thing at mount, before the top strip
//! that carries the export row renders; `document_commands::export_compiled_now` calls
//! [`publish_compile_findings`](crate::compile_findings_publisher::publish_compile_findings).
//! **Signals & state:** one thread-local cell, `COMPILE_FINDINGS_PUBLISHER`, holding the
//! registered publisher; `None` until the first page mount registers it.
//! **Invariants:** a registration replaces the previous one, so a remount never stacks two
//! publishers; the registered publisher is a capture-free function, so a registration that
//! outlives its page is harmless. Every compile publishes, an empty list included, so a clean
//! compile clears the previous report. With nothing registered the findings are shown nowhere;
//! the export's toast still carries the one-line summary.

use std::cell::RefCell;
use std::rc::Rc;

use mission_validation::Finding;

/// The registered compile-findings publisher.
pub type CompileFindingsPublisher = Rc<dyn Fn(&[Finding])>;

thread_local! {
    /// The publisher the workspace's validation panel registers. Thread-local because the panel
    /// it feeds lives in `!Send` page signals.
    static COMPILE_FINDINGS_PUBLISHER: RefCell<Option<CompileFindingsPublisher>> =
        const { RefCell::new(None) };
}

/// Registers the compile-findings publisher, replacing any earlier registration.
pub fn register_compile_findings_publisher(publisher: CompileFindingsPublisher) {
    COMPILE_FINDINGS_PUBLISHER.with(|slot| *slot.borrow_mut() = Some(publisher));
}

/// Hands one compile's findings to the registered publisher. Returns `true` when a publisher ran
/// and `false` when none is registered, in which case the findings are shown nowhere.
pub fn publish_compile_findings(findings: &[Finding]) -> bool {
    let publisher = COMPILE_FINDINGS_PUBLISHER.with(|slot| slot.borrow().clone());
    match publisher {
        Some(publisher) => {
            publisher(findings);
            true
        }
        None => false,
    }
}

#[cfg(test)]
#[path = "tests/compile_findings_publisher.rs"]
mod tests;
