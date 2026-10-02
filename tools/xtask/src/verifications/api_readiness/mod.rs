//! API completion is the conjunction of current evidence for every registered requirement.
//!
//! **Role:** `cargo xtask verify api-readiness`: runs the local checks under `--execute`, judges
//! every receipt the acceptance register names, and fails each requirement whose checks did not
//! all hold. [`operational_recording`] writes the receipts of the staging checks.
//!
//! **Position:** `commands/verify/dispatch.rs` calls [`verify`]; the `cargo xtask staging`
//! harness records staging runs through [`operational_recording`]. Receipts and logs live in the
//! evidence folder (default `target/api-readiness`).
//!
//! **Signals & state:** none held; reads the register, the Git tree, the configuration files and
//! the process environment, and writes receipts and logs into the evidence folder.
//!
//! **Invariants:** a receipt counts only against both current fingerprints; a run whose tree or
//! configuration changed while it ran is refused, and `--execute` never runs an `operational`
//! check or a check without a command.

mod case_count;
mod evidence;
mod evidence_storage;
mod fingerprint;
mod operational;
mod operational_log;
pub(crate) mod operational_recording;
mod property_evidence;
mod register;
mod tool_identity;

use anyhow::{Result, ensure};
use evidence::Receipt;
use register::{Check, EvidenceClass};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use verification_core::{Kind, NotRun, Report, Verdict, proc::Run};

pub(crate) fn verify(root: &Path, directory: &Path, execute: bool) -> Result<u8> {
    super::property_test_configuration::PropertyTestConfiguration::from_environment()?;
    let register = register::read(root)?;
    let source = fingerprint::source(root)?;
    let configuration = fingerprint::configuration(root)?;
    let directory = if directory.is_absolute() {
        directory.to_owned()
    } else {
        root.join(directory)
    };
    let execution_failures = if execute {
        execute_local(root, &directory, &register.checks, &source, &configuration)?
    } else {
        BTreeMap::new()
    };
    let mut report = Report::new("api-readiness");
    let mut outcomes = BTreeMap::new();
    for check in &register.checks {
        let path = directory.join(format!("{}.json", check.id));
        let verdict = if let Some(reason) = execution_failures.get(&check.id) {
            Verdict::failed(format!(
                "{}: latest execution did not complete: {reason}",
                check.id
            ))
        } else if !path.is_file() {
            Verdict::did_not_run(
                format!("{} evidence unavailable", check.id),
                Kind::Pin,
                NotRun::TargetMissing(path),
            )
        } else {
            match evidence::read(&directory, check).and_then(|(receipt, output)| {
                evidence::validate(check, &receipt, &output, &source, &configuration, now())
            }) {
                Ok(cases) => {
                    println!("{}: {cases} successful cases ({:?})", check.id, check.class);
                    Verdict::Held
                }
                Err(error) => Verdict::failed(format!("{}: {error:#}", check.id)),
            }
        };
        outcomes.insert(&check.id, matches!(verdict, Verdict::Held));
        report.check(verdict);
    }
    for requirement in &register.requirements {
        if !requirement
            .checks
            .iter()
            .all(|id| outcomes.get(id) == Some(&true))
        {
            report.check(Verdict::failed(format!(
                "{} has unfulfilled acceptance evidence: {}",
                requirement.id, requirement.behavior
            )));
        }
    }
    ensure!(
        source == fingerprint::source(root)?,
        "source changed during API verification; evidence is not a coherent snapshot"
    );
    ensure!(
        configuration == fingerprint::configuration(root)?,
        "configuration changed during API verification"
    );
    Ok(report.finish() as u8)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_secs()
}

fn execute_local(
    root: &Path,
    directory: &Path,
    checks: &[Check],
    source: &str,
    configuration: &str,
) -> Result<BTreeMap<String, String>> {
    let property_configuration =
        super::property_test_configuration::PropertyTestConfiguration::from_environment()?;
    std::fs::create_dir_all(directory)?;
    let versions = tool_identity::versions(root)?;
    let mut outputs = BTreeMap::new();
    let mut attempted = BTreeSet::new();
    let mut failures = BTreeMap::new();
    for check in checks {
        let Some(command) = &check.command else {
            continue;
        };
        if check.class == EvidenceClass::Operational {
            continue;
        }
        if !outputs.contains_key(command) {
            if !attempted.insert(command.clone()) {
                continue;
            }
            println!("Running {}", command.join(" "));
            let started = now();
            match Run::new(&command[0])
                .args(&command[1..])
                .env(
                    "PROPTEST_RNG_SEED",
                    property_configuration.rng_seed.to_string(),
                )
                .env_remove("PROPTEST_CASES")
                .cwd(root)
                .timeout(Duration::from_secs(check.timeout_seconds))
                .merged_output()
            {
                Ok(output) => {
                    outputs.insert(command.clone(), (started, output));
                }
                Err(error) => {
                    // Remove prior evidence so a failed attempt cannot inherit a green receipt.
                    for related in checks
                        .iter()
                        .filter(|c| c.command.as_ref() == Some(command))
                    {
                        failures.insert(related.id.clone(), format!("{error:?}"));
                        match std::fs::remove_file(directory.join(format!("{}.json", related.id))) {
                            Ok(()) => {}
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                            Err(e) => return Err(e.into()),
                        }
                    }
                    eprintln!("{} did not run: {error:?}", check.id);
                    continue;
                }
            }
        }
        let Some((started, output)) = outputs.get(command) else {
            continue;
        };
        let output_file = format!("{}.log", check.id);
        evidence_storage::write(directory, &output_file, output.text.as_bytes())?;
        let receipt = Receipt {
            version: 1,
            check_id: check.id.clone(),
            class: check.class,
            source_sha256: source.to_owned(),
            configuration_sha256: configuration.to_owned(),
            command: command.clone(),
            tool_versions: versions.clone(),
            started_unix_seconds: *started,
            duration_milliseconds: output.duration.as_millis(),
            exit_code: output.code,
            output_file,
            output_sha256: content_digest::sha256_hex(output.text.as_bytes()),
            environment: property_configuration.receipt_environment(),
            observations: None,
            property_runs: property_evidence::parse(&output.text).unwrap_or_default(),
        };
        evidence_storage::write(
            directory,
            &format!("{}.json", check.id),
            &serde_json::to_vec_pretty(&receipt)?,
        )?;
    }
    Ok(failures)
}

#[cfg(test)]
#[path = "tests/evidence.rs"]
mod tests;
