use super::*;
use crate::core::repository_layout::documentation::{
    ARCHIVE_DIR, CODE_TREES, DOCUMENTATION_ROOT, GAP_ANALYSIS, PENDING_MERGE_DIR,
    PROGRAM_RECORDS_PREFIX, ROADMAP, TICKET_DOCUMENTS_DIR,
};

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

#[test]
fn the_readme_span_is_the_code_trees_and_the_documentation_root() {
    for tree in CODE_TREES {
        assert!(in_code_tree(tree));
        assert!(in_readme_span(tree));
        assert!(in_readme_span(&format!("{tree}/deep/folder")));
    }
    assert!(in_readme_span(DOCUMENTATION_ROOT));
    assert!(in_readme_span(ARCHIVE_DIR));
    assert!(!in_code_tree(DOCUMENTATION_ROOT));
    assert!(!in_readme_span(PENDING_MERGE_DIR));
    assert!(!in_readme_span(&format!("{PENDING_MERGE_DIR}/writer")));
    assert!(!in_readme_span(".ai/tickets"));
    assert!(!in_readme_span(""));
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
        "tools_v2/x/generated",
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
fn the_size_limit_skips_frozen_pending_record_and_sync_managed_documents() {
    for exempt in [
        format!("{TICKET_DOCUMENTS_DIR}/specs/t1_x.md"),
        format!("{ARCHIVE_DIR}/topic/old.md"),
        format!("{PENDING_MERGE_DIR}/writer/source.md"),
        format!("{PROGRAM_RECORDS_PREFIX}program_plan.md"),
        format!("{PROGRAM_RECORDS_PREFIX}move_manifest/README.md"),
        ROADMAP.to_string(),
        GAP_ANALYSIS.to_string(),
    ] {
        assert!(is_size_exempt(&exempt), "{exempt} should be exempt");
    }
    for live in [
        format!("{DOCUMENTATION_ROOT}/README.md"),
        format!("{DOCUMENTATION_ROOT}/runbooks/deploy.md"),
        format!("{DOCUMENTATION_ROOT}/website/refactor_notes.md"),
    ] {
        assert!(!is_size_exempt(&live), "{live} is a live document");
    }
}
