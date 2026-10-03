//! The single-pass verification against the per-row judge it replaces.
//!
//! The oracle below is the verification as it ran before the single pass: every manifest reads
//! the whole tree, and every row scans every judged file on its own. A checkout with several
//! composed manifests (shared and overlapping spellings, a single-segment path that is also a Rust
//! prefix segment, scopes a later manifest moved or emptied, a missing scope, a manifest that moves
//! a frozen area, frozen records, a closed ticket, a migration, over forty offences in one row) is
//! judged by both, and every verdict and note must render identically.

use std::path::Path;

use verification_core::{Finding, Kind, NotRun, Verdict};

use super::file_treatment::{FileTreatment, TreatmentAreas, manifests_folder};
use super::fixture_repository::FixtureRepository;
use super::manifest::{ManifestRow, RowKind, RowScope, parse_manifest};
use super::manifest_chronology::chronological_manifests;
use super::path_mapping::{PathMapping, normalize, parent_folder};
use super::path_references::allowed_spans;
use super::path_references::path_tokens::{PathOccurrence, classify_occurrence, match_starts};
use super::path_references::relative_references::ReferenceFileKind;
use super::repository_files::{FileContent, PathSet, RepositorySnapshot, TrackedTree};
use super::retired_spellings::{ManifestToJudge, RowJudgement, judge_manifests};
use super::rust_lexer::line_of;
use super::rust_paths::path_rules::RustPathRules;
use super::rust_paths::rust_path_edits;
use super::scope_history::LaterMoves;
use super::verify;

const HEADER: &str = "kind\tfrom\tto\tscope\n";

#[test]
fn relocate_verify_single_pass_matches_the_per_row_judge_over_composed_manifests() {
    let repo = composed_checkout();
    let snapshot = RepositorySnapshot::load(repo.root()).expect("list the fixture checkout");
    let manifests = composed_manifests(repo.root());
    assert_eq!(manifests.len(), 4, "four stage manifests are composed");

    let expected: Vec<String> = manifests
        .iter()
        .flat_map(|(label, rows, later)| {
            rendered(oracle_verify_rows(&snapshot, label, rows, None, later))
        })
        .collect();
    let judged: Vec<ManifestToJudge<'_>> = manifests
        .iter()
        .map(|(label, rows, later)| ManifestToJudge {
            label,
            rows,
            run_manifest: None,
            later,
        })
        .collect();
    let actual: Vec<String> = judge_manifests(&snapshot, &judged)
        .into_iter()
        .flat_map(rendered)
        .collect();

    assert_eq!(
        actual, expected,
        "the single pass judges as the per-row judge"
    );
    let failed = expected
        .iter()
        .filter(|line| line.contains("Failed"))
        .count();
    assert!(
        failed >= 6,
        "the fixture keeps retired spellings: {expected:#?}"
    );
    assert!(
        expected.iter().any(|line| line.contains("… and ")),
        "one row lists more than forty offences: {expected:#?}"
    );
    assert!(
        expected.iter().any(|line| line.starts_with("note:")),
        "an emptied scope leaves a note: {expected:#?}"
    );
    assert!(
        expected.iter().any(|line| line.contains("TargetMissing")),
        "a missing scope is a did-not-run: {expected:#?}"
    );
    assert_eq!(
        verify(repo.root(), None),
        2,
        "a missing scope makes the run a did-not-run"
    );
}

/// A checkout after four manifests, three committed in turn and one uncommitted.
fn composed_checkout() -> FixtureRepository {
    let repo = FixtureRepository::new("single-pass");
    let manifest = |name: &str| format!("{}/{name}", manifests_folder());
    let many_offences: String = (0..45)
        .map(|n| format!("step {n}: old_tools/run.cfg\n"))
        .collect();
    repo.write(
        &manifest("stage_a.tsv"),
        &format!(
            "{HEADER}path\told_tools\ttools_moved\t\npath\tlegacy\tretired_layer\t\n\
             rust_path\tcrate::retired::\tcrate::current::\tapp/src/widgets\n\
             rust_path\tcrate::retired::\tcrate::current::\tapp/src/emptied\n\
             rust_path\tcrate::retired::gadgets::\tcrate::current::gadgets::\t\n\
             rust_path\tcrate::retired::\tcrate::glob::\tapp/src/widgets/**/*.rs\n\
             text\tOldName\tNewName\t\n"
        ),
    )
    .write("app/Cargo.toml", "[package]\nname = \"app\"\n")
    .write("app/src/widgets/mod.rs", "pub struct Widget;\n")
    .write("app/src/emptied/mod.rs", "pub struct Thing;\n")
    .commit("stage a");
    repo.write(
        &manifest("stage_b.tsv"),
        &format!(
            "{HEADER}path\tapp/src/widgets\tapp/src/controls\t\n\
             path\told_tools/guide.md\tdocs/guide.md\t\n\
             path\tdocumentation/archive\tdocumentation/history\t\n\
             rust_path\tcrate::retired::\tcrate::other::\t\n"
        ),
    )
    .commit("stage b");
    repo.write(
        &manifest("stage_c.tsv"),
        &format!("{HEADER}path\tapp/src/emptied/mod.rs\tapp/src/flat.rs\t\n"),
    )
    .commit("stage c");
    for moved in ["app/src/widgets", "app/src/emptied"] {
        std::fs::remove_dir_all(repo.root().join(moved)).expect("move a fixture folder away");
    }
    repo.write(
        &manifest("stage_d.tsv"),
        &format!(
            "{HEADER}path\tlegacy\tagain_retired\t\n\
             rust_path\tcrate::gone::\tcrate::here::\tapp/src/nowhere\n"
        ),
    )
    .write(
        "app/src/lib.rs",
        "pub mod controls;\nuse crate::retired::gadgets::Gizmo; // see old_tools/guide.md\n",
    )
    .write(
        "app/src/controls/mod.rs",
        "use crate::{retired::{Widget, gadgets::Gizmo}};\n/// [`crate::retired::Widget`]\n\
         const RUN: &str = \"old_tools/run.cfg\";\nconst LAYER: &str = \"retired/mod.rs\";\n",
    )
    .write("app/src/flat.rs", "use crate::retired::Thing;\n")
    .write("app/migrations/0001_first.sql", "-- old_tools/run.cfg\n")
    .write(
        "README.md",
        &format!(
            "See `old_tools/run.cfg`, /retired/a.md, ./retired/b.md, ../old_tools/c.md, \
             https://example.com/retired/x, my_legacy, retired.md and\\nlegacy/esc.\n{many_offences}"
        ),
    )
    .write(
        "documentation/history/notes.md",
        "Prose `old_tools/a.cfg`, `app/src/widgets/mod.rs` and [a link](/old_tools/b.md), retired/c.\n",
    )
    .write(
        "documentation/archive/kept.md",
        "Prose `retired/a`, `old_tools/guide.md` and [a link](/retired/b.md).\n",
    )
    .write(
        ".ai/tickets/T-1.toml",
        "id = \"T-1\"\nstatus = \"shipped\"\nspec = \"old_tools/spec.md\"\n\
         title = \"old_tools mention\"\n",
    )
    .write("tools_moved/run.cfg", "#!/bin/sh\necho retired/run\n")
    .write("docs/guide.md", "# Guide\n")
    .track();
    repo
}

/// Every stage manifest of the checkout, oldest first, with the moves of the manifests after it.
fn composed_manifests(root: &Path) -> Vec<(String, Vec<ManifestRow>, LaterMoves)> {
    let read: Vec<(String, Vec<ManifestRow>)> = chronological_manifests(root)
        .expect("order the fixture manifests")
        .iter()
        .map(|path| {
            let name = path.file_name().expect("a manifest file name");
            let label = format!("{}/{}", manifests_folder(), name.to_string_lossy());
            let text = std::fs::read_to_string(path).expect("read a fixture manifest");
            (
                label,
                parse_manifest(&text).expect("a valid fixture manifest"),
            )
        })
        .collect();
    read.iter()
        .enumerate()
        .map(|(position, (label, rows))| {
            let later = LaterMoves::new(
                read[position + 1..]
                    .iter()
                    .map(|(label, rows)| (label.clone(), PathMapping::from_rows(rows))),
            );
            (label.clone(), rows.clone(), later)
        })
        .collect()
}

/// One line per note and per verdict, in the order the report receives them.
fn rendered(judgement: RowJudgement) -> Vec<String> {
    judgement
        .notes
        .iter()
        .map(|note| format!("note: {note}"))
        .chain(
            judgement
                .verdicts
                .iter()
                .map(|verdict| format!("{verdict:?}")),
        )
        .collect()
}

/// One live text file, as the per-row judge reads it.
struct OracleFile {
    path: String,
    text: String,
    treatment: FileTreatment,
}

/// The per-row judge: the manifest reads the whole tree, then each row scans every judged file.
fn oracle_verify_rows(
    tree: &impl TrackedTree,
    label: &str,
    rows: &[ManifestRow],
    run_manifest: Option<&str>,
    later: &LaterMoves,
) -> RowJudgement {
    let mapping = PathMapping::from_rows(rows);
    let areas = TreatmentAreas::current(run_manifest).relocated(&mapping);
    let mut judgement = RowJudgement::default();
    let mut files = Vec::new();
    for path in tree.paths().files() {
        let FileContent::Text(text) = tree.read(path).expect("read a fixture file") else {
            continue;
        };
        let treatment = areas.treatment_of(path, &text);
        if !matches!(
            treatment,
            FileTreatment::Excluded | FileTreatment::FrozenDocument
        ) {
            files.push(OracleFile {
                path: path.clone(),
                text,
                treatment,
            });
        }
    }
    for row in rows {
        let verdict = match row.kind {
            RowKind::Path => oracle_path_row(tree.paths(), &files, label, row),
            RowKind::RustPath => match oracle_scope(tree, &mapping, later, label, row) {
                Ok(scope) => oracle_rust_path_row(&files, &scope, label, row),
                Err(Ok(note)) => {
                    judgement.notes.push(note);
                    Verdict::Held
                }
                Err(Err(verdict)) => verdict,
            },
            RowKind::Text => continue,
        };
        judgement.verdicts.push(verdict);
    }
    judgement
}

fn oracle_path_row(
    paths: &PathSet,
    files: &[OracleFile],
    label: &str,
    row: &ManifestRow,
) -> Verdict {
    let mut offences = Vec::new();
    if paths.contains(&row.from) {
        offences.push(format!("`{}` is still tracked", row.from));
    }
    for file in files {
        let kind = ReferenceFileKind::of(&file.path);
        let allowed = allowed_spans(&file.text, file.treatment, kind);
        for start in match_starts(&file.text, &row.from) {
            let end = start + row.from.len();
            if allowed.allows(&(start..end)) && oracle_is_retired(paths, file, start, end) {
                offences.push(oracle_offence(&file.path, &file.text, start));
            }
        }
    }
    oracle_verdict(label, row, offences)
}

fn oracle_is_retired(paths: &PathSet, file: &OracleFile, start: usize, end: usize) -> bool {
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

/// The judged scope, or the emptied scope's note, or the missing scope's did-not-run.
fn oracle_scope(
    tree: &impl TrackedTree,
    mapping: &PathMapping,
    later: &LaterMoves,
    label: &str,
    row: &ManifestRow,
) -> Result<RowScope, Result<String, Verdict>> {
    let folder = match &row.scope {
        RowScope::Everywhere => return Ok(RowScope::Everywhere),
        RowScope::Glob(pattern) => {
            return Ok(RowScope::Glob(
                later.follow_pattern(&mapping.relocate(pattern)),
            ));
        }
        RowScope::Folder(folder) => later.follow_folder(&mapping.relocate(folder)),
    };
    if tree.paths().contains(&folder.folder) {
        return Ok(RowScope::Folder(folder.folder));
    }
    Err(match folder.emptied_by {
        Some(emptied_by) => Ok(format!(
            "{label} line {}: the scope `{}` of `{}` holds nothing after {emptied_by} moved its \
             files; nothing left to judge there",
            row.line,
            row.scope.spelling(),
            row.from
        )),
        None => Err(Verdict::did_not_run(
            format!("{label} line {}: the scope of `{}`", row.line, row.from),
            Kind::Ban,
            NotRun::TargetMissing(tree.root().join(&folder.folder)),
        )),
    })
}

fn oracle_rust_path_row(
    files: &[OracleFile],
    scope: &RowScope,
    label: &str,
    row: &ManifestRow,
) -> Verdict {
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
        offences.extend(offsets.map(|offset| oracle_offence(&file.path, &file.text, offset)));
    }
    oracle_verdict(label, row, offences)
}

fn oracle_offence(path: &str, text: &str, offset: usize) -> String {
    let line = line_of(text, offset);
    let content = text.lines().nth(line - 1).unwrap_or("").trim();
    let shown: String = content.chars().take(120).collect();
    format!("{path}:{line}: {shown}")
}

fn oracle_verdict(label: &str, row: &ManifestRow, offences: Vec<String>) -> Verdict {
    if offences.is_empty() {
        return Verdict::Held;
    }
    let mut detail: Vec<String> = offences.iter().take(40).cloned().collect();
    if offences.len() > 40 {
        detail.push(format!("… and {} more", offences.len() - 40));
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
