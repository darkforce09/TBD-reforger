//! Count completed generated checks and identify their reproducible input stream.

use proptest::{
    strategy::Strategy,
    test_runner::{Config, RngAlgorithm, RngSeed, TestCaseResult, TestRunner},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::cell::{Cell, RefCell};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct PropertyRun {
    pub version: u32,
    pub id: String,
    pub requested_cases: u32,
    pub executed_cases: u32,
    pub seed: u64,
    pub algorithm: &'static str,
    pub input_sha256: String,
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

pub(crate) fn run_property<S: Strategy>(
    id: &str,
    cases: u32,
    strategy: &S,
    check: impl Fn(S::Value) -> TestCaseResult,
) {
    let record = collect_property(id, cases, environment_seed(), strategy, check);
    println!("property-run: {}", serde_json::to_string(&record).unwrap());
}

pub(crate) fn collect_property<S: Strategy>(
    id: &str,
    cases: u32,
    seed: u64,
    strategy: &S,
    check: impl Fn(S::Value) -> TestCaseResult,
) -> PropertyRun {
    assert!(cases > 0, "property test must execute at least one case");
    assert!(
        !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
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
    let digest = RefCell::new(Sha256::new());
    let result = TestRunner::new(config).run(strategy, |value| {
        let encoded = format!("{value:?}");
        check(value)?;
        completed.set(
            completed
                .get()
                .checked_add(1)
                .expect("property count overflow"),
        );
        let mut hash = digest.borrow_mut();
        hash.update((encoded.len() as u64).to_le_bytes());
        hash.update(encoded.as_bytes());
        Ok(())
    });
    assert!(result.is_ok(), "property {id}, seed {seed}: {result:?}");
    assert_eq!(
        completed.get(),
        cases,
        "property runner did not complete every required check"
    );
    PropertyRun {
        version: 1,
        id: id.to_owned(),
        requested_cases: cases,
        executed_cases: completed.get(),
        seed,
        algorithm: "ChaCha",
        input_sha256: hex::encode(digest.into_inner().finalize()),
    }
}
