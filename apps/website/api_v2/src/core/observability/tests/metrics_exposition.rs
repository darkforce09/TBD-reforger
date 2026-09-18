//! Text-format invariants that need no HTTP.

use std::time::Duration;

use super::Scrape;
use crate::core::observability::metrics_registry::Registry;

/// The numeric value on the exposition line starting with `prefix`, or `None`.
///
/// Matching the whole line (not `contains`) is deliberate: `contains("tbd_http_requests_total")`
/// is true of the `# TYPE` comment, so a registry that recorded nothing at all would still pass a
/// `contains` assertion.
fn value(body: &str, prefix: &str) -> Option<f64> {
    body.lines()
        .find(|l| l.starts_with(prefix))?
        .rsplit(' ')
        .next()?
        .parse()
        .ok()
}

/// Label escaping is enforced, not assumed.
#[test]
fn label_values_are_escaped() {
    let reg = Registry::new();
    reg.record("GET", "/weird\"\\path", 200, Duration::from_millis(2));
    let out = reg.render(&Scrape {
        db_up: true,
        db_ping: Duration::from_millis(1),
        pool_connections: 3,
        pool_idle: 1,
    });
    assert!(
        out.contains("route=\"/weird\\\"\\\\path\""),
        "unescaped label value would produce unparseable exposition:\n{out}"
    );
    assert_eq!(
        value(&out, "tbd_db_pool_connections{state=\"in_use\"}"),
        Some(2.0)
    );
}
