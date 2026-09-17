//! Walking a tree and reporting the offending LINES.
//!
//! [`crate::gate`] answers "does this pattern appear in these files" — the whole-file boolean that
//! `gate_ban`/`gate_require` gave. Plenty of gates need the other shape: `grep -rn` over a
//! directory, printing every hit with its path and line number so the operator can go and fix
//! them. `verify-no-select-star.sh`, `verify-route-tags.sh` and most of the `verify-t*.sh` family
//! are all that shape.
//!
//! ── FAIL-CLOSED WALKING ──────────────────────────────────────────────────────────────────────
//!
//! The bash idiom for this is
//!
//! ```text
//! done < <(grep -rnE 'PATTERN' "$ROOT/src/handlers" "$ROOT/src/services" 2>/dev/null || true)
//! ```
//!
//! — taken verbatim from `verify-no-select-star.sh`, and it contains the defect twice over.
//! `2>/dev/null` hides "no such directory" and `|| true` converts the resulting failure into an
//! empty result set, which the loop below it reads as *zero violations*. Rename `src/handlers` and
//! that gate prints `no-select-star: clean` forever.
//!
//! So [`walk_files`] treats a missing or unreadable root as [`NotRun::TargetMissing`], and an
//! unreadable file as [`NotRun::Unreadable`]. There is no "skip it quietly" path.
//!
//! No `walkdir` dependency: the recursion is a dozen lines of `std::fs` and this crate is linked
//! by everything, so its dep list stays short on purpose.

use std::path::{Path, PathBuf};

use crate::pattern::Pattern;
use crate::verdict::NotRun;

/// One matching line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub path: PathBuf,
    /// 1-based, as `grep -n` reports it.
    pub line_no: usize,
    pub line: String,
}

impl Hit {
    /// `path:line:text`, the shape `grep -rn` prints and every consumer already parses.
    pub fn rendered(&self) -> String {
        format!("{}:{}:{}", self.path.display(), self.line_no, self.line)
    }
}

/// Recursively collect files under `roots`, keeping those `keep` accepts.
///
/// A root that does not exist is a failure, not an empty result — see the module docs.
pub fn walk_files(roots: &[&Path], keep: impl Fn(&Path) -> bool) -> Result<Vec<PathBuf>, NotRun> {
    let mut out = Vec::new();
    for root in roots {
        if !root.exists() {
            return Err(NotRun::TargetMissing(root.to_path_buf()));
        }
        collect(root, &keep, &mut out)?;
    }
    // Deterministic order: a gate's output must not depend on readdir ordering, or two runs over
    // the same tree disagree and the diff-based port acceptance becomes meaningless.
    out.sort();
    Ok(out)
}

fn collect(
    dir: &Path,
    keep: &impl Fn(&Path) -> bool,
    out: &mut Vec<PathBuf>,
) -> Result<(), NotRun> {
    if dir.is_file() {
        if keep(dir) {
            out.push(dir.to_path_buf());
        }
        return Ok(());
    }
    let entries = std::fs::read_dir(dir).map_err(|source| NotRun::Unreadable {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| NotRun::Unreadable {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let ty = entry.file_type().map_err(|source| NotRun::Unreadable {
            path: path.clone(),
            source,
        })?;
        if ty.is_dir() {
            collect(&path, keep, out)?;
        } else if ty.is_file() && keep(&path) {
            out.push(path);
        }
        // Symlinks are deliberately not followed: a link out of the tree would let a gate report
        // on files that are not in this repository, and a cycle would hang it.
    }
    Ok(())
}

/// Every line in `files` matching `pattern`, in file then line order.
pub fn grep_lines(pattern: &Pattern, files: &[PathBuf]) -> Result<Vec<Hit>, NotRun> {
    let mut hits = Vec::new();
    for path in files {
        let text = match std::fs::read(path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(source) => {
                return Err(NotRun::Unreadable {
                    path: path.clone(),
                    source,
                });
            }
        };
        for (i, line) in text.lines().enumerate() {
            if pattern.is_match(line) {
                hits.push(Hit {
                    path: path.clone(),
                    line_no: i + 1,
                    line: line.to_string(),
                });
            }
        }
    }
    Ok(hits)
}

/// `keep` predicate for a set of file extensions.
pub fn with_extension(exts: &'static [&'static str]) -> impl Fn(&Path) -> bool {
    move |p: &Path| {
        p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| exts.contains(&e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TmpDir(PathBuf);
    impl TmpDir {
        fn new(name: &str) -> TmpDir {
            let mut p = std::env::temp_dir();
            p.push(format!("tbd-gate-scan-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            TmpDir(p)
        }
        fn file(&self, rel: &str, body: &str) -> PathBuf {
            let p = self.0.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, body).unwrap();
            p
        }
    }
    impl Drop for TmpDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_missing_root_is_did_not_run_not_zero_hits() {
        // THE DEFECT. `grep -rn ... 2>/dev/null || true` reads a renamed directory as "clean".
        let got = walk_files(&[Path::new("/nonexistent/tbd-gate/scan")], |_| true);
        assert!(matches!(got, Err(NotRun::TargetMissing(_))));
    }

    #[test]
    fn walks_recursively_and_deterministically() {
        let d = TmpDir::new("walk");
        d.file("a.rs", "");
        d.file("sub/b.rs", "");
        d.file("sub/deep/c.rs", "");
        let files = walk_files(&[&d.0], |_| true).unwrap();
        assert_eq!(files.len(), 3);
        let mut sorted = files.clone();
        sorted.sort();
        assert_eq!(files, sorted, "order must not depend on readdir");
    }

    #[test]
    fn extension_filter_applies() {
        let d = TmpDir::new("ext");
        d.file("keep.rs", "");
        d.file("drop.txt", "");
        let files = walk_files(&[&d.0], with_extension(&["rs"])).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("keep.rs"));
    }

    #[test]
    fn a_file_root_is_accepted_directly() {
        let d = TmpDir::new("fileroot");
        let f = d.file("solo.rs", "");
        assert_eq!(walk_files(&[&f], |_| true).unwrap(), vec![f.clone()]);
    }

    #[test]
    fn grep_lines_reports_one_based_line_numbers() {
        let d = TmpDir::new("grep");
        let f = d.file("x.rs", "first\nSELECT * FROM users\nthird\n");
        let hits = grep_lines(
            &Pattern::regex("SELECT \\* FROM").unwrap(),
            std::slice::from_ref(&f),
        )
        .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line_no, 2, "grep -n is 1-based");
        assert_eq!(hits[0].line, "SELECT * FROM users");
        assert!(hits[0].rendered().ends_with(":2:SELECT * FROM users"));
    }

    #[test]
    fn grep_lines_finds_every_occurrence() {
        let d = TmpDir::new("multi");
        let f = d.file("y.rs", "hit\nmiss\nhit\n");
        let hits = grep_lines(&Pattern::literal("hit"), &[f]).unwrap();
        assert_eq!(
            hits.iter().map(|h| h.line_no).collect::<Vec<_>>(),
            vec![1, 3]
        );
    }

    #[test]
    fn grep_lines_on_a_missing_file_is_did_not_run() {
        let got = grep_lines(
            &Pattern::literal("x"),
            &[PathBuf::from("/nonexistent/tbd/z.rs")],
        );
        assert!(matches!(got, Err(NotRun::Unreadable { .. })));
    }

    #[test]
    fn non_utf8_bytes_do_not_abort_the_scan() {
        // A stray latin-1 byte in a source file must not make the gate unable to run.
        let d = TmpDir::new("binary");
        let p = d.0.join("odd.rs");
        std::fs::write(&p, [b'h', b'i', 0xff, b'\n', b'x', b'\n']).unwrap();
        let hits = grep_lines(&Pattern::literal("x"), &[p]).unwrap();
        assert_eq!(hits.len(), 1);
    }
}
