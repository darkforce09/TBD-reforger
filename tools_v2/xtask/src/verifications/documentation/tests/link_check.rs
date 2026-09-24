use std::cell::RefCell;

use super::super::UntrackedFiles;
use super::super::fixture_checkout::{FixtureCheckout, failures, outcome_counts, request};
use super::repository_permalinks::{BlobObject, ObjectLookup, PermalinkObjects};
use super::*;
use crate::cli::{Cli, TopCmd};
use crate::commands::verify::cli::{DocumentationGateArgs, VerifyCmd};
use crate::core::repository_layout::TICKETS_DIR;
use crate::core::repository_layout::documentation::{
    ARCHIVE_DIR, ARTIFACTS_DIR, CURSOR_RULE_DIRS, PROGRAM_RECORDS_PREFIX, PROJECT_INSTRUCTIONS,
};
use clap::Parser;

/// A history that holds no object at all: every permalink is unknown.
struct EmptyHistory;

impl PermalinkObjects for EmptyHistory {
    fn lookup(&self, names: &[String]) -> Result<Vec<ObjectLookup>, NotRun> {
        let unknown = ObjectLookup::Unknown {
            answer: "missing".to_string(),
        };
        Ok(vec![unknown; names.len()])
    }

    fn contents(&self, blobs: &[BlobObject]) -> Result<Vec<String>, NotRun> {
        Ok(vec![String::new(); blobs.len()])
    }
}

/// A rule that judges live documents only and records which ones it saw, the way a rule that
/// skips frozen records plugs into the pipeline.
struct LiveDocumentsOnly<'s> {
    seen: &'s RefCell<Vec<String>>,
}

impl DocumentRule for LiveDocumentsOnly<'_> {
    fn judges(&self, area: DocumentArea) -> bool {
        !area.is_frozen()
    }

    fn judge(&mut self, document: &JudgedDocument<'_>, _: &RuleContext<'_>, _: &mut RuleFindings) {
        self.seen.borrow_mut().push(document.path.to_string());
    }

    fn totals(&self) -> Vec<String> {
        vec![format!(
            "  live-only rule: {} document(s)",
            self.seen.borrow().len()
        )]
    }
}

fn run(fixture: &FixtureCheckout, scope: &[&str], listing: BreakListing) -> GateRun {
    let mut rules: Vec<Box<dyn DocumentRule + '_>> =
        vec![Box::new(LinkTargets::new(&EmptyHistory))];
    judge(
        fixture.root(),
        Ok(fixture.tree()),
        &request(scope, UntrackedFiles::Invisible),
        listing,
        &mut rules,
    )
}

/// The link rule and the backticked-path rule over the whole checkout as the real listing and
/// git's own ignore rules see it.
fn run_listed_by_git(fixture: &FixtureCheckout, untracked: UntrackedFiles) -> GateRun {
    let ignore_rules = GitIgnoreRules::new(fixture.root());
    let mut rules: Vec<Box<dyn DocumentRule + '_>> = vec![
        Box::new(LinkTargets::new(&EmptyHistory)),
        Box::new(BacktickedPaths::new(&ignore_rules, &[])),
    ];
    judge(
        fixture.root(),
        fixture.listed_by_git(untracked),
        &request(&[], untracked),
        BreakListing::Every,
        &mut rules,
    )
}

/// Every judged area holds one document with one broken link; files outside the judged set hold
/// broken links that must never be reported.
fn every_area(tag: &str) -> FixtureCheckout {
    let mut fixture = FixtureCheckout::new(tag);
    let broken = "# Doc\n\n[gone](nowhere.md)\n";
    fixture
        .tracked("documentation_v2/runbooks/deploy.md", broken)
        .tracked(&format!("{ARCHIVE_DIR}/topic/old.md"), broken)
        .tracked(&format!("{TICKETS_DIR}/spec_template.md"), broken)
        .tracked(&format!("{}/workflow.mdc", CURSOR_RULE_DIRS[0]), broken)
        .tracked(PROJECT_INSTRUCTIONS, broken)
        .tracked("apps/tool/README.md", broken)
        .tracked(&format!("{PROGRAM_RECORDS_PREFIX}program_plan.md"), broken)
        .tracked(&format!("{ARTIFACTS_DIR}/README.md"), broken)
        .tracked("apps/tool/NOTES.md", broken)
        .tracked(&format!("{TICKETS_DIR}/nested/deep.md"), broken)
        .tracked("apps/tool/main.rs", "fn main() {}\n");
    fixture
}

#[test]
fn every_judged_area_is_judged_and_nothing_else() {
    let fixture = every_area("links-areas");
    let run = run(&fixture, &[], BreakListing::Every);
    let headlines: Vec<String> = failures(&run)
        .iter()
        .map(|failure| failure.lines().next().unwrap_or("").to_string())
        .collect();
    assert_eq!(
        headlines,
        [
            format!("FAIL: {TICKETS_DIR}/spec_template.md: 1 break(s)"),
            format!("FAIL: {}/workflow.mdc: 1 break(s)", CURSOR_RULE_DIRS[0]),
            format!("FAIL: {PROJECT_INSTRUCTIONS}: 1 break(s)"),
            "FAIL: apps/tool/README.md: 1 break(s)".to_string(),
            format!("FAIL: {ARCHIVE_DIR}/topic/old.md: 1 break(s)"),
            "FAIL: documentation_v2/runbooks/deploy.md: 1 break(s)".to_string(),
        ]
    );
    assert_eq!(outcome_counts(&run), (0, 6, 0));
    assert_eq!(run.print(), 1);
}

#[test]
fn the_totals_count_breaks_by_rule_and_by_area() {
    let fixture = every_area("links-totals");
    let run = run(&fixture, &[], BreakListing::Every);
    let totals = run.totals.join("\n");
    for expected in [
        "  documents: 6 judged (1 frozen record(s)), 6 with breaks, 0 unreadable",
        "  links: 6 judged — 6 into this checkout, 0 permalink(s), 0 code view(s) of this \
         repository without a full commit id, 0 external and not fetched",
        "  breaks by rule: 6 in all",
        "    missing target: 6",
        "    unknown permalink object: 0",
        "    documentation_v2 live documents: 1 break(s) in 1 of 1 document(s)",
        "    documentation_v2 frozen records: 1 break(s) in 1 of 1 document(s)",
        "    README.md files elsewhere: 1 break(s) in 1 of 1 document(s)",
    ] {
        assert!(
            totals.contains(expected),
            "missing `{expected}` in\n{totals}"
        );
    }
}

#[test]
fn every_break_prints_as_path_line_rule_message() {
    let mut fixture = FixtureCheckout::new("links-report");
    fixture.tracked(
        "documentation_v2/guide.md",
        "# Guide\n\n[a](gone.md) [b](#nowhere)\n\n[c][undefined]\n",
    );
    let run = run(&fixture, &[], BreakListing::Every);
    assert_eq!(
        failures(&run),
        ["FAIL: documentation_v2/guide.md: 3 break(s)\n      \
          documentation_v2/guide.md:3: missing target: `gone.md` resolves to \
          `documentation_v2/gone.md`, which is no tracked file or folder\n      \
          documentation_v2/guide.md:3: missing anchor: `#nowhere`: this document has no heading \
          or anchor `nowhere`\n      \
          documentation_v2/guide.md:5: undefined reference: `[undefined]` names no reference \
          definition in this document"]
    );
}

#[test]
fn without_the_report_flag_only_the_first_breaks_print_in_full() {
    let mut fixture = FixtureCheckout::new("links-first");
    let many: String = (0..15)
        .map(|index| format!("[x](gone{index}.md)\n"))
        .collect();
    fixture
        .tracked("documentation_v2/a.md", &many)
        .tracked("documentation_v2/b.md", &many)
        .tracked("documentation_v2/c.md", &many);
    let first = run(&fixture, &[], BreakListing::First);
    let detail: Vec<usize> = first
        .verdicts
        .iter()
        .map(|verdict| match verdict {
            Verdict::Failed(finding) => finding.detail.len(),
            _ => 0,
        })
        .collect();
    assert_eq!(
        detail,
        [15, 6, 0],
        "15 + 5 breaks, then a count line, then headlines only"
    );
    assert!(
        first
            .totals
            .contains(&"  the first 20 breaks are shown; --report lists all 45".to_string())
    );
    let every = run(&fixture, &[], BreakListing::Every);
    assert!(every.verdicts.iter().all(|verdict| matches!(
        verdict,
        Verdict::Failed(finding) if finding.detail.len() == 15
    )));
}

#[test]
fn a_clean_tree_holds() {
    let mut fixture = FixtureCheckout::new("links-clean");
    fixture
        .tracked("documentation_v2/guide.md", "# Guide\n\n## Setup\n")
        .tracked(
            "README.md",
            "[guide](documentation_v2/guide.md#setup) [web](https://example.com)\n",
        );
    let run = run(&fixture, &[], BreakListing::First);
    assert_eq!(outcome_counts(&run), (2, 0, 0));
    assert_eq!(run.print(), 0);
}

#[test]
fn a_link_to_an_untracked_file_resolves_only_with_untracked_files_included() {
    let mut fixture = FixtureCheckout::new("links-untracked");
    fixture
        .tracked(".gitignore", "*.log\n")
        .tracked(
            "documentation_v2/guide.md",
            "# Guide\n\n[new](new_page.md#setup) [log](run.log)\n\n\
             See `documentation_v2/new_page.md` and `documentation_v2/run.log`.\n",
        )
        .untracked("documentation_v2/new_page.md", "# New page\n\n## Setup\n")
        .untracked("documentation_v2/run.log", "");
    let missing = |written: &str, target: &str| {
        format!(
            "documentation_v2/guide.md:3: missing target: `{written}` resolves to \
             `documentation_v2/{target}`, which is no tracked file or folder"
        )
    };
    let backticked = |tracked: usize, naming_nothing: usize| {
        format!(
            "  backticked paths: 2 judged in live documents — {tracked} tracked, 1 ignored by \
             git, 0 exempt historical spelling(s), {naming_nothing} naming nothing, 0 unchecked; \
             0 pattern(s) skipped"
        )
    };
    let committed = run_listed_by_git(&fixture, UntrackedFiles::Invisible);
    assert_eq!(
        failures(&committed),
        [format!(
            "FAIL: documentation_v2/guide.md: 3 break(s)\n      {}\n      {}\n      \
             documentation_v2/guide.md:5: backticked path names nothing: \
             `documentation_v2/new_page.md`",
            missing("new_page.md#setup", "new_page.md"),
            missing("run.log", "run.log")
        )]
    );
    assert!(committed.totals.contains(&backticked(0, 1)));
    assert_eq!(committed.summary_label(), "link-check");
    assert_eq!(committed.print(), 1);

    let with_untracked = run_listed_by_git(&fixture, UntrackedFiles::Included);
    assert_eq!(
        failures(&with_untracked),
        [format!(
            "FAIL: documentation_v2/guide.md: 1 break(s)\n      {}",
            missing("run.log", "run.log")
        )],
        "the untracked page, its heading and its backticked path resolve; the ignored log stays \
         missing as a link and passes as a backticked path"
    );
    assert!(with_untracked.totals.contains(&backticked(1, 0)));
    assert_eq!(
        outcome_counts(&with_untracked),
        (1, 1, 0),
        "the untracked page is judged too"
    );
    assert_eq!(
        with_untracked.summary_label(),
        "link-check --with-untracked (untracked files included)"
    );
    assert_eq!(with_untracked.print(), 1);
}

#[test]
fn the_scope_narrows_the_judged_documents_and_an_empty_one_did_not_run() {
    let fixture = every_area("links-scope");
    let scoped = run(
        &fixture,
        &["documentation_v2/runbooks"],
        BreakListing::Every,
    );
    assert_eq!(outcome_counts(&scoped), (0, 1, 0));
    let refused = run(&fixture, &["apps/tool/src"], BreakListing::Every);
    assert_eq!(outcome_counts(&refused), (0, 0, 1));
    assert!(failures(&refused)[0].contains("names no tracked folder"));
    let unjudged = run(
        &fixture,
        &[&format!("{TICKETS_DIR}/nested")],
        BreakListing::Every,
    );
    assert_eq!(outcome_counts(&unjudged), (0, 0, 1));
    assert!(failures(&unjudged)[0].starts_with("FAIL: link-check judged nothing in"));
    assert_eq!(unjudged.print(), 2);
}

#[test]
fn an_unreadable_document_or_a_failed_listing_did_not_run() {
    let mut fixture = FixtureCheckout::new("links-unreadable");
    fixture
        .listed_only("documentation_v2/lost.md")
        .tracked("documentation_v2/kept.md", "# Kept\n");
    let unreadable = run(&fixture, &[], BreakListing::First);
    assert_eq!(outcome_counts(&unreadable), (1, 0, 1));
    assert!(unreadable.totals[0].ends_with("0 with breaks, 1 unreadable"));
    assert_eq!(unreadable.print(), 2);

    let failed = judge(
        Path::new("/nonexistent/documentation-gates"),
        Err(NotRun::ToolError {
            tool: "git ls-files -z".to_string(),
            status: 128,
            stderr: "fatal: not a git repository".to_string(),
        }),
        &GateRequest::default(),
        BreakListing::First,
        &mut [],
    );
    assert_eq!(
        failures(&failed)[0].lines().next(),
        Some("FAIL: link-check could not list the tracked files — git ls-files -z exited 128")
    );
    assert_eq!(failed.print(), 2);
}

#[test]
fn a_rule_that_skips_frozen_records_plugs_in_beside_the_link_rule() {
    let fixture = every_area("links-plug-in");
    let seen = RefCell::new(Vec::new());
    let mut rules: Vec<Box<dyn DocumentRule + '_>> = vec![
        Box::new(LinkTargets::new(&EmptyHistory)),
        Box::new(LiveDocumentsOnly { seen: &seen }),
    ];
    let run = judge(
        fixture.root(),
        Ok(fixture.tree()),
        &GateRequest::default(),
        BreakListing::Every,
        &mut rules,
    );
    assert_eq!(
        seen.borrow().len(),
        5,
        "every judged document but the frozen record"
    );
    assert!(
        !seen
            .borrow()
            .iter()
            .any(|path| path.starts_with(ARCHIVE_DIR))
    );
    assert!(
        run.totals
            .contains(&"  live-only rule: 5 document(s)".to_string())
    );
}

#[test]
fn the_verb_takes_the_report_flag_beside_the_documentation_gate_arguments() {
    let parsed = Cli::try_parse_from([
        "xtask",
        "verify",
        "link-check",
        "--report",
        "--path",
        "apps",
        "--with-untracked",
        "--path",
        "tools_v2",
    ])
    .expect("the verb parses");
    match parsed.cmd {
        TopCmd::Verify {
            cmd: VerifyCmd::LinkCheck { report, arguments },
        } => {
            assert!(report);
            assert_eq!(arguments.paths, ["apps", "tools_v2"]);
            assert!(arguments.with_untracked);
        }
        other => panic!("link-check parsed as {other:?}"),
    }
    let bare = Cli::try_parse_from(["xtask", "verify", "link-check"]).expect("no flags");
    assert!(matches!(
        bare.cmd,
        TopCmd::Verify {
            cmd: VerifyCmd::LinkCheck {
                report: false,
                arguments: DocumentationGateArgs { paths, with_untracked: false },
            },
        } if paths.is_empty()
    ));
}
