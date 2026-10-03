//! Tests for [`super`] — each gate reports the library's judgement of this checkout.
//!
//! The laws themselves are tested beside them in
//! `tools/foundation/repository_laws/src/workspace_laws/tests/`; these pin the gates'
//! delegation over this checkout, the frontend layer table xtask passes in, and the refusal of a
//! checkout the gates cannot read.

use super::*;
use repository_laws::workspace_laws::frontend_layering::layering_edges;

/// Every workspace law, in the order the task row runs them.
const WORKSPACE_LAWS: &[WorkspaceLaw] = &[
    WorkspaceLaw::CrateTiers,
    WorkspaceLaw::CrateAnatomy,
    WorkspaceLaw::Strangler,
    WorkspaceLaw::FrontendLayering,
    WorkspaceLaw::TailwindSources,
];

fn this_repo() -> std::path::PathBuf {
    repository_layout::find_repository_root_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("repository root")
}

#[test]
fn crate_tiers_and_every_workspace_law_pass_this_checkout() {
    for law in WORKSPACE_LAWS {
        let report = workspace_law_report(*law, &this_repo());
        assert_eq!(report.exit_code, 0, "{law:?}:\n{}", report.lines.join("\n"));
        assert!(
            report
                .lines
                .last()
                .is_some_and(|line| line.ends_with(": PASS"))
        );
    }
    for law in WORKSPACE_LAWS {
        assert_eq!(verify_workspace_law(*law, &this_repo()).unwrap(), 0);
    }
}

#[test]
fn frontend_layering_the_layer_table_maps_every_frontend_source_and_holds_no_edge() {
    let scan = layering_edges(&this_repo(), &FRONTEND_LAYERS[0]).unwrap();
    assert_eq!(scan.unmapped, Vec::<String>::new());
    assert!(
        scan.unordered.is_empty(),
        "every foundation sub-area sits in the order: {:?}",
        scan.unordered
    );
    assert_eq!(scan.edges, Vec::new(), "the law is hard at zero");
    let report = workspace_law_report(WorkspaceLaw::FrontendLayering, &this_repo());
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("FRONTEND-LAYERING: PASS")
    );
}

#[test]
fn strangler_and_every_workspace_law_refuse_a_checkout_they_cannot_read() {
    let missing = Path::new("/nonexistent/tbd-workspace-laws/gate");
    for law in WORKSPACE_LAWS {
        let report = workspace_law_report(*law, missing);
        assert_eq!(report.exit_code, 2, "{law:?}");
        assert!(
            report
                .lines
                .last()
                .is_some_and(|line| line.ends_with(": FAIL (did not run)"))
        );
    }
    for law in WORKSPACE_LAWS {
        assert_eq!(verify_workspace_law(*law, missing).unwrap(), 2);
    }
}
