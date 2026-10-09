//! Count completed generated checks and identify their reproducible input stream.
//!
//! **Role:** [`run_property`] runs a proptest strategy with a fixed seed and case count and prints
//! the [`PropertyRun`] record of the run; [`collect_property`] returns the record instead.
//! **Position:** the whole of this dev-only crate; the API's operations unit tests and its
//! property suites under `crates/api/api_server/tests/` call [`run_property`].
//! **Signals & state:** none kept between runs; one run holds its completed-check count and input
//! digest in cells for the length of the run.
//! **Invariants:** a run with zero cases, a failed check, or fewer completed checks than requested
//! panics instead of producing a record; rejected cases are not counted; the input digest is the
//! SHA-256 of every completed case's `Debug` text, each framed by its length as a `u64`
//! little-endian prefix, so the same seed reproduces the same digest; `PROPTEST_CASES` must be
//! unset and `PROPTEST_RNG_SEED`, when set, is decimal digits.

use std::cell::{Cell, RefCell};

use content_digest::Sha256Hasher;
use proptest::{
    strategy::Strategy,
    test_runner::{Config, RngAlgorithm, RngSeed, TestCaseResult, TestRunner},
};
use serde::Serialize;

/// The record of one property run, printed as one `property-run: <json>` line.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct PropertyRun {
    version: u32,
    id: String,
    requested_cases: u32,
    executed_cases: u32,
    seed: u64,
    algorithm: &'static str,
    input_sha256: String,
}

impl PropertyRun {
    /// How many checks completed; equal to the requested case count in every returned record.
    pub fn executed_cases(&self) -> u32 {
        self.executed_cases
    }

    /// Lowercase hex SHA-256 of the length-framed `Debug` text of every completed case.
    pub fn input_sha256(&self) -> &str {
        &self.input_sha256
    }
}

fn environment_seed() -> u64 {
    assert!(
        std::env::var_os("PROPTEST_CASES").is_none(),
        "PROPTEST_CASES must be unset; property suites own their case counts"
    );
    match std::env::var_os("PROPTEST_RNG_SEED") {
        None => 2_026_092_201,
        Some(value) => {
            let value = value
                .to_str()
                .expect("PROPTEST_RNG_SEED must be UTF-8 decimal digits");
            assert!(
                !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()),
                "PROPTEST_RNG_SEED must be decimal digits"
            );
            value.parse().expect("PROPTEST_RNG_SEED exceeds u64")
        }
    }
}

/// Runs `check` over `cases` values of `strategy` from the environment's seed and prints the
/// run's record as one `property-run: <json>` line; panics when the property does not hold.
pub fn run_property<S: Strategy>(
    property_name: &str,
    cases: u32,
    strategy: &S,
    check: impl Fn(S::Value) -> TestCaseResult,
) {
    let record = collect_property(property_name, cases, environment_seed(), strategy, check);
    println!(
        "property-run: {}",
        serde_json::to_string(&record).expect("a property run record serializes")
    );
}

/// Runs `check` over `cases` values of `strategy` from `seed` and returns the run's record;
/// panics when the property does not hold or a case count is not met.
pub fn collect_property<S: Strategy>(
    property_name: &str,
    cases: u32,
    seed: u64,
    strategy: &S,
    check: impl Fn(S::Value) -> TestCaseResult,
) -> PropertyRun {
    assert!(cases > 0, "property test must execute at least one case");
    assert!(
        !property_name.is_empty()
            && property_name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_'),
        "invalid property ID"
    );
    let config = Config {
        cases,
        rng_seed: RngSeed::Fixed(seed),
        rng_algorithm: RngAlgorithm::ChaCha,
        failure_persistence: None,
        fork: false,
        timeout: 0,
        ..Config::default()
    };
    let completed = Cell::new(0_u32);
    let digest = RefCell::new(Sha256Hasher::new());
    let result = TestRunner::new(config).run(strategy, |value| {
        let encoded = format!("{value:?}");
        check(value)?;
        completed.set(
            completed
                .get()
                .checked_add(1)
                .expect("property count overflow"),
        );
        digest.borrow_mut().update_length_framed(encoded.as_bytes());
        Ok(())
    });
    assert!(
        result.is_ok(),
        "property {property_name}, seed {seed}: {result:?}"
    );
    assert_eq!(
        completed.get(),
        cases,
        "property runner did not complete every required check"
    );
    PropertyRun {
        version: 1,
        id: property_name.to_owned(),
        requested_cases: cases,
        executed_cases: completed.get(),
        seed,
        algorithm: "ChaCha",
        input_sha256: digest.into_inner().finalize_hex(),
    }
}

#[cfg(test)]
#[path = "tests/property_run.rs"]
mod tests;
