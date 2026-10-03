//! Pins on how the application state builds its shared services, read from its source where
//! the value is a constructor argument no accessor exposes.

use api_http_layer::middleware::{DURABLE_STRICT_BURST, DURABLE_STRICT_RPS};

/// The durable tier's numbers are the strict tier's numbers. `AppState::new` builds the
/// strict limiter as `IpLimiter::new(1, 10)`; if one side is retuned and the other is not, L1
/// silently becomes the only limiter that can ever refuse (or L2 starts refusing traffic L1
/// was sized to allow).
#[test]
fn durable_strict_policy_matches_the_in_memory_strict_policy() {
    assert_eq!(DURABLE_STRICT_RPS, 1);
    assert_eq!(DURABLE_STRICT_BURST, 10);
    let src = include_str!("../application_state.rs");
    assert!(
        src.contains(&format!(
            "IpLimiter::new({DURABLE_STRICT_RPS}, {DURABLE_STRICT_BURST})"
        )),
        "application_state.rs no longer builds the strict limiter as IpLimiter::new({DURABLE_STRICT_RPS}, \
         {DURABLE_STRICT_BURST}) — retune the durable tier with it"
    );
}
