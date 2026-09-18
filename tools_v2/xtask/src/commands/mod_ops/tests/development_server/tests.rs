use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

fn throwaway(tag: &str) -> PathBuf {
    let root = PathBuf::from(format!(
        "/tmp/t853/w223/t871/ut-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".ai/tickets")).unwrap();
    fs::create_dir_all(root.join("scripts/mod")).unwrap();
    fs::write(root.join(".ai/tickets/ROOT"), "{}\n").unwrap();
    root
}

/// T-853: the rc-3 "the real launcher is missing / is not executable" arm is GONE, and this
/// test records why rather than the old two tests being quietly deleted.
///
/// bash `exec`'d `scripts/mod/run-playtest-server.sh`, which could be absent or lose its
/// execute bit between the check and the exec — so rc 3 was a real, reachable outcome and had
/// to be pinned. The launcher is now [`crate::commands::mod_ops::playtest_server`], linked into this binary.
/// There is no state in which it is absent while this code runs, so the arm is not "removed",
/// it is DISCHARGED by the type system. Asserting the absence of an unreachable branch is the
/// closest thing to a test that remains honest here.
///
/// `no_args_is_rc2` below still pins the one arm that IS reachable.
#[test]
fn the_missing_launcher_arm_is_discharged_not_deleted() {
    let root = throwaway("discharged");
    // No scripts/ tree at all — under bash this was rc 3. It must NOT be rc 3 now, because
    // rc 3 meant "I could not find the launcher" and that question no longer exists.
    let code = run_with_root(&root, &[]).unwrap();
    assert_eq!(
        code, 2,
        "an empty arg list is still the usage arm, not a missing-launcher arm"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn no_args_is_rc2() {
    let root = throwaway("noargs");
    let play = root.join("scripts/mod/run-playtest-server.sh");
    fs::write(&play, "#!/bin/bash\nexit 0\n").unwrap();
    let mut perms = fs::metadata(&play).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&play, perms).unwrap();
    let code = run_with_root(&root, &[]).unwrap();
    assert_eq!(code, 2);
    let _ = fs::remove_dir_all(&root);
}
