use super::repository_permalinks::{BlobObject, ObjectLookup, PermalinkObjects};
use super::*;
use crate::UntrackedFiles;
use crate::fixture_checkout::{FixtureCheckout, failures, outcome_counts, request};
use repository_layout::PROJECT_INSTRUCTIONS;

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

/// Every judged area holds one document with one broken link; files outside the judged set hold
/// broken links that must never be reported.
fn every_area(tag: &str) -> FixtureCheckout {
    let mut fixture = FixtureCheckout::new(tag);
    let broken = "# Doc\n\n[gone](nowhere.md)\n";
    fixture
        .tracked("documentation/runbooks/deploy.md", broken)
        .tracked(PROJECT_INSTRUCTIONS, broken)
        .tracked("apps/tool/README.md", broken)
        .tracked("apps/tool/NOTES.md", broken)
        .tracked("apps/notes/nested/deep.md", broken)
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
            format!("FAIL: {PROJECT_INSTRUCTIONS}: 1 break(s)"),
            "FAIL: apps/tool/README.md: 1 break(s)".to_string(),
            "FAIL: documentation/runbooks/deploy.md: 1 break(s)".to_string(),
        ]
    );
    assert_eq!(outcome_counts(&run), (0, 3, 0));
    assert_eq!(run.print(), 1);
}

#[test]
fn every_break_prints_as_path_line_rule_message() {
    let mut fixture = FixtureCheckout::new("links-report");
    fixture.tracked(
        "documentation/guide.md",
        "# Guide\n\n[a](gone.md) [b](#nowhere)\n\n[c][undefined]\n",
    );
    let run = run(&fixture, &[], BreakListing::Every);
    assert_eq!(
        failures(&run),
        ["FAIL: documentation/guide.md: 3 break(s)\n      \
          documentation/guide.md:3: missing target: `gone.md` resolves to \
          `documentation/gone.md`, which is no tracked file or folder\n      \
          documentation/guide.md:3: missing anchor: `#nowhere`: this document has no heading \
          or anchor `nowhere`\n      \
          documentation/guide.md:5: undefined reference: `[undefined]` names no reference \
          definition in this document"]
    );
}

#[test]
fn a_clean_tree_holds() {
    let mut fixture = FixtureCheckout::new("links-clean");
    fixture
        .tracked("documentation/guide.md", "# Guide\n\n## Setup\n")
        .tracked(
            "README.md",
            "[guide](documentation/guide.md#setup) [web](https://example.com)\n",
        );
    let run = run(&fixture, &[], BreakListing::First);
    assert_eq!(outcome_counts(&run), (2, 0, 0));
    assert_eq!(run.print(), 0);
}

#[test]
fn the_scope_narrows_the_judged_documents_and_an_empty_one_did_not_run() {
    let fixture = every_area("links-scope");
    let scoped = run(&fixture, &["documentation/runbooks"], BreakListing::Every);
    assert_eq!(outcome_counts(&scoped), (0, 1, 0));
    let refused = run(&fixture, &["apps/tool/src"], BreakListing::Every);
    assert_eq!(outcome_counts(&refused), (0, 0, 1));
    assert!(failures(&refused)[0].contains("names no tracked folder"));
    let unjudged = run(&fixture, &["apps/notes/nested"], BreakListing::Every);
    assert_eq!(outcome_counts(&unjudged), (0, 0, 1));
    assert!(failures(&unjudged)[0].starts_with("FAIL: link-check judged nothing in"));
    assert_eq!(unjudged.print(), 2);
}

#[test]
fn an_unreadable_document_or_a_failed_listing_did_not_run() {
    let mut fixture = FixtureCheckout::new("links-unreadable");
    fixture
        .listed_only("documentation/lost.md")
        .tracked("documentation/kept.md", "# Kept\n");
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
