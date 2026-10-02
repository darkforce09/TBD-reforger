//! `cargo xtask refactor relocate`: move tracked files and rewrite every reference to them.
//!
//! **Role:** runs the three modes over one checkout. `--dry-run` builds the plan of a manifest and
//! prints it; `--apply` builds it, refuses it while anything is unresolved, writes it and then
//! verifies the same manifest; `--verify` judges one manifest, or every committed manifest when
//! none is named, against the checkout as it stands.
//!
//! **Position:** called by [`super::dispatch`]; built from the manifest parser, the plan builder,
//! the plan application, the summary and the verification in this folder.
//!
//! **Signals & state:** the checkout, which only `--apply` changes.
//!
//! **Invariants:** exit 0 means the plan is clean (dry run) or every judged row held (apply,
//! verify); exit 1 means unresolved references or retired spellings; exit 2 means the run could
//! not reach a verdict: an unreadable or invalid manifest, a move the tree cannot take, an
//! unreadable checkout, or a missing manifests folder — never a pass. The example manifest in the
//! manifests folder is the format sample the tests run and is never judged as a stage manifest.

pub(crate) mod file_treatment;
pub(crate) mod manifest;
pub(crate) mod path_mapping;
pub(crate) mod path_references;
pub(crate) mod plan_application;
pub(crate) mod plan_summary;
pub(crate) mod relocation_plan;
pub(crate) mod repository_files;
pub(crate) mod retired_spellings;
pub(crate) mod rust_lexer;
pub(crate) mod rust_paths;
pub(crate) mod text_edits;
pub(crate) mod text_tokens;

use std::path::{Path, PathBuf};

use verification_core::{Kind, NotRun, Report, Verdict};

use file_treatment::manifests_folder;
use manifest::{ManifestRow, parse_manifest};
use plan_application::apply_plan;
use plan_summary::render_summary;
use relocation_plan::{PlanRefusal, RelocationPlan, build_plan};
use repository_files::RepositorySnapshot;
use retired_spellings::verify_rows;

/// The manifest in the manifests folder that documents the format and feeds the tests.
pub(crate) const EXAMPLE_MANIFEST: &str = "example.tsv";

/// The label every verification report carries.
const VERIFY_LABEL: &str = "refactor relocate --verify";

/// Exit status: the run reached no verdict.
const DID_NOT_RUN: u8 = 2;

/// Exit status: unresolved references or retired spellings.
const FINDINGS: u8 = 1;

/// `--dry-run`: print the plan of `manifest`; 0 when nothing is unresolved.
pub(crate) fn dry_run(root: &Path, manifest: &Path) -> u8 {
    match planned(root, manifest) {
        Ok((label, rows, plan, _)) => {
            print!("{}", render_summary(&label, &rows, &plan));
            if plan.unresolved.is_empty() {
                0
            } else {
                FINDINGS
            }
        }
        Err(code) => code,
    }
}

/// `--apply`: write the plan of `manifest`, then verify it.
pub(crate) fn apply(root: &Path, manifest: &Path) -> u8 {
    let (label, rows, plan, run_manifest) = match planned(root, manifest) {
        Ok(planned) => planned,
        Err(code) => return code,
    };
    print!("{}", render_summary(&label, &rows, &plan));
    if !plan.unresolved.is_empty() {
        eprintln!("refactor relocate --apply: unresolved references; nothing was written");
        return FINDINGS;
    }
    if let Err(reason) = apply_plan(root, &plan) {
        eprintln!("refactor relocate --apply: {reason}");
        return DID_NOT_RUN;
    }
    println!(
        "applied: {} move(s), {} file(s) rewritten",
        plan.moves.len(),
        plan.rewrites.len()
    );
    verify_manifests(root, vec![(label, Ok(rows))], run_manifest.as_deref())
}

/// `--verify`: judge `manifest`, or every committed manifest when it is `None`.
pub(crate) fn verify(root: &Path, manifest: Option<&Path>) -> u8 {
    let manifests = match manifest {
        Some(path) => vec![(path.display().to_string(), read_manifest(path))],
        None => match committed_manifests(root) {
            Ok(paths) => paths
                .into_iter()
                .map(|path| {
                    (
                        repository_path(root, &path).unwrap_or_default(),
                        read_manifest(&path),
                    )
                })
                .collect(),
            Err(cause) => {
                let mut report = Report::new(VERIFY_LABEL);
                report.check(Verdict::did_not_run(
                    "the relocation manifests folder",
                    Kind::Ban,
                    cause,
                ));
                return exit_code(report.finish());
            }
        },
    };
    if manifest.is_none() && manifests.is_empty() {
        println!(
            "no stage manifest in {}; {EXAMPLE_MANIFEST} is the format sample",
            manifests_folder()
        );
    }
    let run_manifest = manifest.and_then(|path| repository_path(root, path));
    verify_manifests(root, manifests, run_manifest.as_deref())
}

fn verify_manifests(
    root: &Path,
    manifests: Vec<(String, ManifestRows)>,
    run_manifest: Option<&str>,
) -> u8 {
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
    for (label, rows) in manifests {
        match rows {
            Ok(rows) => {
                println!("{label}: {} row(s)", rows.len());
                for verdict in verify_rows(&snapshot, &label, &rows, run_manifest) {
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

/// A manifest's rows, or every error that makes it invalid.
type ManifestRows = Result<Vec<ManifestRow>, Vec<String>>;

type Planned = (String, Vec<ManifestRow>, RelocationPlan, Option<String>);

/// The manifest's label, rows, plan and repository path, or the exit status of the refusal.
fn planned(root: &Path, manifest: &Path) -> Result<Planned, u8> {
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
        Ok(plan) => Ok((label, rows, plan, run_manifest)),
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

/// The stage manifests: every `.tsv` in the manifests folder except the example, in name order.
fn committed_manifests(root: &Path) -> Result<Vec<PathBuf>, NotRun> {
    let folder = root.join(manifests_folder());
    let entries = std::fs::read_dir(&folder).map_err(|_| NotRun::TargetMissing(folder.clone()))?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "tsv")
                && path
                    .file_name()
                    .is_some_and(|name| name != EXAMPLE_MANIFEST)
        })
        .collect();
    found.sort();
    Ok(found)
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

#[cfg(test)]
#[path = "tests/fixture_repository.rs"]
mod fixture_repository;

#[cfg(test)]
#[path = "tests/building_blocks.rs"]
mod building_blocks;

#[cfg(test)]
#[path = "tests/relocation_scenarios.rs"]
mod relocation_scenarios;

#[cfg(test)]
#[path = "tests/rust_path_scenarios.rs"]
mod rust_path_scenarios;
