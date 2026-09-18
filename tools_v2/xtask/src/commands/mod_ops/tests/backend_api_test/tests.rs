use super::*;

#[test]
fn paths_mirror_paths_sh() {
    let root = Path::new("/repo");
    let p = Paths::from_root(root);
    assert_eq!(p.web, PathBuf::from("/repo/apps/website/api_v2"));
    assert_eq!(p.mod_root, PathBuf::from("/repo/apps/mod"));
    assert_eq!(p.schema, PathBuf::from("/repo/contracts_v2"));
}

#[test]
fn clamp_code_passthrough() {
    assert_eq!(clamp_code(7), 7);
    assert_eq!(clamp_code(52), 52);
    assert_eq!(clamp_code(127), 127);
    assert_eq!(clamp_code(-1), 1);
}
