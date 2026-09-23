//! Markdown placement: documents live in the documentation tree, and live documents stay short.
//!
//! **Role:** the `markdown-placement` gate. Over the tracked files the scope selects it judges
//! three rules: the code trees hold no Markdown file but README.md (test, generated-output and
//! hidden folders excepted); the retired documentation root holds no tracked file; and every live
//! Markdown document under the documentation root is at most [`LIVE_DOCUMENT_LINE_LIMIT`] lines.
//!
//! **Position:** `cargo xtask verify markdown-placement [--path <dir>]...` calls
//! [`verify_markdown_placement`]; every region comes from [`super::path_regions`].
//!
//! **Signals & state:** none; one pass over the tracked tree per run.
//!
//! **Invariants:** only tracked files are judged; the frozen ticket records and archives, the
//! pending merge sources, the program records at the documentation root and the sync-managed
//! documents are outside the size limit; a document that cannot be read is "did not run", never a
//! pass.

use std::path::Path;

use verification_core::{Finding, Kind, NotRun, Verdict};

use super::gate_scope::GateScope;
use super::path_regions::{
    README, below_exempt_folder, file_name, in_code_tree, in_documentation_root, is_markdown,
    is_size_exempt, is_within, parent_folder,
};
use super::tracked_tree::TrackedTree;
use super::{GateRun, Tally, judged_nothing, prepare, read_tracked, scope_line};
use crate::core::repository_layout::documentation::{DOCUMENTATION_ROOT, RETIRED_DOCS_ROOT};

/// The gate's name on its header and summary lines.
const GATE: &str = "markdown-placement";

/// The most lines a live document may hold; a longer one splits by topic into a folder with a
/// README index.
const LIVE_DOCUMENT_LINE_LIMIT: usize = 500;

/// `cargo xtask verify markdown-placement`: judge the tracked tree at `repo_root` over the
/// `--path` scope, print every verdict, and return the exit status (0 held, 1 violation, 2 did
/// not run).
pub(crate) fn verify_markdown_placement(repo_root: &Path, scope: &[String]) -> u8 {
    judge(repo_root, TrackedTree::load(repo_root), scope).print()
}

/// The gate over an already-attempted listing, so a failed listing and a fixture tree take the
/// same path as a real run.
fn judge(
    repo_root: &Path,
    listing: Result<TrackedTree, NotRun>,
    scope_values: &[String],
) -> GateRun {
    let (tree, scope) = match prepare(GATE, Kind::Ban, repo_root, listing, scope_values) {
        Ok(prepared) => prepared,
        Err(stopped) => return stopped,
    };
    let mut run = GateRun::new(
        GATE,
        vec![
            format!(
                "==> {GATE}: code trees hold only {README}, {RETIRED_DOCS_ROOT}/ holds nothing, \
                 live documents stay at or under {LIVE_DOCUMENT_LINE_LIMIT} lines"
            ),
            scope_line(&scope, &tree),
        ],
    );
    let code_trees = judge_code_trees(&tree, &scope, &mut run.verdicts);
    let retired = judge_retired_root(&tree, &scope, &mut run.verdicts);
    let documents = judge_documents(repo_root, &tree, &scope, &mut run.verdicts);
    if code_trees.judged + documents.judged == 0 && retired.is_none() {
        run.verdicts
            .push(judged_nothing(GATE, Kind::Ban, repo_root, &scope));
        return run;
    }
    run.totals = vec![
        format!(
            "  code trees: {} Markdown file(s) judged, {} other than {README}",
            code_trees.judged, code_trees.failed
        ),
        match retired {
            Some(held) => format!("  {RETIRED_DOCS_ROOT}/: {held} tracked file(s)"),
            None => format!("  {RETIRED_DOCS_ROOT}/: outside the scope"),
        },
        format!(
            "  {DOCUMENTATION_ROOT}/: {} live document(s) judged, {} over \
             {LIVE_DOCUMENT_LINE_LIMIT} lines, {} unreadable",
            documents.judged, documents.failed, documents.unread
        ),
    ];
    run
}

/// Rule 1: every Markdown file in a code tree, outside exempt folders, is a README.md.
fn judge_code_trees(tree: &TrackedTree, scope: &GateScope, verdicts: &mut Vec<Verdict>) -> Tally {
    let mut tally = Tally::default();
    for path in tree.files().filter(|path| {
        scope.contains(path)
            && in_code_tree(path)
            && is_markdown(path)
            && !below_exempt_folder(parent_folder(path))
    }) {
        let verdict = if file_name(path) == README {
            Verdict::Held
        } else {
            Verdict::failed(format!(
                "{path}: Markdown in a code tree; a code tree holds only {README}, and documents \
                 live under {DOCUMENTATION_ROOT}/"
            ))
        };
        tally.count(&verdict);
        verdicts.push(verdict);
    }
    tally
}

/// Rule 2: the retired documentation root holds no tracked file. `None` when the scope does not
/// reach it; otherwise how many tracked files it holds.
fn judge_retired_root(
    tree: &TrackedTree,
    scope: &GateScope,
    verdicts: &mut Vec<Verdict>,
) -> Option<usize> {
    if !scope.overlaps(RETIRED_DOCS_ROOT) {
        return None;
    }
    let held: Vec<&str> = tree
        .files()
        .filter(|path| is_within(path, RETIRED_DOCS_ROOT) && scope.contains(path))
        .collect();
    verdicts.push(if held.is_empty() {
        Verdict::Held
    } else {
        Verdict::Failed(Finding {
            headline: format!(
                "{RETIRED_DOCS_ROOT}/ holds {} tracked file(s); every document lives under \
                 {DOCUMENTATION_ROOT}/",
                held.len()
            ),
            detail: held.iter().map(ToString::to_string).collect(),
        })
    });
    Some(held.len())
}

/// Rule 3: every live Markdown document under the documentation root is at most
/// [`LIVE_DOCUMENT_LINE_LIMIT`] lines.
fn judge_documents(
    repo_root: &Path,
    tree: &TrackedTree,
    scope: &GateScope,
    verdicts: &mut Vec<Verdict>,
) -> Tally {
    let mut tally = Tally::default();
    for path in tree.files().filter(|path| {
        scope.contains(path)
            && in_documentation_root(path)
            && is_markdown(path)
            && !is_size_exempt(path)
    }) {
        let verdict = match read_tracked(repo_root, path) {
            Err(cause) => {
                Verdict::did_not_run(format!("{path} could not be read"), Kind::Ban, cause)
            }
            Ok(text) => match text.lines().count() {
                lines if lines <= LIVE_DOCUMENT_LINE_LIMIT => Verdict::Held,
                lines => Verdict::failed(format!(
                    "{path}: {lines} lines; a live document stays at or under \
                     {LIVE_DOCUMENT_LINE_LIMIT}, so split it by topic into a folder with a \
                     {README} index"
                )),
            },
        };
        tally.count(&verdict);
        verdicts.push(verdict);
    }
    tally
}

#[cfg(test)]
#[path = "tests/markdown_placement.rs"]
mod tests;
