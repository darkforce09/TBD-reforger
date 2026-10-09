//! `cargo xtask refactor relocate`: move tracked files and rewrite every reference to them.
//!
//! **Role:** runs the three modes over one checkout. `--dry-run` builds the plan of a manifest,
//! prints it and verifies the tree the plan would leave, in memory; `--apply` does the same,
//! refuses the plan while anything is unresolved or the planned tree keeps a retired spelling,
//! writes it and then verifies the same manifest on the checkout; `--verify` judges one manifest,
//! or every committed manifest when none is named, against the checkout as it stands, each
//! manifest's scopes followed through the moves of the stage manifests after it in the order they
//! entered the history, and its retired `path` spellings legal again where those moves revived
//! them ([`crate::manifest_chronology`], [`crate::scope_history`]).
//!
//! **Position:** re-exported at the crate root and called by the `cargo xtask refactor relocate`
//! dispatch of the xtask binary; built from the manifest parser, the plan builder, the plan
//! application, the summary and the verification of this crate.
//!
//! **Signals & state:** the checkout, which only `--apply` changes.
//!
//! **Invariants:** exit 0 means the plan is clean and its planned tree verifies clean (dry run),
//! or every judged row held (apply, verify); exit 1 means unresolved references or retired
//! spellings, in the checkout or in the planned tree; exit 2 means the run could not reach a
//! verdict: an unreadable or invalid manifest, a move the tree cannot take, an unreadable
//! checkout, or a missing manifests folder — never a pass. A dry run and an apply judge the
//! planned tree with the verification `--verify` runs, so a plan that passes its dry run leaves a
//! checkout that verifies clean. The example manifest in the manifests folder is the format sample
//! the tests run and is never judged as a stage manifest.

use std::path::{Path, PathBuf};

use verification_core::{Kind, NotRun, Report, Verdict};

use crate::file_treatment::manifests_folder;
use crate::manifest::{ManifestRow, parse_manifest};
use crate::manifest_chronology::chronological_manifests;
use crate::path_mapping::{PathMapping, parent_folder};
use crate::plan_application::apply_plan;
use crate::plan_summary::render_summary;
use crate::planned_tree::PlannedTree;
use crate::relocation_plan::{PlanRefusal, RelocationPlan, build_plan};
use crate::repository_files::RepositorySnapshot;
use crate::retired_spellings::{ManifestToJudge, RowJudgement, judge_manifests};
use crate::scope_history::LaterMoves;

/// The manifest in the manifests folder that documents the format and feeds the tests.
pub(crate) const EXAMPLE_MANIFEST: &str = "example.tsv";

/// The label every verification report of the checkout carries.
const VERIFY_LABEL: &str = "refactor relocate --verify";

/// The label of the verification report of the tree a plan would leave.
const PLANNED_TREE_LABEL: &str = "refactor relocate: the planned tree";

/// Exit status: the run reached no verdict.
const DID_NOT_RUN: u8 = 2;

/// Exit status: unresolved references or retired spellings.
const FINDINGS: u8 = 1;

/// `--dry-run`: print the plan of `manifest` and verify the tree it would leave; 0 when nothing
/// is unresolved and the planned tree verifies clean, 1 on findings, 2 when no verdict was reached.
pub fn dry_run(root: &Path, manifest: &Path) -> u8 {
    match planned(root, manifest) {
        Ok(run) => {
            print!("{}", render_summary(&run.label, &run.rows, &run.plan));
            plan_verdict(&run)
        }
        Err(code) => code,
    }
}

/// `--apply`: check the plan of `manifest` as `--dry-run` does, write it, then verify it; returns
/// the exit code [`dry_run`] documents, and 2 when a write failed and every change was undone.
pub fn apply(root: &Path, manifest: &Path) -> u8 {
    let run = match planned(root, manifest) {
        Ok(run) => run,
        Err(code) => return code,
    };
    print!("{}", render_summary(&run.label, &run.rows, &run.plan));
    let verdict = plan_verdict(&run);
    if verdict != 0 {
        eprintln!(
            "refactor relocate --apply: unresolved references or retired spellings in the planned \
             tree; nothing was written"
        );
        return verdict;
    }
    if let Err(reason) = apply_plan(root, &run.plan) {
        eprintln!("refactor relocate --apply: {reason}");
        return DID_NOT_RUN;
    }
    println!(
        "applied: {} move(s), {} file(s) rewritten",
        run.plan.moves.len(),
        run.plan.rewrites.len()
    );
    verify_manifests(
        root,
        vec![JudgedManifest {
            label: run.label,
            rows: Ok(run.rows),
            later: LaterMoves::none(),
        }],
        run.run_manifest.as_deref(),
    )
}

/// The verdict on a plan before anything is written: the verification of the tree it would leave
/// (printed), combined with its unresolved items. 2 when the planned tree could not be judged, 1
/// when anything is unresolved or the planned tree keeps a retired spelling, 0 otherwise.
fn plan_verdict(run: &PlannedRun) -> u8 {
    let mapping = PathMapping::from_rows(&run.rows);
    let tree = PlannedTree::new(&run.snapshot, &mapping, &run.plan);
    let mut report = Report::new(PLANNED_TREE_LABEL);
    let later = LaterMoves::none();
    let judged = [ManifestToJudge {
        label: &run.label,
        rows: &run.rows,
        run_manifest: run.run_manifest.as_deref(),
        later: &later,
    }];
    for judgement in judge_manifests(&tree, &judged) {
        for verdict in judgement.verdicts {
            report.check(verdict);
        }
    }
    match exit_code(report.finish()) {
        DID_NOT_RUN => DID_NOT_RUN,
        0 if run.plan.unresolved.is_empty() => 0,
        _ => FINDINGS,
    }
}

/// `--verify`: judge `manifest`, or every committed manifest when it is `None`, each with its
/// scopes followed through the stage manifests after it; 0 when every row held, 1 on a retired
/// spelling, 2 when a manifest, the manifests' history or the checkout could not be read.
pub fn verify(root: &Path, manifest: Option<&Path>) -> u8 {
    let run_manifest = manifest.and_then(|path| repository_path(root, path));
    let judged = match manifest {
        Some(path) => named_manifest(root, path, run_manifest.as_deref()),
        None => stage_manifests(root),
    };
    let judged = match judged {
        Ok(judged) => judged,
        Err(cause) => {
            let mut report = Report::new(VERIFY_LABEL);
            report.check(Verdict::did_not_run(
                "the relocation manifests folder",
                Kind::Ban,
                cause,
            ));
            return exit_code(report.finish());
        }
    };
    if manifest.is_none() && judged.is_empty() {
        println!(
            "no stage manifest in {}; {EXAMPLE_MANIFEST} is the format sample",
            manifests_folder()
        );
    }
    verify_manifests(root, judged, run_manifest.as_deref())
}

/// Every stage manifest, oldest first, each with the moves of the valid manifests after it.
fn stage_manifests(root: &Path) -> Result<Vec<JudgedManifest>, NotRun> {
    let read = read_in_order(root)?;
    let mut judged = Vec::with_capacity(read.len());
    for (index, (label, rows)) in read.iter().enumerate() {
        judged.push(JudgedManifest {
            label: label.clone(),
            rows: rows.clone(),
            later: later_moves(&read[index + 1..]),
        });
    }
    Ok(judged)
}

/// The manifest at `path`: when it is a stage manifest of the folder, with the moves of the
/// valid stage manifests after it; otherwise with none.
fn named_manifest(
    root: &Path,
    path: &Path,
    run_manifest: Option<&str>,
) -> Result<Vec<JudgedManifest>, NotRun> {
    let in_folder =
        run_manifest.is_some_and(|spelled| parent_folder(spelled) == manifests_folder());
    let later = if in_folder {
        let read = read_in_order(root)?;
        let position = read
            .iter()
            .position(|(label, _)| Some(label.as_str()) == run_manifest);
        position.map_or_else(LaterMoves::none, |index| later_moves(&read[index + 1..]))
    } else {
        LaterMoves::none()
    };
    Ok(vec![JudgedManifest {
        label: path.display().to_string(),
        rows: read_manifest(path),
        later,
    }])
}

/// Every stage manifest, oldest first, as its repository path and its rows.
fn read_in_order(root: &Path) -> Result<Vec<(String, ManifestRows)>, NotRun> {
    Ok(chronological_manifests(root)?
        .into_iter()
        .map(|path| {
            (
                repository_path(root, &path).unwrap_or_default(),
                read_manifest(&path),
            )
        })
        .collect())
}

/// The moves of `manifests`, oldest first; an invalid manifest moves nothing and is reported on
/// its own.
fn later_moves(manifests: &[(String, ManifestRows)]) -> LaterMoves {
    LaterMoves::new(manifests.iter().filter_map(|(label, rows)| {
        rows.as_ref()
            .ok()
            .map(|rows| (label.clone(), PathMapping::from_rows(rows)))
    }))
}

fn verify_manifests(root: &Path, manifests: Vec<JudgedManifest>, run_manifest: Option<&str>) -> u8 {
    let mut report = Report::new(VERIFY_LABEL);
    let snapshot = match RepositorySnapshot::load(root) {
        Ok(snapshot) => snapshot,
        Err(cause) => {
            report.check(Verdict::did_not_run(
                "the checkout listing",
                Kind::Ban,
                cause,
            ));
            return exit_code(report.finish());
        }
    };
    let valid: Vec<ManifestToJudge<'_>> = manifests
        .iter()
        .filter_map(|manifest| {
            manifest.rows.as_ref().ok().map(|rows| ManifestToJudge {
                label: &manifest.label,
                rows,
                run_manifest,
                later: &manifest.later,
            })
        })
        .collect();
    let mut judgements = judge_manifests(&snapshot, &valid).into_iter();
    for JudgedManifest { label, rows, .. } in &manifests {
        match rows {
            Ok(rows) => {
                println!("{label}: {} row(s)", rows.len());
                let judgement = judgements.next().unwrap_or_else(|| unjudged(label));
                for note in judgement.notes {
                    println!("  note: {note}");
                }
                for verdict in judgement.verdicts {
                    report.check(verdict);
                }
            }
            Err(errors) => {
                report.check(Verdict::did_not_run(
                    format!("{label}: invalid manifest ({})", errors.join("; ")),
                    Kind::Ban,
                    NotRun::Unreadable {
                        path: PathBuf::from(&label),
                        source: std::io::Error::other("invalid manifest"),
                    },
                ));
            }
        }
    }
    exit_code(report.finish())
}

/// The did-not-run of a valid manifest the verification returned no judgement for.
fn unjudged(label: &str) -> RowJudgement {
    RowJudgement {
        verdicts: vec![Verdict::did_not_run(
            format!("{label}: no judgement"),
            Kind::Ban,
            NotRun::ToolError {
                tool: "the relocation verification".to_string(),
                status: -1,
                stderr: "one judgement per valid manifest was expected".to_string(),
            },
        )],
        notes: Vec::new(),
    }
}

/// One manifest to judge: its label, its rows, and the moves of the manifests after it.
struct JudgedManifest {
    label: String,
    rows: ManifestRows,
    later: LaterMoves,
}

/// A manifest's rows, or every error that makes it invalid.
type ManifestRows = Result<Vec<ManifestRow>, Vec<String>>;

/// One manifest's plan over one checkout, with what the plan was built from.
struct PlannedRun {
    /// The manifest as the command line named it.
    label: String,
    rows: Vec<ManifestRow>,
    snapshot: RepositorySnapshot,
    plan: RelocationPlan,
    /// The manifest's repository path, when it lies in the checkout.
    run_manifest: Option<String>,
}

/// The manifest's plan over the checkout at `root`, or the exit status of the refusal.
fn planned(root: &Path, manifest: &Path) -> Result<PlannedRun, u8> {
    let label = manifest.display().to_string();
    let rows = read_manifest(manifest).map_err(|errors| {
        eprintln!("{label}: invalid manifest");
        for error in errors {
            eprintln!("  {error}");
        }
        DID_NOT_RUN
    })?;
    let snapshot = RepositorySnapshot::load(root).map_err(|cause| {
        eprintln!("{label}: the checkout could not be listed: {cause:?}");
        DID_NOT_RUN
    })?;
    let run_manifest = repository_path(root, manifest);
    match build_plan(&snapshot, &rows, run_manifest.as_deref()) {
        Ok(plan) => Ok(PlannedRun {
            label,
            rows,
            snapshot,
            plan,
            run_manifest,
        }),
        Err(PlanRefusal::Refused(errors)) => {
            eprintln!("{label}: the checkout cannot take these moves");
            for error in errors {
                eprintln!("  {error}");
            }
            Err(DID_NOT_RUN)
        }
        Err(PlanRefusal::NotRun(cause)) => {
            eprintln!("{label}: a file could not be read: {cause:?}");
            Err(DID_NOT_RUN)
        }
    }
}

fn read_manifest(path: &Path) -> ManifestRows {
    let text = std::fs::read_to_string(path)
        .map_err(|error| vec![format!("{} could not be read: {error}", path.display())])?;
    parse_manifest(&text)
}

/// `path` relative to `root`, `/`-separated, when it lies inside the checkout.
fn repository_path(root: &Path, path: &Path) -> Option<String> {
    let absolute = std::fs::canonicalize(path).ok()?;
    let base = std::fs::canonicalize(root).ok()?;
    let relative = absolute.strip_prefix(base).ok()?;
    Some(relative.to_string_lossy().replace('\\', "/"))
}

fn exit_code(status: i32) -> u8 {
    u8::try_from(status).unwrap_or(DID_NOT_RUN)
}
