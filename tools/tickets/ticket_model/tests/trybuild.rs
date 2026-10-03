//! Compile-fail tests: `fail/mod_frontend.rs` names a `Domain::Frontend` variant and passes
//! only while it fails to compile with the error recorded in `fail/mod_frontend.stderr`, which
//! keeps the scope `Domain` enum closed.

#[test]
fn compile_fail_mod_has_no_frontend_layer() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/mod_frontend.rs");
}
