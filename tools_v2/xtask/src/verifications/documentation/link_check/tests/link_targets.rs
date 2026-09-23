use std::cell::RefCell;

use verification_core::NotRun;

use super::super::super::fixture_checkout::FixtureCheckout;
use super::super::Break;
use super::*;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// A permalink source with a fixed answer per name and per blob, recording every batch it serves.
#[derive(Default)]
struct FakeObjects {
    lookups: BTreeMap<String, ObjectLookup>,
    texts: BTreeMap<String, String>,
    fail_lookup: bool,
    fail_contents: bool,
    batches: RefCell<Vec<usize>>,
}

impl FakeObjects {
    fn blob(mut self, name: &str, id: &str, text: &str) -> FakeObjects {
        let blob = BlobObject {
            id: id.to_string(),
            size: text.len(),
        };
        self.lookups
            .insert(name.to_string(), ObjectLookup::Blob(blob));
        self.texts.insert(id.to_string(), text.to_string());
        self
    }

    fn tree(mut self, name: &str) -> FakeObjects {
        let tree = ObjectLookup::NotBlob {
            kind: "tree".to_string(),
        };
        self.lookups.insert(name.to_string(), tree);
        self
    }

    fn refusal() -> NotRun {
        NotRun::ToolError {
            tool: "git cat-file".to_string(),
            status: 128,
            stderr: "fatal".to_string(),
        }
    }
}

impl PermalinkObjects for FakeObjects {
    fn lookup(&self, names: &[String]) -> Result<Vec<ObjectLookup>, NotRun> {
        self.batches.borrow_mut().push(names.len());
        if self.fail_lookup {
            return Err(FakeObjects::refusal());
        }
        let unknown = ObjectLookup::Unknown {
            answer: "missing".to_string(),
        };
        Ok(names
            .iter()
            .map(|name| self.lookups.get(name).cloned().unwrap_or(unknown.clone()))
            .collect())
    }

    fn contents(&self, blobs: &[BlobObject]) -> Result<Vec<String>, NotRun> {
        self.batches.borrow_mut().push(blobs.len());
        if self.fail_contents {
            return Err(FakeObjects::refusal());
        }
        Ok(blobs
            .iter()
            .map(|blob| self.texts[&blob.id].clone())
            .collect())
    }
}

/// Judge `document` (written into `fixture` as tracked) with the link rule over `objects`, and
/// return every break as it renders plus how many checks did not run.
fn judge_document(
    fixture: &mut FixtureCheckout,
    document: &str,
    text: &str,
    objects: &FakeObjects,
) -> (Vec<String>, usize) {
    fixture.tracked(document, text);
    let tree = fixture.tree();
    let context = RuleContext {
        repo_root: fixture.root(),
        tree: &tree,
    };
    let scanned = scan(text);
    let mut rule = LinkTargets::new(objects);
    let mut findings = RuleFindings::default();
    rule.judge(
        &JudgedDocument {
            path: document,
            scan: &scanned,
        },
        &context,
        &mut findings,
    );
    rule.finish(&context, &mut findings);
    let breaks = findings.breaks.iter().map(Break::render).collect();
    (breaks, findings.not_run.len())
}

fn checkout(tag: &str) -> FixtureCheckout {
    let mut fixture = FixtureCheckout::new(&format!("link-targets-{tag}"));
    fixture
        .tracked(
            "documentation_v2/guide.md",
            "# Guide\n\n## Setup steps\n\nText.\n",
        )
        .tracked("apps/tool/src/main.rs", "fn main() {}\n// two\n// three\n")
        .tracked("apps/tool/README.md", "# Tool\n");
    fixture
}

#[test]
fn a_checkout_path_must_name_a_tracked_file_or_folder() {
    let mut fixture = checkout("paths");
    fixture.untracked("documentation_v2/draft.md", "# Draft\n");
    let text = "[ok](guide.md) [folder](/apps/tool/) [root](/)\n\
                [gone](missing.md) [draft](draft.md)\n[out](../../outside.md)\n";
    let (breaks, not_run) = judge_document(
        &mut fixture,
        "documentation_v2/index.md",
        text,
        &FakeObjects::default(),
    );
    assert_eq!(
        breaks,
        [
            "documentation_v2/index.md:2: missing target: `missing.md` resolves to \
             `documentation_v2/missing.md`, which is no tracked file or folder",
            "documentation_v2/index.md:2: missing target: `draft.md` resolves to \
             `documentation_v2/draft.md`, which is no tracked file or folder",
            "documentation_v2/index.md:3: escapes repository: `../../outside.md` climbs above the \
             repository root",
        ]
    );
    assert_eq!(not_run, 0);
}

#[test]
fn a_fragment_must_match_a_heading_or_fit_a_line_anchor() {
    let mut fixture = checkout("fragments");
    let text = "[a](guide.md#setup-steps) [b](guide.md#nope) [c](#local) [d](#elsewhere)\n\
                [e](/apps/tool/src/main.rs#L2-L3) [f](/apps/tool/src/main.rs#L4)\n\
                [g](/apps/tool/src/main.rs#main) [h](/apps/tool#x) [i](guide.md?plain=1#L5)\n\
                [j](guide.md#L1)\n\n## Local\n";
    let (breaks, _) = judge_document(
        &mut fixture,
        "documentation_v2/index.md",
        text,
        &FakeObjects::default(),
    );
    assert_eq!(
        breaks,
        [
            "documentation_v2/index.md:1: missing anchor: `guide.md#nope`: \
             `documentation_v2/guide.md` has no heading or anchor `nope`",
            "documentation_v2/index.md:1: missing anchor: `#elsewhere`: this document has no \
             heading or anchor `elsewhere`",
            "documentation_v2/index.md:2: line anchor out of range: \
             `/apps/tool/src/main.rs#L4`: the file has 3 line(s)",
            "documentation_v2/index.md:3: missing anchor: `/apps/tool/src/main.rs#main`: \
             `apps/tool/src/main.rs` is not rendered Markdown, so `main` matches nothing; only a \
             #L<n> or #L<n>-L<m> line anchor applies",
            "documentation_v2/index.md:3: missing anchor: `/apps/tool#x`: `apps/tool` is a \
             folder, which has no anchors",
            "documentation_v2/index.md:4: missing anchor: `guide.md#L1`: \
             `documentation_v2/guide.md` has no heading or anchor `L1`",
        ]
    );
}

#[test]
fn an_encoded_fragment_decodes_before_it_is_matched() {
    let mut fixture = checkout("encoded");
    let text = "# Überblick\n\n[a](#%C3%BCberblick) [b](#überblick)\n";
    let (breaks, _) = judge_document(
        &mut fixture,
        "documentation_v2/index.md",
        text,
        &FakeObjects::default(),
    );
    assert_eq!(breaks, Vec::<String>::new());
}

#[test]
fn an_undefined_reference_and_a_repository_page_break() {
    let mut fixture = checkout("references");
    let text = format!("[a][nowhere] [b]({}main/README.md)\n", PERMALINK_BASE);
    let (breaks, _) = judge_document(&mut fixture, "README.md", &text, &FakeObjects::default());
    assert_eq!(breaks.len(), 2);
    assert_eq!(
        breaks[0],
        "README.md:1: undefined reference: `[nowhere]` names no reference definition in this \
         document"
    );
    assert!(breaks[1].starts_with("README.md:1: non-permalink repository URL: `"));
}

#[test]
fn an_unreadable_target_did_not_run_once() {
    let mut fixture = checkout("unreadable");
    fixture.listed_only("documentation_v2/lost.md");
    let text = "[a](lost.md#one) [b](lost.md#two) [c](lost.md)\n";
    let (breaks, not_run) = judge_document(
        &mut fixture,
        "documentation_v2/index.md",
        text,
        &FakeObjects::default(),
    );
    assert_eq!(breaks, Vec::<String>::new());
    assert_eq!(not_run, 1, "one verdict for the target, not one per link");
}

#[test]
fn permalinks_are_settled_in_one_lookup_and_one_contents_batch() {
    let mut fixture = checkout("permalinks");
    let code = format!("{COMMIT}:apps/old/main.rs");
    let doc = format!("{COMMIT}:docs/old.md");
    let objects = FakeObjects::default()
        .blob(&code, "c0de", "one\ntwo\n")
        .blob(&doc, "d0c5", "# Old title\n")
        .tree(&format!("{COMMIT}:apps/old"));
    let base = PERMALINK_BASE;
    let text = format!(
        "[a]({base}{COMMIT}/apps/old/main.rs#L2) [b]({base}{COMMIT}/apps/old/main.rs#L3)\n\
         [c]({base}{COMMIT}/docs/old.md#old-title) [d]({base}{COMMIT}/docs/old.md#new-title)\n\
         [e]({base}{COMMIT}/apps/old) [f]({base}{COMMIT}/apps/old#L1)\n\
         [g]({base}{COMMIT}/apps/gone.rs) [h]({base}{COMMIT}/apps/old/main.rs)\n"
    );
    let (breaks, not_run) = judge_document(
        &mut fixture,
        "documentation_v2/archive/x.md",
        &text,
        &objects,
    );
    assert_eq!(not_run, 0);
    assert_eq!(
        *objects.batches.borrow(),
        [4, 2],
        "4 distinct names, 2 blobs read"
    );
    let rules: Vec<(&str, &str)> = breaks
        .iter()
        .map(|found| {
            let mut parts = found.splitn(4, ": ");
            (parts.next().unwrap_or(""), parts.next().unwrap_or(""))
        })
        .collect();
    assert_eq!(
        rules,
        [
            (
                "documentation_v2/archive/x.md:1",
                "line anchor out of range"
            ),
            ("documentation_v2/archive/x.md:2", "missing anchor"),
            ("documentation_v2/archive/x.md:3", "missing anchor"),
            ("documentation_v2/archive/x.md:4", "unknown permalink blob"),
        ]
    );
    assert!(breaks[3].ends_with(&format!(
        "`{COMMIT}:apps/gone.rs` is missing in the local history"
    )));
}

#[test]
fn a_failed_permalink_batch_did_not_run_and_breaks_nothing() {
    let mut fixture = checkout("failed-batch");
    let text = format!("[a]({PERMALINK_BASE}{COMMIT}/x.rs#L1)\n");
    let failing_lookup = FakeObjects {
        fail_lookup: true,
        ..FakeObjects::default()
    };
    let (breaks, not_run) = judge_document(&mut fixture, "README.md", &text, &failing_lookup);
    assert_eq!((breaks.len(), not_run), (0, 1));
    let failing_contents = FakeObjects {
        fail_contents: true,
        ..FakeObjects::default().blob(&format!("{COMMIT}:x.rs"), "b10b", "x\n")
    };
    let (breaks, not_run) = judge_document(&mut fixture, "README.md", &text, &failing_contents);
    assert_eq!((breaks.len(), not_run), (0, 1));
}

#[test]
fn external_links_are_counted_and_never_fetched() {
    let mut fixture = checkout("external");
    let text = "[a](https://example.com) <mailto:x@example.com> [b](guide.md)\n";
    fixture.tracked("documentation_v2/index.md", text);
    let tree = fixture.tree();
    let context = RuleContext {
        repo_root: fixture.root(),
        tree: &tree,
    };
    let objects = FakeObjects::default();
    let mut rule = LinkTargets::new(&objects);
    let mut findings = RuleFindings::default();
    let scanned = scan(text);
    let document = JudgedDocument {
        path: "documentation_v2/index.md",
        scan: &scanned,
    };
    rule.judge(&document, &context, &mut findings);
    rule.finish(&context, &mut findings);
    assert!(findings.breaks.is_empty());
    assert!(objects.batches.borrow().is_empty(), "no permalink, no git");
    assert_eq!(
        rule.totals(),
        [
            "  links: 3 judged — 1 into this checkout, 0 permalink(s), 0 other page(s) of this \
          repository, 2 external and not fetched"
        ]
    );
}
