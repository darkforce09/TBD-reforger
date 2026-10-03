//! The READMEs `ballistics trim-export` writes beside the calibration bundle.
//!
//! **Role:** Renders the fixture folder's README (contents, full provenance: game build, export
//! generation, every resource GUID with its SHA-256, the oracle run and its output hashes, the
//! gravity as reported and as stored, the document hashes, and how many native rows each kind of
//! elevation evidence fixed) and the `negative/` folder's README (each refused bundle and its defect).
//!
//! **Position:** Called by [`super::trim_export::trim_export`] with its [`TrimReport`]; the text is
//! written in the same run as the documents it describes, so the hashes always match.
//!
//! **Signals & state:** none; pure string rendering.
//!
//! **Invariants:** The output depends only on the report, so it is as deterministic as the
//! documents. Paths outside the tracked tree (the gitignored export and oracle folders) are
//! written as plain text, never as code spans.
use super::row_elevations::ElevationEvidence;
use super::trim_export::{CATALOG_ID, CATALOG_VERSION, TrimReport};
use std::fmt::Write as _;

fn evidence_label(evidence: ElevationEvidence) -> &'static str {
    match evidence {
        ElevationEvidence::ForwardSample => "Fixed by equality with one forward sample",
        ElevationEvidence::LatticeEnd => "Fixed at a lattice end",
    }
}

/// The README of `contracts/fixtures/ballistics/<catalog>.v<version>/`.
pub(crate) fn fixture_readme(report: &TrimReport) -> String {
    let catalog = &report.catalog;
    let generation = &catalog.export_generation_id;
    let folder = format!("contracts/fixtures/ballistics/{CATALOG_ID}.v{CATALOG_VERSION}");
    let catalog_path =
        format!("contracts/catalogs/ballistics/{CATALOG_ID}.v{CATALOG_VERSION}.catalog.json");
    let rows: usize = report.evidence_counts.values().sum();
    let samples: usize = report.sample_counts.values().sum();
    let mut text = String::new();
    let _ = writeln!(text, "# Vanilla mortar calibration bundle\n");
    let _ = writeln!(
        text,
        "The calibration cases the ballistics flight model must reproduce before version {CATALOG_VERSION} of the\n\
         `{CATALOG_ID}` catalog is accepted: the game's own ballistic and wind tables for every charge of\n\
         its {} shells, and the engine oracle's answers for the same shells. The catalog is\n\
         `{catalog_path}`. The folder is written whole by\n\
         `cargo xtask ballistics trim-export --generation {generation}` and is never edited by hand.\n",
        catalog.shells.len()
    );
    let _ = writeln!(text, "## Contents\n\n```text\n{folder}/");
    let _ = writeln!(
        text,
        "├── calibration.json  {} native tables ({rows} rows), {} wind tables, {samples} oracle samples",
        report.native_table_count, report.wind_table_count
    );
    let _ = writeln!(
        text,
        "└── negative/         four bundles the evaluator must refuse, one defect each\n```\n"
    );
    let _ = writeln!(
        text,
        "## How it works\n\n\
         The trim reads the gameplay equipment export of generation {generation} (plain path\n\
         assets/equipment/gameplay/generations/{generation}/export, gitignored) and the ballistics oracle's\n\
         output for it (assets/scratch/ballistics_oracle/{generation}, gitignored). It verifies every export\n\
         file against the export manifest's SHA-256 and each oracle file against its `_meta.json` sidecar, then\n\
         keeps only what the catalog's charges need: the native table at each charge coefficient, the wind\n\
         tables at charge coefficients, and the oracle's forward-angle, simulation and altitude-difference\n\
         samples at those coefficients. Every engine number is written as the shortest decimal of the 32-bit\n\
         float the engine holds.\n\n\
         A game table row stores range, an uninterpreted second column and time of flight, but no elevation.\n\
         The oracle samples the engine's forward lookup on a {step}-mil elevation lattice, which holds every\n\
         row's elevation, so each row's `elevation_mils_6400` is fixed by a forward sample whose range and time\n\
         of flight equal the row's within 0.01 m and 0.001 s, or by a lattice end, which the engine answers\n\
         with the time-of-flight sentinel −1 (the first row is the vertical shot at range 0, the last sits at\n\
         the last lattice elevation and its range equals the range the engine reports there). Every sample\n\
         between two adjacent rows equals their linear interpolation. A row neither rule fixes refuses the\n\
         trim, so every native row of every kept table is in the bundle.\n",
        step = report.oracle.lattice_step_mils
    );
    let _ = writeln!(text, "## Provenance\n");
    let _ = writeln!(text, "- Game build: {}.", catalog.game_build);
    let _ = writeln!(text, "- Export generation: {generation}.");
    let _ = writeln!(
        text,
        "- Catalog SHA-256 (the bundle's `catalog_sha256`): `{}`.",
        report.catalog_sha256
    );
    let _ = writeln!(
        text,
        "- Calibration bundle SHA-256: `{}`.",
        report.calibration_sha256
    );
    let oracle = &report.oracle;
    let _ = writeln!(
        text,
        "- Oracle run: plugin revision {}, first run started {}; `output_sha256` `{}` is the SHA-256 of the\n  `sha256sum` listing of the two output files below, in name order.",
        oracle.plugin_revision, oracle.run_at, oracle.output_sha256
    );
    for (file, sha256) in &oracle.file_sha256 {
        let _ = writeln!(text, "  - {file}: `{sha256}`, equal to its sidecar.");
    }
    let _ = writeln!(
        text,
        "- Gravity: {} reported {} m/s²; the catalog's `gravity_m_s2` and the bundle's\n  `gravity_reported_m_s2` store {} m/s², the shortest decimal of that 32-bit float.\n",
        oracle.gravity_source, oracle.gravity_raw_m_s2, catalog.gravity_m_s2
    );
    let _ = writeln!(
        text,
        "| Document | GUID | Resource | SHA-256 of the exported file |\n|---|---|---|---|"
    );
    for (document, resources) in [
        ("catalog", &catalog.resources),
        ("calibration", &report.bundle_resources),
    ] {
        for resource in resources.iter() {
            let _ = writeln!(
                text,
                "| {document} | {} | {} | `{}` |",
                resource.guid, resource.resource_name, resource.sha256
            );
        }
    }
    let _ = writeln!(text, "\n## Native row elevations\n");
    for (evidence, count) in &report.evidence_counts {
        let _ = writeln!(text, "- {}: {count}.", evidence_label(*evidence));
    }
    let _ = writeln!(
        text,
        "- Every native row is matched; none is interpolated or left out."
    );
    let _ = writeln!(text, "\n## Format\n");
    let _ = writeln!(
        text,
        "- Encoding: UTF-8 JSON indented by two spaces, one table row, wind row, resource or oracle sample per\n  line, with a final newline.\n\
         - Schema: `contracts/definitions/ballistics-calibration.schema.json`; the catalog it pins follows\n  `contracts/definitions/ballistics-catalog.schema.json`.\n\
         - Oracle samples carry the charge's `rings` first in `inputs`, then the engine call's arguments under\n  the oracle's names; `outputs` are the oracle's results unchanged.\n\
         - Changing a file: rerun the trim after a new export or oracle run, then `cargo xtask schema validate`.\n"
    );
    let _ = writeln!(
        text,
        "## Producers and consumers\n\n\
         - Producer: `cargo xtask ballistics trim-export` (`tools/commands/ballistics_oracle_tooling/src/`).\n\
         - Consumers: `cargo xtask schema validate`, whose ballistics section checks both documents and their\n  provenance and coverage; the `ballistics_calibration` tests; the upload of the catalog pair through\n  `POST /api/v1/ballistics-catalogs`."
    );
    text
}

/// The README of the fixture's `negative/` folder.
pub(crate) fn negative_readme(report: &TrimReport) -> String {
    let folder = format!("contracts/fixtures/ballistics/{CATALOG_ID}.v{CATALOG_VERSION}/negative");
    let width = report
        .negatives
        .iter()
        .map(|(file, _, _)| file.len())
        .max()
        .unwrap_or_default();
    let mut text = String::new();
    let _ = writeln!(text, "# Refused calibration bundles\n");
    let _ = writeln!(
        text,
        "Copies of `../calibration.json` that each carry exactly one defect, so the calibration evaluator\n\
         and the catalog upload must refuse every one of them for its own reason. The trim writes them with the\n\
         good bundle; they are never edited by hand.\n"
    );
    let _ = writeln!(text, "## Contents\n\n```text\n{folder}/");
    for (index, (file, defect, _)) in report.negatives.iter().enumerate() {
        let branch = if index + 1 == report.negatives.len() {
            "└──"
        } else {
            "├──"
        };
        let _ = writeln!(text, "{branch} {file:<width$}  {defect}");
    }
    let _ = writeln!(text, "```\n\n## Hashes\n");
    for (file, _, sha256) in &report.negatives {
        let _ = writeln!(text, "- {file}: `{sha256}`");
    }
    text
}
