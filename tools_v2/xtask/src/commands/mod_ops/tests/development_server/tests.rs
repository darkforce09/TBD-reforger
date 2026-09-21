use super::*;
use std::fs;
use std::path::PathBuf;

fn throwaway(tag: &str) -> PathBuf {
    let root = PathBuf::from(format!(
        "{}/mod-dev-server-{tag}-{}",
        std::env::temp_dir().display(),
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".ai/tickets")).unwrap();
    fs::write(root.join(".ai/tickets/ROOT"), "{}\n").unwrap();
    root
}

/// An empty argument list is the only refusal this gate owns, and it is rc 2 — nothing on disk
/// can change that verdict, because the launcher it hands off to is linked into this binary.
#[test]
fn no_args_is_rc2() {
    let root = throwaway("noargs");
    let code = run_with_root(&root, &[]).unwrap();
    assert_eq!(code, 2);
    let _ = fs::remove_dir_all(&root);
}

/// The usage an operator reads on that refusal must be retypeable: it names the playtest command
/// and the two flags without which the server boots into a healthy-looking, unplayable state.
#[test]
fn usage_names_the_runnable_playtest_command() {
    let usage = usage();
    assert!(
        usage.contains("cargo xtask mod playtest --mission-id=<id> [--admin=<identityId>]"),
        "{usage:?}"
    );
    assert!(
        !usage.contains(".sh"),
        "usage names only runnable commands: {usage:?}"
    );
}
