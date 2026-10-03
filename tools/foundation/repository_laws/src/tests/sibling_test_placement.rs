//! Tests for [`super`] — inline test-module bodies, found by name or by an enabling `cfg`.
//!
//! Every fixture is a one-line string with `\n` escapes, so no line of this file opens with the
//! module keyword.

use super::*;
use crate::temporary_checkout::TemporaryCheckout;

fn names(text: &str) -> Vec<String> {
    inline_test_modules_in(text)
        .into_iter()
        .map(|(_, module)| module)
        .collect()
}

#[test]
fn a_cfg_test_module_body_is_found_on_its_mod_line() {
    let text = "//! Header.\n\npub fn f() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n";
    assert_eq!(inline_test_modules_in(text), [(6, "tests".to_string())]);
}

#[test]
fn a_sibling_file_declaration_is_not_an_inline_body() {
    let text = "#[cfg(test)]\n#[path = \"tests/example.rs\"]\nmod tests;\n";
    assert!(names(text).is_empty());
}

#[test]
fn a_module_named_tests_counts_without_any_cfg() {
    assert_eq!(names("mod tests {\n}\n"), ["tests"]);
    assert_eq!(names("pub(crate) mod test { }\n"), ["test"]);
}

#[test]
fn the_cfg_predicate_decides_and_not_test_never_counts() {
    let text = "#[cfg(any(target_arch = \"wasm32\", test))]\nmod forbidden {}\n\
                #[cfg(all(\n    test,\n    feature = \"store\"\n))]\nmod spanning {}\n\
                #[cfg(not(test))] mod production_only {}\n\
                #[cfg_attr(test, allow(dead_code))] mod conditional_lint {}\n\
                #[cfg_attr(feature = \"extra\", cfg(test))] mod conditional_tests {}\n\
                #[cfg(not(target_arch = \"wasm32\"))]\npub(crate) mod role_id {}\n";
    assert_eq!(names(text), ["forbidden", "spanning", "conditional_tests"]);
}

#[test]
fn doc_comments_and_other_attributes_keep_the_cfg_in_force() {
    let text = "#[cfg(test)]\n/// Fixture builders.\n#[allow(dead_code)]\n\nmod builders {}\n";
    assert_eq!(names(text), ["builders"]);
}

#[test]
fn a_code_line_ends_the_attribute_block() {
    let text = "#[cfg(test)]\nuse std::fmt;\nmod helpers {}\n";
    assert!(names(text).is_empty());
}

#[test]
fn a_quoted_module_is_not_a_module() {
    let text = "const SOURCE: &str = \"#[cfg(test)] mod string_content {}\";\n\
                // #[cfg(test)] mod comment_content {}\n\
                fn helper() { mod nested_in_a_line {} }\n";
    assert!(names(text).is_empty());
}

#[test]
fn attributes_parse_as_cfg_predicates() {
    assert!(attribute_enables_tests("#[cfg(test)]"));
    assert!(attribute_enables_tests(
        "#[cfg(all(test, feature = \"store\"))]"
    ));
    assert!(!attribute_enables_tests("#[cfg(not(test))]"));
    assert!(!attribute_enables_tests("#[cfg(feature = \"test\")]"));
    assert!(!attribute_enables_tests("#[path = \"tests/test.rs\"]"));
    assert!(!attribute_enables_tests("#[allow(test)]"));
}

#[test]
fn the_scan_reads_production_files_and_skips_the_sibling_test_files() {
    let checkout = TemporaryCheckout::with_law_roots("placement");
    let inline = "//! Header.\n#[cfg(test)]\nmod tests {}\n";
    checkout.write("apps/api/src/core/clock.rs", inline);
    checkout.write("apps/api/src/core/tests/clock.rs", inline);
    checkout.write("tools/xtask/src/clock_tests.rs", inline);
    let scan = scan_inline_test_modules(checkout.root()).unwrap();
    assert_eq!(
        scan.findings,
        [InlineTestModule {
            path: "apps/api/src/core/clock.rs".into(),
            line_no: 3,
            module: "tests".into(),
        }]
    );
    assert_eq!(
        scan.findings[0].rendered(),
        "apps/api/src/core/clock.rs:3: inline test module `tests` — move its body to \
         a sibling tests/ file"
    );
    assert!(scan.production_files > 0);
}
