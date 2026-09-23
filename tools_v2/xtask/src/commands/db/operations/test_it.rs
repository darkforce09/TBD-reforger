//! Isolated integration-test invocation and ownership-checked database cleanup.
//!
//! An operator label selects a human-readable prefix, never a shared database to erase.
//! Each invocation reserves a random namespace with CREATE DATABASE before running tests.
//! Its complete namespace fits inside the per-suite identifier's preserved prefix.

use std::io::Read;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, ensure};

use super::{IT_BASE_DB, IT_MAINT_DB, echo, finish_status, runtime, web};
use crate::commands::deploy::database_operations as dbc;

/// A plain identifier that can safely appear unquoted in PostgreSQL commands.
fn is_scratch_identifier(name: &str) -> bool {
    name.len() <= 63
        && name
            .as_bytes()
            .first()
            .is_some_and(|b| b.is_ascii_lowercase() || *b == b'_')
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        && dbc::is_safe_scratch_database_name(name)
}

fn validate_label(label: &str) -> Result<(), String> {
    if label
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && dbc::is_safe_scratch_database_name(label)
    {
        return Ok(());
    }
    Err(format!(
        "REFUSING integration database label {label:?} (scratch allow-list). \
         Use an ASCII label matching rust_it, tbd_gate*, *_cold, *_it, or *_probe. \
         The label is never dropped; each invocation allocates its own database."
    ))
}

/// Read and validate the optional operator label before allocating any database.
pub(crate) fn guarded_base() -> Result<String, String> {
    let label = std::env::var("TBD_IT_BASE_DB")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| IT_BASE_DB.to_string());
    validate_label(&label)?;
    Ok(label)
}

/// Keep the namespace at most 42 bytes so its trailing separator fits in the
/// integration harness's 43-byte prefix even when a long suite name is hashed.
fn namespace_from_random(label: &str, random: [u8; 16]) -> String {
    let prefix: String = label
        .chars()
        .take(5)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let suffix: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("i{prefix}_{suffix}_it")
}

fn invocation_database_name(label: &str) -> Result<String> {
    validate_label(label).map_err(anyhow::Error::msg)?;
    let mut random = [0_u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(&mut random))
        .context("read OS randomness for the integration database namespace")?;
    let name = namespace_from_random(label, random);
    ensure!(
        is_scratch_identifier(&name),
        "invalid generated database namespace"
    );
    Ok(name)
}

/// Select only exact namespace matches; underscores have no wildcard meaning.
pub(crate) fn reap_select(base: &str) -> String {
    // Quoting remains defensive even though reap validates the identifier first.
    let quoted = base.replace('\'', "''");
    format!(
        "SELECT datname FROM pg_database WHERE datname = '{quoted}' OR \
         (left(datname, {}) = '{quoted}_' AND right(datname, 3) = '_it' \
         AND length(datname) > {})",
        base.len() + 1,
        base.len() + 4,
    )
}

/// Independently reject unrelated or malformed rows returned by the cleanup query.
fn owns_database(base: &str, database: &str) -> bool {
    is_scratch_identifier(base)
        && is_scratch_identifier(database)
        && (database == base
            || database
                .strip_prefix(base)
                .and_then(|suffix| suffix.strip_prefix('_'))
                .and_then(|suffix| suffix.strip_suffix("_it"))
                .is_some_and(|suite| !suite.is_empty()))
}

/// A pre-bridged container psql command against the maintenance database.
fn psql_cmd(sql_flag: &str, sql: &str) -> (Command, String) {
    let (rt, logical) = runtime();
    let container = dbc::db_container();
    let user = dbc::db_user();
    let echo_line =
        format!("{logical} exec {container} psql -U {user} -d {IT_MAINT_DB} {sql_flag} \"{sql}\"");
    let mut cmd = Command::new(&rt[0]);
    cmd.args(&rt[1..]).args([
        "exec",
        &container,
        "psql",
        "-U",
        &user,
        "-d",
        IT_MAINT_DB,
        sql_flag,
        sql,
    ]);
    (cmd, echo_line)
}

/// A development narrowing of the suite. The empty selection is the canonical complete run.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct TestSelection {
    pub binaries: Vec<String>,
    pub library: bool,
    pub name_filter: Option<String>,
}

impl TestSelection {
    fn is_complete_suite(&self) -> bool {
        self.binaries.is_empty() && !self.library && self.name_filter.is_none()
    }
}

fn is_selector_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
}

/// Cargo arguments for the selection; libtest always retains successful property records.
pub(crate) fn cargo_test_arguments(selection: &TestSelection) -> Result<Vec<String>, String> {
    let mut arguments: Vec<String> = ["test", "--locked", "--no-fail-fast"]
        .map(String::from)
        .to_vec();
    if selection.library {
        arguments.push("--lib".into());
    }
    for binary in &selection.binaries {
        if !is_selector_text(binary) || binary.contains(':') {
            return Err(format!("REFUSING test binary selector {binary:?}"));
        }
        arguments.extend(["--test".into(), binary.clone()]);
    }
    arguments.extend(["--".into(), "--show-output".into()]);
    if let Some(filter) = &selection.name_filter {
        if !is_selector_text(filter) {
            return Err(format!("REFUSING test name filter {filter:?}"));
        }
        arguments.push(filter.clone());
    }
    Ok(arguments)
}

pub(crate) fn run(selection: TestSelection) -> Result<u8> {
    let property_configuration = crate::verifications::property_test_configuration::PropertyTestConfiguration::from_environment()?;
    println!("{}", property_configuration.marker());
    let arguments = match cargo_test_arguments(&selection) {
        Ok(arguments) => arguments,
        Err(message) => {
            eprintln!("{message}");
            return Ok(2);
        }
    };
    if !selection.is_complete_suite() {
        println!("test-selection: narrowed development run; not a readiness receipt");
    }
    let label = match guarded_base() {
        Ok(label) => label,
        Err(message) => {
            eprintln!("{message}");
            return Ok(1);
        }
    };
    let base = invocation_database_name(&label)?;
    let web = web()?;

    // CREATE claims ownership atomically. A random-name collision fails without
    // dropping, cleaning, or otherwise touching a database owned by another run.
    let (mut create_cmd, create_echo) = psql_cmd("-qc", &format!("CREATE DATABASE {base};"));
    echo(&create_echo);
    let status = create_cmd.status().context("psql CREATE DATABASE")?;
    let rc = finish_status("psql -qc CREATE DATABASE", status);
    if rc != 0 {
        return Ok(rc);
    }

    let url = format!("postgres://tbd:tbd@localhost:5434/{base}?sslmode=disable");
    echo(&format!(
        "cd {} && cargo test (isolated database {base})",
        web.rel
    ));
    run_with_cleanup(
        || {
            let status = Command::new("cargo")
                .args(&arguments)
                .env(
                    "PROPTEST_RNG_SEED",
                    property_configuration.rng_seed.to_string(),
                )
                .env_remove("PROPTEST_CASES")
                .current_dir(&web.abs)
                .env("TEST_DATABASE_URL", &url)
                .env("TBD_API_VERIFICATION", "true")
                .status()
                .context("failed to spawn cargo test")?;
            Ok(finish_status("cargo test", status))
        },
        || reap(&base),
    )
}

/// Attempt cleanup after every test outcome, including failure to start Cargo.
fn run_with_cleanup(
    execute: impl FnOnce() -> Result<u8>,
    cleanup: impl FnOnce() -> Result<u8>,
) -> Result<u8> {
    let result = execute();
    let cleanup_result = cleanup();
    match (result, cleanup_result) {
        (Ok(test_rc), Ok(cleanup_rc)) => Ok(join_rc(test_rc, cleanup_rc)),
        (Err(error), Ok(0)) => Err(error),
        (Err(error), Ok(cleanup_rc)) => {
            Err(error.context(format!("database cleanup also exited {cleanup_rc}")))
        }
        (Ok(test_rc), Err(error)) => {
            Err(error.context(format!("database cleanup failed after test exit {test_rc}")))
        }
        (Err(error), Err(cleanup_error)) => {
            Err(error.context(format!("database cleanup also failed: {cleanup_error:#}")))
        }
    }
}

pub(crate) fn join_rc(test_rc: u8, reap_rc: u8) -> u8 {
    if test_rc != 0 { test_rc } else { reap_rc }
}

/// Drop the invocation database and its suites, independently checking ownership.
pub(crate) fn reap(base: &str) -> Result<u8> {
    ensure!(
        is_scratch_identifier(base),
        "invalid cleanup namespace {base:?}"
    );
    let (rc, stdout, stderr) = dbc::ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            dbc::db_user(),
            "-d".into(),
            IT_MAINT_DB.into(),
            "-Atc".into(),
            reap_select(base),
        ],
    )?;
    if rc != 0 {
        eprint!("{stderr}");
        eprintln!("FATAL: cleanup query failed (psql rc={rc}); database cleanup did not complete.");
        return Ok(if rc > 0 { rc.clamp(1, 255) as u8 } else { 1 });
    }
    let (rt, _) = runtime();
    let container = dbc::db_container();
    let mut worst = 0u8;
    for database in stdout.lines().filter(|line| !line.is_empty()) {
        if !owns_database(base, database) {
            eprintln!("REFUSING to drop {database:?} — not owned by namespace {base:?}.");
            worst = 1;
            continue;
        }
        let status = Command::new(&rt[0])
            .args(&rt[1..])
            .args([
                "exec",
                &container,
                "psql",
                "-U",
                &dbc::db_user(),
                "-d",
                IT_MAINT_DB,
                "-qc",
                &format!("DROP DATABASE IF EXISTS {database} WITH (FORCE);"),
            ])
            .stdout(Stdio::null())
            .status()
            .context("psql DROP DATABASE")?;
        let rc = finish_status("psql -qc DROP DATABASE", status);
        if rc != 0 {
            worst = rc;
        }
    }
    Ok(worst)
}

#[cfg(test)]
#[path = "tests/test_it/tests.rs"]
mod tests;
