//! Tests for [`super`] — each gate reports the library's judgement of this checkout.
//!
//! The laws themselves are tested beside them in
//! `tools_v2/verification-core/src/repository_laws/workspace_laws/tests/`; these pin the gates'
//! delegation over this checkout, the frontend layer table xtask passes in, and the refusal of a
//! checkout the gates cannot read.

use super::*;
use verification_core::repository_laws::workspace_laws::frontend_layering::layering_edges;

/// Every workspace law, in the order the task row runs them.
const WORKSPACE_LAWS: &[WorkspaceLaw] = &[
    WorkspaceLaw::CrateTiers,
    WorkspaceLaw::CrateAnatomy,
    WorkspaceLaw::Strangler,
    WorkspaceLaw::FrontendLayering,
    WorkspaceLaw::TailwindSources,
];

fn this_repo() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("tools_v2/xtask sits two levels below the repository root")
        .to_path_buf()
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
fn frontend_layering_the_layer_table_maps_every_frontend_source_and_sits_at_its_ceiling() {
    let (edges, unmapped) = layering_edges(&this_repo(), &FRONTEND_LAYERS[0]).unwrap();
    assert_eq!(unmapped, Vec::<String>::new());
    let production = edges.iter().filter(|edge| !edge.test).count();
    assert_eq!(
        (production, edges.len() - production),
        (
            FRONTEND_LAYERING_CEILING.production,
            FRONTEND_LAYERING_CEILING.test
        ),
        "the ratchet ceiling equals today's edges: {edges:#?}"
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
