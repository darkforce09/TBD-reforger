//! Role: the engine-layer walls' shared machinery — rendering, walking, probing, and judging
//! hits against an enumerated pin.
//! Position: `xtask` — the helper half of [`super`], which owns the rules themselves.
//! Signals & state: none; every function is pure over what it is handed.
//! Invariants: a matcher is proved over subjects whose answers are known BEFORE it judges
//! anything, and a check that could not run exits differently from one that found a breach.

use std::path::{Path, PathBuf};

use tbd_gate::scan::Hit;
use tbd_gate::{Kind, NotRun, Pattern, Verdict, gate};

use super::rules::PROBE_FAIL;

/// Push a fixed block. `""` is a deliberate blank line, which `str::lines` would swallow.
pub(super) fn say(o: &mut Vec<String>, lines: &[&str]) {
    o.extend(lines.iter().map(|s| (*s).to_string()));
}

/// `path:line:text` — [`Hit::rendered`]'s shape, but repo-relative.
///
/// `walk_files` is handed absolute roots, so `rendered()` would print this machine's checkout
/// prefix. Every path in this gate's output names a file a reader has to go and edit, and the
/// relative form is the one that pastes into an editor and into a test assertion unchanged.
pub(super) fn rel(repo_root: &Path, hit: &Hit) -> String {
    let p = hit.path.strip_prefix(repo_root).unwrap_or(&hit.path);
    format!("{}:{}:{}", p.display(), hit.line_no, hit.line)
}

/// Build output is not source.
///
/// A stray `apps/website/graphics-engine/src/target-container/` (536 MB of wasm artifacts, one
/// generated `thiserror` `private.rs` among them) exists in this checkout today, and a gate that
/// reported on a dependency's generated code would be reporting on code nobody in this repo wrote.
/// `.gitignore` covers `/target/` and `target-*/` anywhere in the tree, so a pruned directory can
/// never hold a tracked file — the prune removes noise, not coverage. Only components *below*
/// `repo_root` are considered, or a checkout that happened to live under `~/target-x` would prune
/// the entire repository and the gate would go vacuously green.
pub(super) fn is_source(repo_root: &Path, path: &Path) -> bool {
    let rel = path.strip_prefix(repo_root).unwrap_or(path);
    !rel.components().any(|c| {
        let n = c.as_os_str().to_string_lossy();
        n == "target" || n.starts_with("target-")
    })
}

/// Group `hits` by repo-relative file and judge them against an enumerated pin.
///
/// Returns `(findings, lines)` — findings is what a reader has to go and fix, lines is how tall
/// the report of it is, and those are different numbers because one finding prints many lines.
///
/// Rules 3a and 3b are the same judgement over two different matchers, so they are the same
/// code. Three ways to fail and all three are load-bearing:
///
/// * an **unpinned** file matched at all — the rule's actual subject;
/// * a **pinned** file whose count moved in either direction — a pin that no longer describes
///   the tree has quietly stopped meaning what it says, whether it grew or shrank;
/// * a pinned file that **no longer matches at all**, which never reaches the first loop because
///   it is not in `per_file` — so the pin is also checked from its own side. This is the arm that
///   catches a renamed or deleted directory, i.e. the case where "no violations" and "nothing
///   left to look at" would otherwise be indistinguishable.
pub(super) fn against_pin(
    repo_root: &Path,
    hits: &[Hit],
    pin: &[(&str, usize, &str)],
) -> (usize, Vec<String>) {
    let mut per_file: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for h in hits {
        let p = h.path.strip_prefix(repo_root).unwrap_or(&h.path);
        per_file
            .entry(p.display().to_string())
            .or_default()
            .push(rel(repo_root, h));
    }
    let mut bad: Vec<String> = Vec::new();
    let mut findings = 0usize;
    for (file, lines) in &per_file {
        match pin.iter().find(|(f, _, _)| f == file) {
            None => {
                findings += 1;
                bad.push(format!("  unpinned file — {} site(s):", lines.len()));
                bad.extend(lines.iter().map(|l| format!("    {l}")));
            }
            Some((_, want, _)) if *want != lines.len() => {
                findings += 1;
                bad.push(format!(
                    "  {file}: pinned at {want} site(s), found {} — update the pin.",
                    lines.len()
                ));
                bad.extend(lines.iter().map(|l| format!("    {l}")));
            }
            Some(_) => {}
        }
    }
    for (file, want, _) in pin {
        if !per_file.contains_key(*file) {
            findings += 1;
            bad.push(format!(
                "  {file}: pinned at {want} site(s), found 0 — the pin is stale, delete the row."
            ));
        }
    }
    (findings, bad)
}

/// The subset of `files` sitting under one repo-relative directory.
///
/// [`Path::starts_with`] compares whole components, so `src/data` does not capture a sibling
/// `src/database` — the same property the `\b` gives the matchers, applied to the walk.
pub(super) fn under(repo_root: &Path, files: &[PathBuf], rel: &str) -> Vec<PathBuf> {
    let root = repo_root.join(rel);
    files
        .iter()
        .filter(|p| p.starts_with(&root))
        .cloned()
        .collect()
}

/// Compile a matcher and prove it over subjects whose answers are known, before it judges
/// anything.
///
/// Both lists are load-bearing and the second one more than the first. A matcher that fails to
/// fire is a gate that passes vacuously; a matcher that fires on the spelling every call site is
/// *supposed* to use is a rule nobody can satisfy, and an unsatisfiable rule gets deleted. Rules
/// 1, 2, 3a and 3b spell their probes out inline because they predate this helper and their
/// refusal strings differ; the three added with rules 4 and 7 share one shape, so they share one
/// function rather than three more copies of the same nine-line match.
pub(super) fn probed(
    o: &mut Vec<String>,
    what: &str,
    re: &str,
    must: &[&str],
    must_not: &[&str],
) -> Result<Pattern, (u8, Vec<String>)> {
    let p = match Pattern::regex(re) {
        Ok(p) => p,
        Err(e) => {
            let cause = NotRun::ToolError {
                tool: "regex".into(),
                status: 2,
                stderr: e.to_string(),
            };
            return Err(refuse(o, what, cause));
        }
    };
    for (subject, want) in must
        .iter()
        .map(|s| (s, true))
        .chain(must_not.iter().map(|s| (s, false)))
    {
        match gate::probe_str(&p, subject) {
            Ok(got) if got == want => {}
            Ok(_) => {
                say(o, PROBE_FAIL);
                return Err((1, std::mem::take(o)));
            }
            Err(cause) => return Err(refuse(o, what, cause)),
        }
    }
    Ok(p)
}

pub(super) fn refuse(o: &mut Vec<String>, what: &str, cause: NotRun) -> (u8, Vec<String>) {
    o.push(Verdict::did_not_run(what, Kind::Ban, cause).to_string());
    say(o, &["", "ENGINE-LAYERS: FAIL (did not run)"]);
    // 2, not 1: "the wall is breached" and "I never read the crate" are different operator
    // actions, and phase 2 will move these paths on purpose.
    (2, std::mem::take(o))
}
