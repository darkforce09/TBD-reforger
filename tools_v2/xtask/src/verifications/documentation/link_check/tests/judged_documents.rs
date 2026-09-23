use super::*;

fn area(path: &str) -> Option<DocumentArea> {
    judged_area(path)
}

#[test]
fn documentation_root_markdown_is_live_unless_it_is_a_frozen_record() {
    let root = DOCUMENTATION_ROOT;
    assert_eq!(
        area(&format!("{root}/runbooks/deploy.md")),
        Some(DocumentArea::LiveDocumentation)
    );
    assert_eq!(
        area(&format!("{root}/README.md")),
        Some(DocumentArea::LiveDocumentation)
    );
    assert_eq!(
        area(&format!("{TICKET_DOCUMENTS_DIR}/specs/t1_example.md")),
        Some(DocumentArea::FrozenDocumentation)
    );
    assert_eq!(
        area(&format!("{ARCHIVE_DIR}/topic/old.MD")),
        Some(DocumentArea::FrozenDocumentation)
    );
    assert_eq!(area(&format!("{root}/website/tokens.json")), None);
}

#[test]
fn the_program_records_are_never_judged() {
    assert_eq!(
        area(&format!("{PROGRAM_RECORDS_PREFIX}program_plan.md")),
        None
    );
    assert_eq!(
        area(&format!(
            "{PROGRAM_RECORDS_PREFIX}move_manifest/cp1_questions.md"
        )),
        None
    );
}

#[test]
fn the_ticket_folder_s_own_markdown_is_judged_and_nothing_below_it() {
    assert_eq!(
        area(&format!("{TICKETS_DIR}/spec_template.md")),
        Some(DocumentArea::TicketFolder)
    );
    assert_eq!(
        area(&format!("{TICKETS_DIR}/README.md")),
        Some(DocumentArea::TicketFolder)
    );
    assert_eq!(area(&format!("{TICKETS_DIR}/archive/old.md")), None);
    assert_eq!(area(&format!("{TICKETS_DIR}/T-1.toml")), None);
}

#[test]
fn cursor_rules_are_judged_as_markdown() {
    for folder in CURSOR_RULE_DIRS {
        assert_eq!(
            area(&format!("{folder}/workflow.mdc")),
            Some(DocumentArea::CursorRules)
        );
        assert_eq!(
            area(&format!("{folder}/notes.md")),
            Some(DocumentArea::CursorRules)
        );
        assert_eq!(area(&format!("{folder}/settings.json")), None);
    }
}

#[test]
fn the_project_instructions_and_every_readme_are_judged() {
    assert_eq!(
        area(PROJECT_INSTRUCTIONS),
        Some(DocumentArea::ProjectInstructions)
    );
    assert_eq!(area(README), Some(DocumentArea::Readmes));
    assert_eq!(
        area("apps/website/frontend/src/v2/README.md"),
        Some(DocumentArea::Readmes)
    );
    assert_eq!(area("apps/website/frontend/NOTES.md"), None);
    assert_eq!(area("apps/website/frontend/readme.md"), None);
    assert_eq!(area("apps/CLAUDE.md"), None);
}

#[test]
fn only_the_frozen_records_are_frozen_and_every_area_has_a_label() {
    for kind in DocumentArea::ALL {
        assert_eq!(kind.is_frozen(), kind == DocumentArea::FrozenDocumentation);
        assert!(!kind.label().is_empty());
    }
    assert_eq!(
        DocumentArea::FrozenDocumentation.label(),
        format!("{DOCUMENTATION_ROOT} frozen records")
    );
}
