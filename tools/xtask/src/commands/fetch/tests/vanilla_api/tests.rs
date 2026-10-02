use super::*;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "fetch-vanilla-api-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(p.join("apps/mod/vanilla_reference/apidoc")).unwrap();
    fs::create_dir_all(p.join(".ai/tickets")).unwrap();
    fs::write(p.join(".ai/tickets/ROOT"), "{}").unwrap();
    p
}

fn seed_index(root: &Path) {
    fs::write(
        root.join("apps/mod/vanilla_reference/apidoc/annotated.html"),
        "<html>index</html>\n",
    )
    .unwrap();
}

#[test]
fn doxy_name_mangles_underscores() {
    assert_eq!(
        doxy_name("SCR_BaseGameMode"),
        "interfaceSCR__BaseGameMode.html"
    );
    assert_eq!(doxy_name("NoSuchClass"), "interfaceNoSuchClass.html");
}

#[test]
fn from_file_missing_arg_exits_2() {
    let root = scratch("fromfile-miss");
    seed_index(&root);
    let code = run(&root, &["--from-file".into()]).unwrap();
    assert_eq!(code, 2);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn index_miss_exits_1() {
    let root = scratch("index-miss");
    // Empty cache + stub curl via TBD_FETCH_VANILLA_API_CURL (not PATH — PATH races
    // under cargo test parallel threads and let real curl win → false green rc=0).
    let bin = root.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let stub = bin.join("curl");
    fs::write(&stub, "#!/bin/bash\nexit 1\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&stub).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&stub, perms).unwrap();
    }
    let old = std::env::var_os("TBD_FETCH_VANILLA_API_CURL");
    // SAFETY: restored below; only this test sets the override.
    unsafe { std::env::set_var("TBD_FETCH_VANILLA_API_CURL", &stub) };
    let code = run(&root, &[]).unwrap();
    match old {
        Some(v) => unsafe { std::env::set_var("TBD_FETCH_VANILLA_API_CURL", v) },
        None => unsafe { std::env::remove_var("TBD_FETCH_VANILLA_API_CURL") },
    }
    assert_eq!(code, 1);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn from_file_nonexistent_continues_rc0() {
    let root = scratch("fromfile-nofile");
    seed_index(&root);
    let code = run(
        &root,
        &[
            "--from-file".into(),
            "/tmp/fetch-vanilla-api-does-not-exist-xyz.txt".into(),
        ],
    )
    .unwrap();
    assert_eq!(code, 0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn cache_hit_index_only_rc0() {
    let root = scratch("cache-hit");
    seed_index(&root);
    let code = run(&root, &[]).unwrap();
    assert_eq!(code, 0);
    let _ = fs::remove_dir_all(root);
}

/// The `--from-file` refusal prints ONE usage line and it names the command the operator must
/// retype, not an implementation path — a usage line that names something unrunnable sends the
/// reader nowhere.
#[test]
fn from_file_usage_line_names_the_runnable_command() {
    assert_eq!(USAGE_COMMAND, "cargo xtask fetch vanilla-api");
    assert_eq!(
        format!("usage: {USAGE_COMMAND} --from-file <path>"),
        "usage: cargo xtask fetch vanilla-api --from-file <path>"
    );
}
