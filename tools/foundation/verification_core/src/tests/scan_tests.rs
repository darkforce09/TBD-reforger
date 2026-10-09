use super::*;

struct TmpDir(PathBuf);
impl TmpDir {
    fn new(name: &str) -> TmpDir {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "verification-core-scan-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        TmpDir(p)
    }
    fn file(&self, rel: &str, body: &str) -> PathBuf {
        let p = self.0.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, body).unwrap();
        p
    }
}
impl Drop for TmpDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_missing_root_is_did_not_run_not_zero_hits() {
    // THE DEFECT. A search whose error is silenced reads a renamed directory as "clean".
    let got = walk_files(&[Path::new("/nonexistent/verification_core/scan")], |_| {
        true
    });
    assert!(matches!(got, Err(NotRun::TargetMissing(_))));
}

#[test]
fn walks_recursively_and_deterministically() {
    let d = TmpDir::new("walk");
    d.file("a.rs", "");
    d.file("sub/b.rs", "");
    d.file("sub/deep/c.rs", "");
    let files = walk_files(&[&d.0], |_| true).unwrap();
    assert_eq!(files.len(), 3);
    let mut sorted = files.clone();
    sorted.sort();
    assert_eq!(files, sorted, "order must not depend on readdir");
}

#[test]
fn matching_lines_reports_one_based_line_numbers() {
    let d = TmpDir::new("grep");
    let f = d.file("x.rs", "first\nSELECT * FROM users\nthird\n");
    let hits = matching_lines(
        &Pattern::regex("SELECT \\* FROM").unwrap(),
        std::slice::from_ref(&f),
    )
    .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].line_no, 2, "line numbers are 1-based");
    assert_eq!(hits[0].line, "SELECT * FROM users");
    assert!(hits[0].rendered().ends_with(":2:SELECT * FROM users"));
}

#[test]
fn non_utf8_bytes_do_not_abort_the_scan() {
    // A stray latin-1 byte in a source file must not make the gate unable to run.
    let d = TmpDir::new("binary");
    let p = d.0.join("odd.rs");
    std::fs::write(&p, [b'h', b'i', 0xff, b'\n', b'x', b'\n']).unwrap();
    let hits = matching_lines(&Pattern::literal("x"), &[p]).unwrap();
    assert_eq!(hits.len(), 1);
}
