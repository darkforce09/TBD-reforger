//! Tests for [`super`] — the Tailwind-sources gate reports the library's judgement of this
//! checkout.

use super::*;

fn this_repo() -> std::path::PathBuf {
    repository_root::find_repository_root_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("repository root")
}

#[test]
fn tailwind_sources_pass_this_checkout() {
    let report = workspace_law_report(WorkspaceLaw::TailwindSources, &this_repo());
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("TAILWIND-SOURCES: PASS")
    );
    assert_eq!(
        verify_workspace_law(WorkspaceLaw::TailwindSources, &this_repo()).unwrap(),
        0
    );
}
