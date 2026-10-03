//! Tests of the live ledger's page readers on a native build.

use super::*;
use map_streaming_model::memory_budget::{DEFAULT_BUDGET_MB, MIB};

#[test]
fn a_native_build_takes_the_default_budget_and_measures_no_linear_memory() {
    assert_eq!(
        configured_budget_bytes(),
        DEFAULT_BUDGET_MB * MIB,
        "a native build has no query string and no window global to read a budget from"
    );
    assert_eq!(
        heap_bytes(),
        0,
        "there is no wasm linear memory on the host"
    );
}
