use super::*;

#[test]
fn cargo_xtask_is_allowlisted() {
    assert_eq!(line_reason("cargo xtask verify no-shell"), None);
}

#[test]
fn git_lfs_pull_is_allowlisted() {
    assert_eq!(
        line_reason("git lfs pull --include assets_v2/terrains/everon/**"),
        None
    );
}

#[test]
fn unpinned_cargo_install_is_not_allowlisted() {
    let r = line_reason("cargo install editorconfig-checker").unwrap();
    assert!(r.contains("allowlist"), "{r}");
}

#[test]
fn pipe_is_forbidden_even_on_xtask() {
    assert_eq!(
        line_reason("cargo xtask verify no-shell | tee log").as_deref(),
        Some("pipe")
    );
}

#[test]
fn ampersand_background_is_red() {
    assert_eq!(
        line_reason("cargo xtask verify no-shell & echo pwned").as_deref(),
        Some("`&`")
    );
}

#[test]
fn double_ampersand_still_reports_and_and() {
    assert_eq!(
        line_reason("cargo xtask verify no-shell && true").as_deref(),
        Some("`&&`")
    );
}
