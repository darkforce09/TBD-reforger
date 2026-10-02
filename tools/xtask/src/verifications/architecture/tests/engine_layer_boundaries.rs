//! Tests for [`super`] — the gate reports the library's judgement of this checkout.
//!
//! The rules themselves are tested beside them in
//! `tools/verification_core/src/repository_laws/engine_layers/tests/`; these pin the gate's
//! delegation: its exit code over this checkout and over a checkout it cannot read.

use super::*;

fn this_repo() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("tools/xtask sits two levels below the repository root")
        .to_path_buf()
}

#[test]
fn engine_layer_gate_passes_this_checkout_with_every_rule_judged() {
    let report = check_engine_layers(&this_repo());
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
    assert!(report.judged_every_rule());
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("ENGINE-LAYERS: PASS")
    );
    assert_eq!(verify_engine_layers(&this_repo()).unwrap(), 0);
}

#[test]
fn engine_layer_gate_refuses_a_checkout_it_cannot_read() {
    let missing = Path::new("/nonexistent/tbd-engine-layers/gate");
    assert_eq!(verify_engine_layers(missing).unwrap(), 2);
    let report = check_engine_layers(missing);
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("ENGINE-LAYERS: FAIL (did not run)")
    );
}
