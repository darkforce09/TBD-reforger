//! `gate ballistics-agreement`: the fire-mission solver's WebAssembly build against its native
//! build, case by case.
//!
//! **Role:** proves the captured catalog goldens are the committed catalog, solves the seeded
//! agreement cases natively over the committed catalog, has the browser bench
//! `/debug/ballistics-agreement` solve the same cases from the served goldens, and prints one
//! `case ballistics_wasm_agreement_<id> ... ok|FAILED` line per case, the bit-identity count and
//! `ballistics-wasm-agreement: PASS n/n` (or `FAIL`).
//! **Position:** a `gate` subcommand dispatched by [`crate::command_lines::gate`]; run by
//! `cargo xtask mk ballistics-wasm-agreement` after `trunk build --release`. Its parts:
//! [`golden_provenance`] (the served catalog), [`native_reference`] (the host solves),
//! [`browser_session`] (the bench in Chromium), [`bench_reading`] (the bench's JSON) and
//! [`case_verdict`] (the judgement and the printed lines).
//! **Signals & state:** none beyond one run's server and browser, owned by [`browser_session`].
//! **Invariants:** exit 0 only when every case agrees and at least one case ran; a missing or
//! mismatching golden, a failed bench, an undecodable reading or a disagreeing case exit 1; an
//! unreadable committed catalog or a driver failure is an error (exit 3 through the CLI).

pub mod bench_reading;
pub mod browser_session;
pub mod case_verdict;
pub mod golden_provenance;
pub mod native_reference;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::Result;
use crate::error::ResultExt;
use ballistics_model::catalog::BallisticsCatalog;
use fire_mission_planning::fire_mission::SOLVER_REVISION;

use ::repository_layout::find_repository_root;
use bench_reading::decode_bench_reading;
use browser_session::{BenchSession, read_bench};
use case_verdict::{AgreementVerdict, RequestedRun, judge_reading};
use golden_provenance::check_served_goldens;
use native_reference::native_cases;

/// The committed catalog the cases are drawn over, relative to the repository root.
pub const COMMITTED_CATALOG: &str = "contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json";
/// The captured API goldens (the default of [`AgreementArgs::api_goldens`]), relative to the
/// repository root.
pub const API_GOLDENS: &str = "contracts/fixtures/api_goldens";
/// The bench's route.
pub const BENCH_ROUTE: &str = "/debug/ballistics-agreement";

/// One run's parameters.
#[derive(Clone, Debug)]
pub struct AgreementArgs {
    /// The built single-page app (relative paths resolve against the repository root).
    pub dist: PathBuf,
    /// Seed of the case lattice.
    pub seed: u64,
    /// Number of cases.
    pub count: usize,
    /// The committed catalog file.
    pub catalog: PathBuf,
    /// The folder of captured API goldens the catalog reads are answered from.
    pub api_goldens: PathBuf,
    /// Port of the static server.
    pub port: u16,
    /// Chromium's remote-debugging port.
    pub debug_port: u16,
    /// Longest wait for the bench, seconds.
    pub timeout_s: u64,
}

/// Runs the gate and returns its exit code.
///
/// # Errors
///
/// An unreadable or undecodable committed catalog.
pub async fn run(args: &AgreementArgs) -> Result<u8> {
    let root = find_repository_root()?;
    let catalog_path = under_root(&root, &args.catalog);
    let committed_bytes = std::fs::read(&catalog_path)
        .with_context(|| format!("read the committed catalog {}", catalog_path.display()))?;
    let committed = BallisticsCatalog::from_json_slice(&committed_bytes)
        .with_context(|| format!("decode the committed catalog {}", catalog_path.display()))?;
    let requested = RequestedRun {
        seed: args.seed,
        count: args.count,
        catalog_id: committed.catalog_id.clone(),
        catalog_version: committed.catalog_version,
        solver_revision: SOLVER_REVISION.to_string(),
    };
    let verdict = match check_served_goldens(
        &under_root(&root, &args.api_goldens),
        &committed_bytes,
        &committed,
    ) {
        Err(cause) => AgreementVerdict::failed(cause),
        Ok(goldens) => {
            let started = Instant::now();
            let native = native_cases(&committed, args.seed, args.count);
            println!(
                "native: {} cases solved in {:.1} s",
                native.len(),
                started.elapsed().as_secs_f64()
            );
            let session = BenchSession {
                dist: under_root(&root, &args.dist),
                port: args.port,
                debug_port: args.debug_port,
                bench_path: format!(
                    "{BENCH_ROUTE}?seed={}&count={}&catalog={}&version={}",
                    args.seed, args.count, committed.catalog_id, committed.catalog_version
                ),
                catalog: (committed.catalog_id.to_string(), committed.catalog_version),
                timeout: Duration::from_secs(args.timeout_s),
            };
            match read_bench(&session, &goldens).await {
                Err(error) => AgreementVerdict::failed(error.with_causes()),
                Ok(output) => {
                    for path in &output.unanswered {
                        println!("note: the page asked {path}, answered 404");
                    }
                    match decode_bench_reading(&output.reading) {
                        Err(error) => AgreementVerdict::failed(error.with_causes()),
                        Ok(reading) => judge_reading(&requested, &native, &reading),
                    }
                }
            }
        }
    };
    for line in verdict.lines() {
        println!("{line}");
    }
    Ok(verdict.exit_code())
}

fn under_root(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}
