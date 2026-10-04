//! Unit tests for the editor's source-pin support.

use super::production_half;

#[test]
fn the_production_half_ends_at_the_test_module_declaration() {
    let src =
        "fn shipped() {}\n#[cfg(test)]\n#[path = \"tests/x.rs\"]\nmod tests;\nfn after() {}\n";
    assert_eq!(production_half(src), "fn shipped() {}\n");
}

#[test]
fn a_test_gated_item_above_the_declaration_stays_in_the_production_half() {
    let src = "#[cfg(test)]\npub use inner::helper;\nfn shipped() {}\n#[cfg(test)] mod tests;\n";
    assert_eq!(
        production_half(src),
        "#[cfg(test)]\npub use inner::helper;\nfn shipped() {}\n"
    );
}

#[test]
fn a_visibility_and_an_inline_module_body_still_declare_the_test_module() {
    let src = "fn shipped() {}\n    #[cfg(test)]\n    pub(crate) mod checks {\n    }\n";
    assert_eq!(production_half(src), "fn shipped() {}\n");
}

#[test]
fn a_file_without_a_test_module_is_returned_whole() {
    let src = "fn shipped() {}\n#[cfg(test)]\nconst ONLY_FOR_TESTS: u8 = 1;\n";
    assert_eq!(production_half(src), src);
}
