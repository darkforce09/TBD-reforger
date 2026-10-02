use super::*;

/// Container ids differ per creation; nothing else may be normalised away.
#[test]
fn norm_only_erases_ids() {
    let id = "a".repeat(64);
    assert_eq!(norm(&format!("started {id}")), "started <ID>");
    assert_eq!(norm("Error: no such file"), "Error: no such file");
}

/// make's decoration must be recognised exactly — and a recipe line that merely mentions an
/// error must not be mistaken for it, or arm 6 would drop real output from the make side.
#[test]
fn make_error_lines_are_told_from_recipe_output() {
    assert_eq!(
        make_error_rc("make: *** [Makefile:70: db-up] Error 255"),
        Some(Some(255))
    );
    assert_eq!(
        make_error_rc("make: [Makefile:205: rust-test-it] Error 127 (ignored)"),
        Some(None)
    );
    assert_eq!(make_error_rc("Error: executing podman-compose: 255"), None);
    assert_eq!(make_error_rc("make: nothing to be done"), None);
}
