//! Validation panel validation evaluation and findings.

use super::*;

/// Runs validation over a compiled payload source.
#[must_use]
pub fn evaluate_source(source: &PayloadSource) -> Vec<PanelFinding> {
    use website_map_engine::data::scenario::validate::default_registry;
    use website_map_engine::data::scenario::validate::EvalContext;

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut ctx = EvalContext::default();
        if let Some(ids) = source.known_asset_ids.clone() {
            ctx = ctx.with_known_asset_ids(ids);
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
pub fn publish_compile_findings(rows: Vec<PanelFinding>) {
    COMPILE_FINDINGS.with(|c| *c.borrow_mut() = rows);
    let sink = PANEL_SINK.with(|c| *c.borrow());
    if let Some(sig) = sink {
        sig.set(evaluate_now());
    }
}

/// Returns findings from the latest compile pass.
#[must_use]
pub fn compile_findings() -> Vec<PanelFinding> {
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
