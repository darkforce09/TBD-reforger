//! Permalinks into this repository and the git objects they name.
//!
//! **Role:** reads a code view of this repository on GitHub — a blob view
//! `PERMALINK_BASE<commit>/<path>[#L<n>[-L<m>]]` or a tree view, `tree/` in place of `blob/` —
//! and looks the objects of every permalink up in the local history: one
//! `git cat-file --batch-check` for all of them, and one `git cat-file --batch` for the blobs
//! whose text a fragment needs.
//!
//! **Position:** [`super::target_resolution::classify`] calls [`read_code_view`]; the permalink
//! half of the link rule ([`super::permalink_targets`]) collects the permalinks of the whole run
//! and settles them through a [`PermalinkObjects`] source at the end, [`GitObjects`] in a real
//! run.
//!
//! **Signals & state:** none held; each lookup is one child process.
//!
//! **Invariants:** a permalink is a blob or tree view that names its commit by the full 40- or
//! 64-character id, so a view of a branch, a tag or an abbreviated id is not one, and a page of
//! the repository other than a blob or tree view is no code view at all; git missing, killed,
//! timed out, exiting non-zero or answering in an unexpected shape is a [`NotRun`] cause, never a
//! set of missing objects.

use std::path::Path;
use std::time::Duration;

use verification_core::NotRun;
use verification_core::proc::Run;

use super::target_resolution::{percent_decode, split_fragment};
use crate::core::repository_layout::documentation::PERMALINK_BASE;

/// The program that reads objects.
const GIT: &str = "git";

/// `git cat-file --batch-check`: one `<id> <type> <size>` or `<name> missing` line per name.
const LOOKUP_ARGUMENTS: [&str; 2] = ["cat-file", "--batch-check"];

/// `git cat-file --batch`: each object's header line, its content and a line break.
const CONTENTS_ARGUMENTS: [&str; 2] = ["cat-file", "--batch"];

/// A lookup that takes longer than this is killed: reading objects from a local repository takes
/// well under a second, so a stall is a broken git.
const LOOKUP_DEADLINE: Duration = Duration::from_secs(120);

/// The lengths of a full commit id: SHA-1 and SHA-256 repositories.
const COMMIT_ID_LENGTHS: [usize; 2] = [40, 64];

/// The GitHub view a code URL of this repository opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PermalinkView {
    /// `blob/`: a file, or a folder GitHub opens as its tree view.
    Blob,
    /// `tree/`: a folder; the commit alone names the repository root.
    Tree,
}

impl PermalinkView {
    /// Every view, in the order a URL is matched against them.
    const ALL: [PermalinkView; 2] = [PermalinkView::Blob, PermalinkView::Tree];

    /// The path segment between the repository and the commit that opens the view.
    fn segment(self) -> &'static str {
        match self {
            PermalinkView::Blob => "/blob/",
            PermalinkView::Tree => "/tree/",
        }
    }
}

/// A blob or tree view of this repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum CodeView {
    /// Pinned to a full commit id.
    Permalink(Permalink),
    /// Named by a branch, a tag or an abbreviated commit id, so what it opens moves with the name.
    Unpinned,
}

/// A sha permalink into this repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Permalink {
    pub(super) view: PermalinkView,
    /// The full commit id, lowercase.
    pub(super) commit: String,
    /// The repository-relative path, percent-decoded, without leading or trailing `/`; empty for
    /// the repository root.
    pub(super) path: String,
    /// Whether the query asks for the plain view (`?plain=1`).
    pub(super) plain_view: bool,
    pub(super) fragment: Option<String>,
}

impl Permalink {
    /// The `<commit>:<path>` name git resolves.
    pub(super) fn object_name(&self) -> String {
        format!("{}:{}", self.commit, self.path)
    }
}

/// A blob a lookup found.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct BlobObject {
    pub(super) id: String,
    /// The blob's size in bytes.
    pub(super) size: usize,
}

/// What a `<commit>:<path>` name resolves to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ObjectLookup {
    Blob(BlobObject),
    /// A folder.
    Tree,
    /// Another kind of object, such as the commit a submodule entry records.
    Other {
        kind: String,
    },
    /// Nothing in the local history, or more than one object.
    Unknown {
        answer: String,
    },
}

/// Where permalink objects are read from.
pub(super) trait PermalinkObjects {
    /// What each `<commit>:<path>` name resolves to, in order.
    fn lookup(&self, names: &[String]) -> Result<Vec<ObjectLookup>, NotRun>;
    /// The text of each blob, in order.
    fn contents(&self, blobs: &[BlobObject]) -> Result<Vec<String>, NotRun>;
}

/// The repository's own git history.
pub(super) struct GitObjects<'a> {
    repo_root: &'a Path,
}

impl<'a> GitObjects<'a> {
    pub(super) fn new(repo_root: &'a Path) -> GitObjects<'a> {
        GitObjects { repo_root }
    }

    /// Run one `git cat-file` batch with `input` on stdin; its stdout, or why there is none.
    fn batch(&self, arguments: [&str; 2], input: String) -> Result<String, NotRun> {
        let output = Run::new(GIT)
            .args(arguments)
            .cwd(self.repo_root)
            .stdin(input)
            .timeout(LOOKUP_DEADLINE)
            .output()?;
        if output.code != 0 {
            return Err(batch_problem(arguments, output.code, output.stderr.trim()));
        }
        Ok(output.stdout)
    }
}

impl PermalinkObjects for GitObjects<'_> {
    fn lookup(&self, names: &[String]) -> Result<Vec<ObjectLookup>, NotRun> {
        let stdout = self.batch(LOOKUP_ARGUMENTS, request(names.iter().map(String::as_str)))?;
        parse_lookup(&stdout, names.len())
            .map_err(|problem| batch_problem(LOOKUP_ARGUMENTS, 0, &problem))
    }

    fn contents(&self, blobs: &[BlobObject]) -> Result<Vec<String>, NotRun> {
        let stdout = self.batch(
            CONTENTS_ARGUMENTS,
            request(blobs.iter().map(|blob| blob.id.as_str())),
        )?;
        parse_contents(&stdout, blobs)
            .map_err(|problem| batch_problem(CONTENTS_ARGUMENTS, 0, &problem))
    }
}

/// Read a blob or tree view of this repository: `None` when `destination` is none — another
/// host, or another page of this repository such as its home page, an issue or a release.
///
/// The commit runs from the view segment to the next `/`, `?` or `#`; the path runs from there
/// to the query or the fragment, and an empty path names the repository root.
pub(super) fn read_code_view(destination: &str) -> Option<CodeView> {
    let location = strip_prefix_ignoring_case(destination, "https://")
        .or_else(|| strip_prefix_ignoring_case(destination, "http://"))
        .or_else(|| destination.strip_prefix("//"))?;
    let location = strip_prefix_ignoring_case(location, "www.").unwrap_or(location);
    let rest = strip_prefix_ignoring_case(location, repository_location())?;
    let (view, after_view) = PermalinkView::ALL
        .into_iter()
        .find_map(|view| rest.strip_prefix(view.segment()).map(|after| (view, after)))?;
    let commit_end = after_view.find(['/', '?', '#']).unwrap_or(after_view.len());
    let (commit, target) = after_view.split_at(commit_end);
    let full_id =
        COMMIT_ID_LENGTHS.contains(&commit.len()) && commit.chars().all(|c| c.is_ascii_hexdigit());
    if !full_id {
        return Some(CodeView::Unpinned);
    }
    let (before_fragment, fragment) = split_fragment(target);
    let (path, query) = before_fragment
        .split_once('?')
        .map_or((before_fragment, None), |(path, query)| (path, Some(query)));
    let path = percent_decode(path).unwrap_or_else(|| path.to_string());
    Some(CodeView::Permalink(Permalink {
        view,
        commit: commit.to_ascii_lowercase(),
        path: path.trim_matches('/').to_string(),
        plain_view: query.is_some_and(|query| query.split('&').any(|pair| pair == "plain=1")),
        fragment,
    }))
}

/// The host and path of this repository on GitHub: [`PERMALINK_BASE`] without its scheme and
/// blob view.
fn repository_location() -> &'static str {
    let without_scheme = PERMALINK_BASE
        .strip_prefix("https://")
        .unwrap_or(PERMALINK_BASE);
    without_scheme
        .strip_suffix(PermalinkView::Blob.segment())
        .unwrap_or(without_scheme)
}

/// `text` without `prefix`, compared without regard to ASCII case.
fn strip_prefix_ignoring_case<'t>(text: &'t str, prefix: &str) -> Option<&'t str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

/// One request line per item, each ended by a line break.
fn request<'i>(items: impl Iterator<Item = &'i str>) -> String {
    items.map(|item| format!("{item}\n")).collect()
}

/// The answers of `git cat-file --batch-check` to `expected` names.
pub(super) fn parse_lookup(stdout: &str, expected: usize) -> Result<Vec<ObjectLookup>, String> {
    let answers: Vec<&str> = stdout.lines().collect();
    if answers.len() != expected {
        return Err(format!(
            "{expected} name(s) sent, {} answer line(s) read",
            answers.len()
        ));
    }
    answers
        .into_iter()
        .map(|answer| {
            if answer.ends_with(" missing") || answer.ends_with(" ambiguous") {
                return Ok(ObjectLookup::Unknown {
                    answer: answer.rsplit(' ').next().unwrap_or(answer).to_string(),
                });
            }
            let fields: Vec<&str> = answer.split(' ').collect();
            match fields.as_slice() {
                [id, "blob", size] => size
                    .parse()
                    .map(|size| {
                        ObjectLookup::Blob(BlobObject {
                            id: (*id).to_string(),
                            size,
                        })
                    })
                    .map_err(|_| format!("unreadable blob size in `{answer}`")),
                [_, "tree", _] => Ok(ObjectLookup::Tree),
                [_, kind, _] => Ok(ObjectLookup::Other {
                    kind: (*kind).to_string(),
                }),
                _ => Err(format!("unreadable answer `{answer}`")),
            }
        })
        .collect()
}

/// The text of each blob in the output of `git cat-file --batch` for `blobs`.
///
/// Each answer is a header line, `size` bytes and a line break. The process runner decodes
/// output lossily, so a blob that is not UTF-8 decodes longer than its size; its end is then
/// where the next answer's known header begins.
pub(super) fn parse_contents(stdout: &str, blobs: &[BlobObject]) -> Result<Vec<String>, String> {
    let mut texts = Vec::with_capacity(blobs.len());
    let mut at = 0;
    for (index, blob) in blobs.iter().enumerate() {
        let header = format!("{} blob {}\n", blob.id, blob.size);
        if !stdout[at..].starts_with(&header) {
            return Err(format!("no answer where the one for {} belongs", blob.id));
        }
        let start = at + header.len();
        let next_header = blobs
            .get(index + 1)
            .map(|next| format!("\n{} blob {}\n", next.id, next.size));
        let end = content_end(stdout, start, blob.size, next_header.as_deref())
            .ok_or_else(|| format!("the answer for {} does not end where expected", blob.id))?;
        texts.push(stdout[start..end].to_string());
        at = end + 1;
    }
    Ok(texts)
}

/// Where the content that starts at `start` ends: after `size` bytes when the line break and the
/// next header follow there, otherwise at the next header, or before the final line break.
fn content_end(
    stdout: &str,
    start: usize,
    size: usize,
    next_header: Option<&str>,
) -> Option<usize> {
    let exact = start + size;
    let fits = stdout.is_char_boundary(exact)
        && match next_header {
            Some(next) => stdout[exact..].starts_with(next),
            None => &stdout[exact..] == "\n",
        };
    if fits {
        return Some(exact);
    }
    match next_header {
        Some(next) => stdout[start..].find(next).map(|offset| start + offset),
        None => stdout
            .strip_suffix('\n')
            .map(str::len)
            .filter(|end| *end >= start),
    }
}

/// The did-not-run cause for a batch that failed or answered in an unexpected shape.
fn batch_problem(arguments: [&str; 2], status: i32, problem: &str) -> NotRun {
    NotRun::ToolError {
        tool: format!("{GIT} {}", arguments.join(" ")),
        status,
        stderr: problem.to_string(),
    }
}

#[cfg(test)]
#[path = "tests/repository_permalinks.rs"]
mod tests;
