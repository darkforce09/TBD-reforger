use super::*;
use std::path::PathBuf;
use std::process::Command;

fn scratch(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "t862-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(p.join("apps/mod/vanilla_reference/source_html")).unwrap();
    fs::create_dir_all(p.join(".ai/tickets")).unwrap();
    fs::write(p.join(".ai/tickets/ROOT"), "{}").unwrap();
    p
}

#[test]
fn grep_missing_pattern_exits_2() {
    let root = scratch("grep-miss");
    let code = run(&root, &["--grep".into()]).unwrap();
    assert_eq!(code, 2);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn grep_empty_pattern_exits_2() {
    let root = scratch("grep-empty");
    let code = run(&root, &["--grep".into(), "".into()]).unwrap();
    assert_eq!(code, 2);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn empty_index_map_build_exits_1() {
    let root = scratch("empty-idx");
    let cache = root.join("apps/mod/vanilla_reference/source_html");
    fs::write(cache.join("files.html"), "no hrefs here\n").unwrap();
    // no map.tsv
    let code = run(&root, &["NoSuch.c".into()]).unwrap();
    assert_eq!(code, 1);
    // bash truncates map on failed pipeline
    assert!(cache.join("map.tsv").exists());
    assert_eq!(fs::metadata(cache.join("map.tsv")).unwrap().len(), 0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn help_is_filename_miss_rc0() {
    let root = scratch("help-miss");
    let cache = root.join("apps/mod/vanilla_reference/source_html");
    // minimal valid map so we don't hit empty-index
    fs::write(cache.join("files.html"), "x").unwrap();
    fs::write(
        cache.join("map.tsv"),
        "SCR_BaseGameMode.c\t_s_c_r___base_game_mode_8c_source.html\n",
    )
    .unwrap();
    let code = run(&root, &["--help".into()]).unwrap();
    assert_eq!(code, 0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn curated_list_len_matches_bash() {
    assert_eq!(CURATED.len(), 19);
}

/// Anti-vacuity: bash side must go red on the empty-index fixture before we trust parity.
#[test]
fn bash_empty_index_goes_red_first() {
    let sh =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/mod/fetch-vanilla-source.sh");
    // Script may already be deleted after land — skip if absent (post-delete unit run).
    if !sh.exists() {
        return;
    }
    let root = scratch("bash-red");
    let cache = root.join("apps/mod/vanilla_reference/source_html");
    fs::create_dir_all(root.join("scripts/mod")).unwrap();
    fs::copy(&sh, root.join("scripts/mod/fetch-vanilla-source.sh")).unwrap();
    fs::write(cache.join("files.html"), "no hrefs here\n").unwrap();
    let out = Command::new("bash")
        .arg(root.join("scripts/mod/fetch-vanilla-source.sh"))
        .arg("NoSuch.c")
        .output()
        .expect("bash");
    assert_ne!(
        out.status.code(),
        Some(0),
        "bash must go red on empty index"
    );
    assert_eq!(out.status.code(), Some(1));
    let _ = fs::remove_dir_all(root);
}
