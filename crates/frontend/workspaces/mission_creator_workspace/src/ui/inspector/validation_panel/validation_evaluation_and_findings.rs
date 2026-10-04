//! Validation panel validation evaluation and findings.

use super::*;

/// Runs validation over a compiled payload source.
#[must_use]
pub(crate) fn evaluate_source(source: &PayloadSource) -> Vec<PanelFinding> {
    use mission_validation::EvalContext;
    use mission_validation::default_registry;

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut ctx = EvalContext::default();
        if let Some(ids) = source.known_asset_ids.clone() {
            ctx = ctx.with_known_asset_ids(ids.into_iter().map(Into::into).collect());
        }
        default_registry().evaluate_with_context(&source.payload, &ctx)
    }));

    match result {
        Ok(findings) => findings.iter().map(PanelFinding::from_finding).collect(),
        Err(_) => {
            #[cfg(not(target_arch = "wasm32"))]
            eprintln!("validation_panel: a rule panicked; showing no findings this tick");
            #[cfg(target_arch = "wasm32")]
            web_sys::console::error_1(
                &"validation_panel: a rule panicked; showing no findings this tick".into(),
            );
            Vec::new()
        }
    }
}

thread_local! {
    static COMPILE_FINDINGS: std::cell::RefCell<Vec<PanelFinding>> =
        const { std::cell::RefCell::new(Vec::new()) };

    static PANEL_SINK: std::cell::RefCell<Option<RwSignal<Vec<PanelFinding>>>> =
        const { std::cell::RefCell::new(None) };
}

/// Installs the toolbar findings signal.
pub fn register_panel_sink(sink: RwSignal<Vec<PanelFinding>>) {
    install_seam(&PANEL_SINK, sink);
}

/// Returns the active toolbar findings signal.
#[must_use]
pub fn chip_findings() -> Option<RwSignal<Vec<PanelFinding>>> {
    PANEL_SINK.with(|c| *c.borrow())
}

/// Stores findings produced by a compile pass.
pub(crate) fn publish_compile_findings(rows: Vec<PanelFinding>) {
    COMPILE_FINDINGS.with(|c| *c.borrow_mut() = rows);
    let sink = PANEL_SINK.with(|c| *c.borrow());
    if let Some(sig) = sink {
        sig.set(evaluate_now());
    }
}

/// Registers this panel as the compiled export's findings publisher
/// ([`mission_creator_session::compile_findings_publisher`]): each compile's findings
/// become panel rows through [`PanelFinding::from_finding`] and replace the previous compile's.
/// The Mission Creator page calls this first thing at mount, before the top strip that carries
/// the export row renders; a remount's call replaces the registration with the same function.
pub fn register_compile_findings_publisher() {
    mission_creator_session::compile_findings_publisher::register_compile_findings_publisher(
        std::rc::Rc::new(publish_compile_finding_rows),
    );
}

/// Publishes one compile's engine findings as panel rows.
fn publish_compile_finding_rows(findings: &[Finding]) {
    publish_compile_findings(findings.iter().map(PanelFinding::from_finding).collect());
}

/// Returns findings from the latest compile pass.
#[must_use]
pub(crate) fn compile_findings() -> Vec<PanelFinding> {
    COMPILE_FINDINGS.with(|c| c.borrow().clone())
}

/// Clears compile findings for a new mission.
pub fn clear_compile_findings() {
    publish_compile_findings(Vec::new());
}

/// Runs validation against the currently registered source.
#[must_use]
pub fn evaluate_now() -> Vec<PanelFinding> {
    let mut rows = match read_payload_source() {
        Some(source) => evaluate_source(&source),
        None => Vec::new(),
    };
    rows.extend(compile_findings());
    rows
}
