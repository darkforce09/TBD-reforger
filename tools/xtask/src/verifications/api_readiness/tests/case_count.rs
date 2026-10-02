//! Cargo runner boundaries separate identical case names without inflating repeated receipts.

use super::successful_cases;
use regex::Regex;

fn rust_cases() -> Regex {
    Regex::new(r"test [^\r\n]+ \.\.\. ok").unwrap()
}

fn unit_header(binary: &str) -> String {
    format!("     Running unittests src/lib.rs ({binary})\n")
}

fn integration_header(source: &str, binary: &str) -> String {
    format!("     Running tests/{source}.rs ({binary})\n")
}

#[test]
fn identical_names_in_different_test_binaries_count_separately() {
    let output = format!(
        "{}test checks::same_name ... ok\n{}test checks::same_name ... ok\n",
        integration_header("first", "target/debug/deps/first-a123"),
        integration_header("second", "/tmp/project/target/debug/deps/second-b456"),
    );
    assert_eq!(successful_cases(&rust_cases(), &output), 2);
}

#[test]
fn repeated_headers_cases_and_entire_cargo_output_do_not_inflate_counts() {
    let first = unit_header("target/debug/deps/website_api-a123");
    let second = integration_header("integration", "target/debug/deps/integration-b456");
    let output = format!(
        "{first}test same_name ... ok\ntest same_name ... ok\n\
         {first}test same_name ... ok\ntest unit_only ... ok\n\
         {second}test same_name ... ok\n   Doc-tests website_api\n\
         test src/lib.rs - example (line 10) ... ok\n",
    );
    assert_eq!(successful_cases(&rust_cases(), &output), 4);
    assert_eq!(successful_cases(&rust_cases(), &output.repeat(2)), 4);
}

#[test]
fn unit_integration_and_documentation_suites_have_independent_identities() {
    let output = format!(
        "{}test same ... ok\n{}test same ... ok\n   Doc-tests website_api\n\
         test same ... ok\n   Doc-tests other_crate\ntest same ... ok\n",
        unit_header("target/debug/deps/website_api-a123"),
        integration_header("api", "target/debug/deps/api-b456"),
    );
    assert_eq!(successful_cases(&rust_cases(), &output), 4);
}

#[test]
fn executable_path_not_display_label_or_basename_identifies_a_binary() {
    let output = format!(
        "{}test same ... ok\n{}test same ... ok\n{}test same ... ok\n",
        unit_header("/tmp/first/target/debug/deps/shared-a123"),
        integration_header(
            "renamed_display",
            "/tmp/first/target/debug/deps/shared-a123"
        ),
        unit_header("/tmp/second/target/debug/deps/shared-a123"),
    );
    assert_eq!(successful_cases(&rust_cases(), &output), 2);
}

#[test]
fn no_runner_header_keeps_non_cargo_checks_in_one_scope() {
    let pattern = Regex::new(r"PASS [a-z_-]+").unwrap();
    let output = "PASS architecture\nPASS schema_parity\nPASS architecture\n";
    assert_eq!(successful_cases(&pattern, output), 2);
    assert_eq!(successful_cases(&pattern, &output.repeat(2)), 2);
    assert_eq!(successful_cases(&pattern, "failed all checks"), 0);
    assert_eq!(successful_cases(&pattern, ""), 0);
}

#[test]
fn patterns_can_match_multiple_cases_per_line_and_span_lines() {
    let inline = Regex::new(r"passed:[a-z]+").unwrap();
    assert_eq!(
        successful_cases(&inline, "passed:one passed:two\npassed:one passed:three"),
        3
    );
    let multiline = Regex::new(r"(?m)^case [a-z]+\nstatus ok$").unwrap();
    let output = format!(
        "{}case same\nstatus ok\ncase same\nstatus ok\n{}case same\nstatus ok\n",
        unit_header("target/debug/deps/unit-a123"),
        integration_header("integration", "target/debug/deps/integration-b456"),
    );
    assert_eq!(successful_cases(&multiline, &output), 2);
}

#[test]
fn match_scope_uses_its_start_without_truncating_at_a_runner_boundary() {
    let pattern =
        Regex::new(r"(?m)^begin\n\s*Running tests/next\.rs \(target/debug/deps/next-b456\)\nend$")
            .unwrap();
    let output = "begin\n     Running tests/next.rs (target/debug/deps/next-b456)\nend\n";
    assert_eq!(successful_cases(&pattern, output), 1);
}

#[test]
fn crlf_runner_headers_and_cargo_profile_paths_are_recognized() {
    let output = concat!(
        "     Running unittests src/lib.rs (target/release/deps/lib-a123)\r\n",
        "test same ... ok\r\n",
        "Running tests/api.rs (/tmp/build-output/debug/deps/api-b456)\r\n",
        "test same ... ok\r\n",
        "\tDoc-tests website_api\r\n",
        "test same ... ok\r\n",
    );
    assert_eq!(successful_cases(&rust_cases(), output), 3);
}

#[test]
fn malformed_and_embedded_runner_headers_do_not_create_new_scopes() {
    for malformed in [
        "log: Running tests/other.rs (target/debug/deps/other-b456)",
        "quoted 'Running tests/other.rs (target/debug/deps/other-b456)'",
        "     Running tests/other.rs (target/debug/deps/other-b456) trailing text",
        "     Running tests/other.rs target/debug/deps/other-b456",
        "     Running tests/other.rs (not-an-executable-path)",
        "     Running tests/other.rs (target/debug/other-b456)",
        "     Running tests/other.rs (target/debug/deps/)",
        "     Running unittests (target/debug/deps/other-b456)",
        "     Running tests/other.txt (target/debug/deps/other-b456)",
        "     Running arbitrary command (target/debug/deps/other-b456)",
        "test output: Doc-tests other_crate",
        "   Doc-tests other_crate trailing text",
        "   Doc-tests ",
    ] {
        let output = format!(
            "{}test same ... ok\n{malformed}\ntest same ... ok\n",
            unit_header("target/debug/deps/website_api-a123"),
        );
        assert_eq!(successful_cases(&rust_cases(), &output), 1, "{malformed}");
    }
}
