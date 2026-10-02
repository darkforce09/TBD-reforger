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

/// The private clone carries the development compose file's own name, so `db up` pointed at the
/// clone's folder finds it through the same `-f` it passes in the checkout.
#[test]
fn the_scratch_clone_carries_the_development_compose_file_name() {
    let folder =
        std::env::temp_dir().join(format!("xtask-db-scratch-clone-{}", std::process::id()));
    write_scratch_compose(&folder).expect("write the scratch clone");
    let file_name = crate::core::repository_layout::DEVELOPMENT_COMPOSE_FILE
        .rsplit('/')
        .next()
        .expect("the layout constant names a file");
    let present: Vec<String> = fs::read_dir(&folder)
        .expect("read the scratch folder")
        .map(|entry| {
            entry
                .expect("scratch entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let _ = fs::remove_dir_all(&folder);
    assert_eq!(present, [file_name], "the scratch folder holds {present:?}");
}
