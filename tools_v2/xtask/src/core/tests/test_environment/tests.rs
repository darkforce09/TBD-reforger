use super::*;
use std::path::PathBuf;

#[test]
fn prepend_dir_keeps_usr_bin() {
    let _g = lock_env();
    let previous = std::env::var_os("PATH");
    // SAFETY: under ENV_LOCK; restored below.
    unsafe { std::env::set_var("PATH", "/tmp/only-stub-bin") };
    let stub = PathBuf::from("/tmp/t872-path-guard-stub");
    let _path = PathGuard::prepend_dir(&stub);
    let now = std::env::var("PATH").unwrap();
    assert!(now.starts_with("/tmp/t872-path-guard-stub:"));
    assert!(now.contains("/usr/bin"), "PATH must keep /usr/bin: {now}");
    assert!(now.contains("/bin"), "PATH must keep /bin: {now}");
    drop(_path);
    match previous {
        Some(p) => unsafe { std::env::set_var("PATH", p) },
        None => unsafe { std::env::remove_var("PATH") },
    }
}
