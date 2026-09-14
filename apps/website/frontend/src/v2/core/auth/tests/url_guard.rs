//! The two copies of the scheme guard must agree, on a table that still contains the attacks.

use super::*;

// The shared input table, the same file the API crate's own tests include. Editing either
// implementation without the other turns this red.
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../shared/is_http_url_cases.rs"
));

#[test]
fn matches_the_api_guard_on_every_shared_case() {
    let mut wrong = Vec::new();
    for (input, expected) in IS_HTTP_URL_CASES {
        let got = is_http_url(input);
        if got != *expected {
            wrong.push(format!("  {input:?}: expected {expected}, got {got}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "frontend is_http_url disagrees with the shared table on {} of {} cases \
         (the API guard is checked against the SAME table in \
         website-api services::text — if that one is green and this one is not, \
         the two implementations have DRIFTED):\n{}",
        wrong.len(),
        IS_HTTP_URL_CASES.len(),
        wrong.join("\n")
    );
}

/// The table is only worth anything if it actually contains the attacks. A future edit that
/// trims it down to the easy cases would leave the test above green and meaningless.
#[test]
fn shared_table_still_carries_the_adversarial_inputs() {
    let inputs: Vec<&str> = IS_HTTP_URL_CASES.iter().map(|(i, _)| *i).collect();
    for required in [
        "javascript:alert(1)",
        "JaVaScRiPt:alert(1)",
        "java%73cript:alert(1)",
        "java\tscript:alert(1)",
        "java\nscript:alert(1)",
        "java\rscript:alert(1)",
        "jav\u{0}ascript:alert(1)",
        "\u{200b}javascript:alert(1)",
        "\u{feff}javascript:alert(1)",
        "\u{a0}javascript:alert(1)",
        "\u{ad}javascript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "vbscript:msgbox(1)",
        "blob:https://evil.com/1234",
        "file:///etc/passwd",
        "//evil.com",
        "http://",
    ] {
        assert!(
            inputs.contains(&required),
            "the shared case table no longer contains {required:?} — \
             adversarial cases are not to be removed"
        );
    }
    let rejects = IS_HTTP_URL_CASES.iter().filter(|(_, ok)| !ok).count();
    assert!(
        rejects >= 40,
        "the shared table is down to {rejects} reject cases; it carried forty"
    );
}
