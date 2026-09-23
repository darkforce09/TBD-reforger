//! Reproducible property-test seeds with case counts owned by each test suite.

use anyhow::{Context, Result, ensure};
use std::ffi::OsStr;

const DEFAULT_RNG_SEED: u64 = 2_026_092_201;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PropertyTestConfiguration {
    pub rng_seed: u64,
}

impl PropertyTestConfiguration {
    /// Reject case-count overrides before running any acceptance tests.
    pub(crate) fn from_environment() -> Result<Self> {
        let cases = std::env::var_os("PROPTEST_CASES");
        let seed = std::env::var_os("PROPTEST_RNG_SEED");
        Self::from_values(cases.as_deref(), seed.as_deref())
    }

    /// Parse supplied values independently of process-global environment state.
    fn from_values(cases: Option<&OsStr>, seed: Option<&OsStr>) -> Result<Self> {
        ensure!(
            cases.is_none(),
            "PROPTEST_CASES must be unset; each property test defines its own case count"
        );
        let rng_seed = match seed {
            None => DEFAULT_RNG_SEED,
            Some(seed) => {
                let seed = seed
                    .to_str()
                    .context("PROPTEST_RNG_SEED must contain UTF-8 decimal digits")?;
                ensure!(
                    !seed.is_empty() && seed.bytes().all(|byte| byte.is_ascii_digit()),
                    "PROPTEST_RNG_SEED must contain only decimal digits"
                );
                seed.parse::<u64>()
                    .context("PROPTEST_RNG_SEED exceeds the u64 range")?
            }
        };
        Ok(Self { rng_seed })
    }

    pub(crate) fn marker(&self) -> String {
        format!(
            "property-test-configuration: rng_seed={}; cases=suite-defined",
            self.rng_seed
        )
    }

    pub(crate) fn receipt_environment(&self) -> Vec<String> {
        vec![
            format!("PROPTEST_RNG_SEED={}", self.rng_seed),
            "PROPTEST_CASES=suite-defined".into(),
        ]
    }
}

#[cfg(test)]
#[path = "tests/property_test_configuration.rs"]
mod tests;
