use super::*;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tools_v2/xtask has a parent")
        .parent()
        .expect("xtask parent")
        .to_path_buf()
}

#[test]
fn live_tree_holds() {
    assert_eq!(verify_t437(&repo()).unwrap(), 0);
}

#[test]
fn strip_keeps_block_newlines() {
    assert_eq!(strip_c_comments("a // x\nb"), "a \nb");
    assert_eq!(strip_c_comments("a /* x\ny */ b"), "a \n b");
}

#[test]
fn paraphrase_injection_is_caught() {
    let reg = std::fs::read_to_string(repo().join(REG_REL)).unwrap();
    let dirty = inject_paraphrase_lie(&reg).expect("inject");
    let mut failed = false;
    let clean = scan_forbidden(Path::new("/tmp/tmp.test"), &dirty, &mut failed).unwrap();
    assert!(!clean);
    assert!(failed);
}

#[test]
fn collapsed_returns_fail_registry_pins() {
    let reg = std::fs::read_to_string(repo().join(REG_REL)).unwrap();
    let dirty = collapse_diagnose_returns(&reg).expect("collapse");
    let mut failed = false;
    assert!(!assert_registry_pins(&dirty, "t", &mut failed).unwrap());
}
