use super::*;
use repository_layout::{ARCHIVE_DIR, PENDING_MERGE_DIR, RETIRED_DOCS_ROOT, TICKET_DOCUMENTS_DIR};
use repository_layout::{
    documentation::DOCUMENTATION_ROOT, documentation::GAP_ANALYSIS, documentation::ROADMAP,
};

/// Top-level folders the span derives rather than lists: today's code trees and two that no list
/// has ever named.
const DERIVED_TOP_LEVEL_FOLDERS: [&str; 8] = [
    "apps",
    "tools",
    "contracts",
    "assets",
    "crates",
    "deploy",
    "engines",
    "a_folder_born_later",
];

#[test]
fn a_path_is_within_a_folder_only_at_a_component_boundary() {
    assert!(is_within("apps", "apps"));
    assert!(is_within("apps/a/b.rs", "apps"));
    assert!(is_within("apps/a/b.rs", "apps/a"));
    assert!(!is_within("apps_extra/x", "apps"));
    assert!(!is_within("app", "apps"));
    assert!(
        is_within("anything", ""),
        "the repository root holds every path"
    );
}

#[test]
fn paths_split_into_folder_and_name() {
    assert_eq!(parent_folder("apps/a/b.rs"), "apps/a");
    assert_eq!(parent_folder("top.md"), "");
    assert_eq!(file_name("apps/a/b.rs"), "b.rs");
    assert_eq!(file_name("top.md"), "top.md");
    assert_eq!(join("apps/a", README), "apps/a/README.md");
    assert_eq!(join("", README), "README.md");
}

/// The span is every folder below the repository root, so a top-level folder is judged the moment
/// it is tracked; the exempt folders and the retired documentation root stay outside it.
#[test]
fn the_readme_span_is_every_folder_below_the_repository_root() {
    for top_level in DERIVED_TOP_LEVEL_FOLDERS
        .into_iter()
        .chain([DOCUMENTATION_ROOT])
    {
        assert!(in_readme_span(top_level), "{top_level} is judged");
        let deep = format!("{top_level}/deep/folder");
        assert!(in_readme_span(&deep), "{deep} is judged");
    }
    assert!(in_readme_span(ARCHIVE_DIR));
    assert!(!in_readme_span(PENDING_MERGE_DIR));
    assert!(!in_readme_span(&format!("{PENDING_MERGE_DIR}/writer")));
    assert!(!in_readme_span(RETIRED_DOCS_ROOT));
    assert!(!in_readme_span(&format!("{RETIRED_DOCS_ROOT}/images")));
    for hidden in [
        ".ai",
        ".ai/tickets",
        ".github/workflows",
        ".cursor/rules",
        ".cargo",
    ] {
        assert!(!in_readme_span(hidden), "{hidden} is hidden");
    }
    assert!(!in_readme_span(""));
}

/// A file lies in a code tree when any top-level folder but the two documentation roots holds
/// it; a file at the repository root lies in none.
#[test]
fn every_top_level_folder_but_the_documentation_roots_is_a_code_tree() {
    for top_level in DERIVED_TOP_LEVEL_FOLDERS {
        let file = format!("{top_level}/area/NOTES.md");
        assert!(in_code_tree(&file), "{file} lies in a code tree");
    }
    for outside in [
        format!("{DOCUMENTATION_ROOT}/runbooks/deploy.md"),
        format!("{RETIRED_DOCS_ROOT}/guide.md"),
        "CLAUDE.md".to_string(),
        README.to_string(),
    ] {
        assert!(!in_code_tree(&outside), "{outside} lies in no code tree");
    }
}

#[test]
fn test_generated_and_hidden_folders_are_exempt_with_their_subtrees() {
    assert!(below_exempt_folder("apps/x/tests"));
    assert!(below_exempt_folder("apps/x/tests/fixtures"));
    assert!(below_exempt_folder("apps/x/generated/models"));
    assert!(below_exempt_folder("apps/x/.hidden/rules"));
    assert!(!below_exempt_folder("apps/x/test_fixtures"));
    assert!(!below_exempt_folder("apps/x/latests"));
    assert!(!below_exempt_folder("apps/x/src"));
}

#[test]
fn exempt_folders_lie_outside_the_readme_span() {
    for exempt in [
        "apps/x/tests",
        "apps/x/tests/fixtures",
        "tools/x/generated",
        "apps/x/.cfg",
        "apps/x/.cfg/nested",
    ] {
        assert!(!in_readme_span(exempt), "{exempt} is exempt");
    }
    let tests_below_documentation = format!("{DOCUMENTATION_ROOT}/topic/tests");
    assert!(!in_readme_span(&tests_below_documentation));
    assert!(in_readme_span("apps/x/test_fixtures"));
    assert!(in_readme_span(&format!("{DOCUMENTATION_ROOT}/topic")));
}

#[test]
fn generated_folder_exemption_matches_the_lowercase_and_capitalised_spellings() {
    let generated_below_documentation = format!("{DOCUMENTATION_ROOT}/topic/Generated");
    for exempt in [
        "apps/x/generated",
        "apps/x/generated/models",
        "apps/mod/x/Policy/Generated",
        "apps/mod/x/Policy/Generated/weapon",
        &generated_below_documentation,
    ] {
        assert!(below_exempt_folder(exempt), "{exempt} is exempt");
        assert!(
            !in_readme_span(exempt),
            "{exempt} lies outside the README span"
        );
    }
}

#[test]
fn generated_folder_exemption_ignores_other_casings_and_longer_names() {
    for judged in [
        "apps/x/GENERATED",
        "apps/x/GENERATED/models",
        "apps/x/generated_data",
        "apps/x/Generated_data",
        "apps/x/regenerated",
    ] {
        assert!(!below_exempt_folder(judged), "{judged} is judged");
        assert!(
            in_readme_span(judged),
            "{judged} lies inside the README span"
        );
    }
}

#[test]
fn generated_folder_exemption_leaves_the_test_folder_match_exact() {
    assert!(below_exempt_folder("apps/x/tests"));
    for judged in ["apps/x/Tests", "apps/x/TESTS", "apps/x/tests_data"] {
        assert!(!below_exempt_folder(judged), "{judged} is judged");
        assert!(
            in_readme_span(judged),
            "{judged} lies inside the README span"
        );
    }
}

#[test]
fn markdown_is_any_letter_case_of_the_md_extension() {
    assert!(is_markdown("apps/a/NOTES.md"));
    assert!(is_markdown("apps/a/NOTES.MD"));
    assert!(is_markdown("README.md"));
    assert!(!is_markdown("apps/a/rules.mdc"));
    assert!(!is_markdown("apps/a/md"));
    assert!(!is_markdown("apps/a/.md"));
}

#[test]
fn the_size_limit_skips_frozen_pending_and_sync_managed_documents() {
    for exempt in [
        format!("{TICKET_DOCUMENTS_DIR}/specs/t1_x.md"),
        format!("{ARCHIVE_DIR}/topic/old.md"),
        format!("{PENDING_MERGE_DIR}/writer/source.md"),
        ROADMAP.to_string(),
        GAP_ANALYSIS.to_string(),
    ] {
        assert!(is_size_exempt(&exempt), "{exempt} should be exempt");
    }
    for live in [
        format!("{DOCUMENTATION_ROOT}/README.md"),
        format!("{DOCUMENTATION_ROOT}/runbooks/deploy.md"),
        format!("{DOCUMENTATION_ROOT}/website/refactor_notes.md"),
        format!("{DOCUMENTATION_ROOT}/refactor_program_plan.md"),
        format!("{DOCUMENTATION_ROOT}/restructure/program_plan.md"),
    ] {
        assert!(!is_size_exempt(&live), "{live} is a live document");
    }
}
