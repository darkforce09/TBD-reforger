use super::*;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "fetch-vanilla-source-{name}-{}-{}",
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

/// The curated default set is a fixed spine of nineteen entries; a silent addition or removal
/// changes what a bare `fetch vanilla-source` mirrors.
#[test]
fn curated_list_holds_nineteen_entries() {
    assert_eq!(CURATED.len(), 19);
}

/// The `--grep` refusal prints ONE usage line and it names the command the operator must retype,
/// not an implementation path — a usage line that names something unrunnable sends the reader
/// nowhere.
#[test]
fn grep_usage_line_names_the_runnable_command() {
    assert_eq!(USAGE_COMMAND, "cargo xtask fetch vanilla-source");
    assert_eq!(
        format!("usage: {USAGE_COMMAND} --grep <pattern>"),
        "usage: cargo xtask fetch vanilla-source --grep <pattern>"
    );
}
