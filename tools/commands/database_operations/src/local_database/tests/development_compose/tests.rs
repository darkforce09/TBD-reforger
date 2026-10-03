use super::*;
use crate::local_database::{SEEDS, seed_file};
use tool_test_support::test_repo_root;

/// Every compose call names the layout's compose file with `-f` and runs in the folder that holds
/// it, which is the folder compose resolves the file's relative paths against.
#[test]
fn the_checkout_project_runs_compose_on_the_layout_file_in_its_folder() {
    let root = Path::new("/checkout");
    let project = ComposeProject::in_checkout(root);
    let (_, file_name) = compose_file_parts();
    assert_eq!(
        project.folder.join(file_name),
        root.join(DEVELOPMENT_COMPOSE_FILE)
    );
    assert_eq!(
        compose_argv(&["podman".to_string()], &["up", "-d", "db"]),
        ["podman", "compose", "-f", file_name, "up", "-d", "db"]
    );
    assert_eq!(
        compose_argv(
            &["distrobox-host-exec".to_string(), "podman".to_string()],
            &["down"]
        ),
        [
            "distrobox-host-exec",
            "podman",
            "compose",
            "-f",
            file_name,
            "down"
        ]
    );
}

/// The echoed line is the shell form of the call: it enters the compose file's folder and names
/// the file, and the runtime appears without its bridge prefix.
#[test]
fn the_echoed_line_enters_the_compose_folder_and_names_the_file() {
    assert_eq!(
        ComposeLine::of_checkout().render("podman", &["up", "-d", "db"], None),
        "cd deploy && podman compose -f compose.dev.yml up -d db"
    );
}

/// A seed read on stdin is shown as the path from the compose folder to the API's seeds, and the
/// runner opens that same path, so the line run by a shell reads the file the command read.
#[test]
fn every_seed_is_opened_through_the_path_its_line_shows() {
    let root = test_repo_root();
    let project = ComposeProject::in_checkout(&root);
    for file in SEEDS {
        let from_root = seed_file(file);
        let line = project.shown.render("podman", &["exec"], Some(&from_root));
        assert!(
            line.ends_with(&format!(" < ../apps/api/seeds/{file}")),
            "{line}"
        );
        let opened = project.stdin_file(&from_root);
        assert!(opened.is_file(), "no seed at {}", opened.display());
        assert_eq!(
            std::fs::canonicalize(&opened).expect("canonical seed path"),
            std::fs::canonicalize(root.join(&from_root)).expect("canonical root path"),
        );
    }
}

/// The layout's compose file defines the `db` service as the `tbd_reforger_db` container on host
/// port 5434, the container `db test-it`, the backups and the checksum repair reach by name.
#[test]
fn the_layout_compose_file_defines_the_development_database() {
    let text = std::fs::read_to_string(test_repo_root().join(DEVELOPMENT_COMPOSE_FILE))
        .expect("the development compose file is committed");
    for needle in [
        "\nservices:\n  db:\n",
        "container_name: tbd_reforger_db",
        "\"5434:5432\"",
    ] {
        assert!(
            text.contains(needle),
            "{DEVELOPMENT_COMPOSE_FILE} lacks {needle:?}"
        );
    }
}

/// The folder override keeps the file name, shows the folder as given, and shows a stdin file by
/// its absolute path, which every folder reaches.
#[test]
fn the_folder_override_keeps_the_file_name_and_shows_the_folder_as_given() {
    let relative = ComposeProject::in_override_folder(
        "target/db-selftest",
        Path::new("/work"),
        Path::new("/checkout"),
    );
    assert_eq!(relative.folder, Path::new("/work/target/db-selftest"));
    assert_eq!(
        relative.shown.render("podman", &["down"], None),
        "cd target/db-selftest && podman compose -f compose.dev.yml down"
    );
    assert_eq!(
        relative.stdin_file("apps/api/seeds/x.sql"),
        Path::new("/checkout/apps/api/seeds/x.sql")
    );
    let absolute = ComposeProject::in_override_folder(
        "/elsewhere/project",
        Path::new("/work"),
        Path::new("/checkout"),
    );
    assert_eq!(absolute.folder, Path::new("/elsewhere/project"));
}
