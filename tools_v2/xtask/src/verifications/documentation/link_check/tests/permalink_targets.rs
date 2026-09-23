use std::cell::Cell;

use verification_core::NotRun;

use super::*;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// A permalink of `view` into `path` at [`COMMIT`], written at line 1 of the root README.
fn waiting(view: PermalinkView, path: &str, fragment: Option<&str>) -> PendingPermalink {
    PendingPermalink {
        document: "README.md".to_string(),
        line: 1,
        destination: format!("link-to-{path}"),
        permalink: Permalink {
            view,
            commit: COMMIT.to_string(),
            path: path.to_string(),
            plain_view: false,
            fragment: fragment.map(str::to_string),
        },
    }
}

fn file(id: &str) -> ObjectLookup {
    ObjectLookup::Blob(BlobObject {
        id: id.to_string(),
        size: 0,
    })
}

fn missing() -> ObjectLookup {
    ObjectLookup::Unknown {
        answer: "missing".to_string(),
    }
}

/// The rule a settled permalink breaks, or `None` when it holds.
fn rule_of(
    view: PermalinkView,
    path: &str,
    fragment: Option<&str>,
    lookup: &ObjectLookup,
    texts: &BTreeMap<String, String>,
) -> Option<BreakRule> {
    judge_permalink(&waiting(view, path, fragment), lookup, texts).map(|(rule, _)| rule)
}

#[test]
fn a_tree_view_holds_only_on_a_folder_without_a_fragment() {
    let none = BTreeMap::new();
    let tree = PermalinkView::Tree;
    let folder = ObjectLookup::Tree;
    let submodule = ObjectLookup::Other {
        kind: "commit".to_string(),
    };
    assert_eq!(rule_of(tree, "apps", None, &folder, &none), None);
    assert_eq!(rule_of(tree, "", None, &folder, &none), None, "the root");
    for fragment in ["L1", "readme"] {
        assert_eq!(
            rule_of(tree, "apps", Some(fragment), &folder, &none),
            Some(BreakRule::MissingAnchor),
            "{fragment}"
        );
    }
    for (path, lookup) in [
        ("a.md", file("b10b")),
        ("vendor/lib", submodule),
        ("gone", missing()),
    ] {
        assert_eq!(
            rule_of(tree, path, None, &lookup, &none),
            Some(BreakRule::UnknownPermalinkObject),
            "{path}"
        );
    }
}

#[test]
fn a_tree_view_of_a_file_names_the_object_and_the_view() {
    let found = judge_permalink(
        &waiting(PermalinkView::Tree, "a.md", None),
        &file("b10b"),
        &BTreeMap::new(),
    );
    assert_eq!(
        found,
        Some((
            BreakRule::UnknownPermalinkObject,
            format!(
                "`link-to-a.md`: `{COMMIT}:a.md` is a file in the local history, and a tree view \
                 opens a folder"
            )
        ))
    );
}

#[test]
fn a_blob_view_holds_on_a_file_or_a_folder_and_judges_its_fragment_against_the_file() {
    let texts = BTreeMap::from([("b10b".to_string(), "# Title\n\nline\n".to_string())]);
    let blob = PermalinkView::Blob;
    let text = file("b10b");
    assert_eq!(rule_of(blob, "a.md", None, &text, &texts), None);
    assert_eq!(
        rule_of(blob, "apps", None, &ObjectLookup::Tree, &texts),
        None,
        "GitHub opens a folder's blob view as its tree view"
    );
    assert_eq!(rule_of(blob, "a.md", Some("title"), &text, &texts), None);
    assert_eq!(rule_of(blob, "a.txt", Some("L3"), &text, &texts), None);
    for (path, fragment, lookup, rule) in [
        ("a.md", "other", &text, BreakRule::MissingAnchor),
        ("a.txt", "L4", &text, BreakRule::LineAnchorOutOfRange),
        ("a.txt", "top", &text, BreakRule::MissingAnchor),
        ("apps", "L1", &ObjectLookup::Tree, BreakRule::MissingAnchor),
        (
            "gone.md",
            "top",
            &missing(),
            BreakRule::UnknownPermalinkObject,
        ),
    ] {
        assert_eq!(
            rule_of(blob, path, Some(fragment), lookup, &texts),
            Some(rule),
            "{path}#{fragment}"
        );
    }
    assert_eq!(
        rule_of(blob, "a.md", Some("other"), &text, &BTreeMap::new()),
        None,
        "a blob whose text was not read makes no break; its batch did not run"
    );
}

/// A permalink source that answers every name as one small file, counting the batches it serves.
#[derive(Default)]
struct EveryNameAFile {
    lookups: Cell<usize>,
    reads: Cell<usize>,
}

impl PermalinkObjects for EveryNameAFile {
    fn lookup(&self, names: &[String]) -> Result<Vec<ObjectLookup>, NotRun> {
        self.lookups.set(self.lookups.get() + 1);
        Ok(names.iter().map(|_| file("b10b")).collect())
    }

    fn contents(&self, blobs: &[BlobObject]) -> Result<Vec<String>, NotRun> {
        self.reads.set(self.reads.get() + 1);
        Ok(blobs.iter().map(|_| "x\n".to_string()).collect())
    }
}

#[test]
fn settling_runs_no_git_without_permalinks_and_reads_no_text_for_a_tree_view() {
    let objects = EveryNameAFile::default();
    let mut findings = RuleFindings::default();
    PendingPermalinks::default().settle(&objects, &mut findings);
    assert_eq!((objects.lookups.get(), objects.reads.get()), (0, 0));

    let mut pending = PendingPermalinks::default();
    let anchored_tree = waiting(PermalinkView::Tree, "a.rs", Some("L1"));
    pending.hold(
        &anchored_tree.document,
        anchored_tree.line,
        &anchored_tree.destination,
        anchored_tree.permalink,
    );
    pending.settle(&objects, &mut findings);
    assert_eq!(
        (objects.lookups.get(), objects.reads.get()),
        (1, 0),
        "a tree view's fragment reads no blob"
    );
    let rules: Vec<BreakRule> = findings.breaks.iter().map(|found| found.rule).collect();
    assert_eq!(rules, [BreakRule::UnknownPermalinkObject]);
}
