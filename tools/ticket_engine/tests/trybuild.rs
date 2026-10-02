#[test]
fn compile_fail_mod_has_no_frontend_layer() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/mod_frontend.rs");
}
