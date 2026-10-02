//! The relocation verification: no live file still spells what a manifest retired.
//!
//! **Role:** judges one or more manifests against a [`TrackedTree`]: the checkout, or the tree a
//! plan would leave. For each `path` row: its `from` is no longer tracked, and no live text file
//! spells it on segment boundaries, whether from the repository root, after a leading `/`, after
//! the letter of a control escape, behind a deployment prefix (a `file:` URL's path included) or
//! through `..` segments that name nothing any more. For each `rust_path` row: no `.rs` file in
//! its scope, read after the moves, still holds the retired prefix in a path, a `use` tree, a
//! comment or a string literal. Each row is one [`Verdict`].
//!
//! **Position:** `cargo xtask refactor relocate --verify` and the last step of `--apply` judge the
//! checkout; `--dry-run` and the first check of `--apply` judge the planned tree
//! ([`super::planned_tree::PlannedTree`]).
//!
//! **Signals & state:** none held; reads the judged tree once per run.
//!
//! **Invariants:** the files judged and the parts of them judged are the ones the rewrite passes
//! may edit (same [`TreatmentAreas::treatment_of`] and [`allowed_spans`], the areas moved where the
//! manifest's own moves put them), so a clean apply verifies clean; the
//! frozen records and the manifests are never judged; a spelling that, with the segments before
//! it or the escape letter glued to it, names a path that exists now is another path and no
//! finding; a listing that cannot be read is a did-not-run, never a pass.

use verification_core::{Finding, Kind, NotRun, Verdict};

use super::file_treatment::{FileTreatment, TreatmentAreas};
use super::manifest::{ManifestRow, RowKind, RowScope};
use super::path_mapping::{PathMapping, normalize, parent_folder};
use super::path_references::allowed_spans;
use super::path_references::path_tokens::{PathOccurrence, classify_occurrence, match_starts};
use super::path_references::relative_references::ReferenceFileKind;
use super::repository_files::{FileContent, PathSet, TrackedTree};
use super::rust_lexer::line_of;
use super::rust_paths::path_rules::RustPathRules;
use super::rust_paths::rust_path_edits;

/// How many offending lines one finding lists before it counts the rest.
const LISTED_OFFENCES: usize = 40;

/// One live text file, as the verification reads it.
struct JudgedFile {
    path: String,
    text: String,
    treatment: FileTreatment,
}

/// The verdict of every row of `rows`, the rows of the manifest named `label`, over `tree`.
pub(crate) fn verify_rows(
    tree: &impl TrackedTree,
    label: &str,
    rows: &[ManifestRow],
    run_manifest: Option<&str>,
) -> Vec<Verdict> {
    let mapping = PathMapping::from_rows(rows);
    let areas = TreatmentAreas::current(run_manifest).relocated(&mapping);
    let files = match judged_files(tree, &areas) {
        Ok(files) => files,
        Err(cause) => {
            return vec![Verdict::did_not_run(
                format!("{label}: the tracked files could not be read"),
                Kind::Ban,
                cause,
            )];
        }
    };
    rows.iter()
        .filter_map(|row| match row.kind {
            RowKind::Path => Some(verify_path_row(tree.paths(), &files, label, row)),
            RowKind::RustPath => Some(verify_rust_path_row(tree, &files, &mapping, label, row)),
            RowKind::Text => None,
        })
        .collect()
}

fn judged_files(
    tree: &impl TrackedTree,
    areas: &TreatmentAreas,
) -> Result<Vec<JudgedFile>, NotRun> {
    let mut files = Vec::new();
    for path in tree.paths().files() {
        let FileContent::Text(text) = tree.read(path)? else {
            continue;
        };
        let treatment = areas.treatment_of(path, &text);
        if matches!(
            treatment,
            FileTreatment::Excluded | FileTreatment::FrozenDocument
        ) {
            continue;
        }
        files.push(JudgedFile {
            path: path.clone(),
            text,
            treatment,
        });
    }
    Ok(files)
}

fn verify_path_row(
    paths: &PathSet,
    files: &[JudgedFile],
    label: &str,
    row: &ManifestRow,
) -> Verdict {
    let mut offences = Vec::new();
    if paths.contains(&row.from) {
        offences.push(format!("`{}` is still tracked", row.from));
    }
    for file in files {
        let allowed = allowed_spans(
            &file.text,
            file.treatment,
            ReferenceFileKind::of(&file.path),
        );
        for start in match_starts(&file.text, &row.from) {
            let end = start + row.from.len();
            if !allowed.allows(&(start..end)) || !is_retired_spelling(paths, file, start, end) {
                continue;
            }
            offences.push(offence(&file.path, &file.text, start));
        }
    }
    verdict(label, row, offences)
}

/// Whether the occurrence at `start..end` still spells the retired path.
fn is_retired_spelling(paths: &PathSet, file: &JudgedFile, start: usize, end: usize) -> bool {
    match classify_occurrence(&file.text, start, end) {
        PathOccurrence::NotAPath | PathOccurrence::Url => false,
        PathOccurrence::RepositoryRoot => true,
        PathOccurrence::AfterControlEscape => !paths.contains(&file.text[start - 1..end]),
        PathOccurrence::Embedded { token_start } => {
            !paths.contains(file.text[token_start..end].trim_start_matches('/'))
        }
        PathOccurrence::Relative { token_start } => {
            let literal = &file.text[token_start..end];
            let folder = parent_folder(&file.path);
            let anchors = [
                folder.to_string(),
                paths.crate_folder_of(folder),
                String::new(),
            ];
            !anchors.iter().any(|anchor| {
                normalize(anchor, literal)
                    .is_some_and(|target| !target.is_empty() && paths.contains(&target))
            })
        }
    }
}

fn verify_rust_path_row(
    tree: &impl TrackedTree,
    files: &[JudgedFile],
    mapping: &PathMapping,
    label: &str,
    row: &ManifestRow,
) -> Verdict {
    let scope = match &row.scope {
        RowScope::Everywhere => RowScope::Everywhere,
        RowScope::Folder(folder) => RowScope::Folder(mapping.relocate(folder).into_owned()),
        RowScope::Glob(pattern) => RowScope::Glob(mapping.relocate(pattern).into_owned()),
    };
    if let RowScope::Folder(folder) = &scope
        && !tree.paths().contains(folder)
    {
        return Verdict::did_not_run(
            format!("{label} line {}: the scope of `{}`", row.line, row.from),
            Kind::Ban,
            NotRun::TargetMissing(tree.root().join(folder)),
        );
    }
    let rules = RustPathRules::from_rows(std::iter::once(row));
    let mut offences = Vec::new();
    for file in files {
        if file.treatment != FileTreatment::Live
            || !file.path.ends_with(".rs")
            || !scope.contains(&file.path)
        {
            continue;
        }
        let outcome = rust_path_edits(&file.text, &rules, None);
        let offsets = outcome
            .edits
            .iter()
            .map(|edit| edit.span.start)
            .chain(outcome.unresolved.iter().map(|(offset, _)| *offset));
        offences.extend(offsets.map(|offset| offence(&file.path, &file.text, offset)));
    }
    verdict(label, row, offences)
}

fn offence(path: &str, text: &str, offset: usize) -> String {
    let line = line_of(text, offset);
    let content = text.lines().nth(line - 1).unwrap_or("").trim();
    let shown: String = content.chars().take(120).collect();
    format!("{path}:{line}: {shown}")
}

fn verdict(label: &str, row: &ManifestRow, offences: Vec<String>) -> Verdict {
    if offences.is_empty() {
        return Verdict::Held;
    }
    let mut detail: Vec<String> = offences.iter().take(LISTED_OFFENCES).cloned().collect();
    if offences.len() > LISTED_OFFENCES {
        detail.push(format!("… and {} more", offences.len() - LISTED_OFFENCES));
    }
    Verdict::Failed(Finding {
        headline: format!(
            "{label} line {}: {} retired spelling(s) of `{}` ({}) remain",
            row.line,
            offences.len(),
            row.from,
            row.kind.label()
        ),
        detail,
    })
}
