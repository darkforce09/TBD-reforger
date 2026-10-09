use super::*;
use std::io::Write;

/// A scratch file that cleans itself up. Avoids a dev-dependency for six tests.
struct Tmp(std::path::PathBuf);
impl Tmp {
    fn new(name: &str, body: &str) -> Tmp {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "verification-core-gate-{}-{}",
            std::process::id(),
            name
        ));
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        Tmp(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn pat(p: &str) -> Pattern {
    Pattern::regex(p).unwrap()
}

#[test]
fn ban_fails_when_present() {
    let f = Tmp::new("ban-dirty", "some evil here\n");
    let v = ban("no evil", &pat("evil"), &[f.path()]);
    assert!(matches!(v, Verdict::Failed(_)));
    assert_eq!(v.to_string(), "FAIL: no evil");
}

#[test]
fn require_fails_when_absent() {
    let f = Tmp::new("req-missing", "nothing relevant\n");
    assert!(matches!(
        require("must pin", &pat("PINNED"), &[f.path()]),
        Verdict::Failed(_)
    ));
}

/// THE DEFECT THIS CRATE EXISTS FOR. A missing target must never read as a clean ban.
#[test]
fn missing_target_is_did_not_run_not_held() {
    let v = ban(
        "no evil",
        &pat("evil"),
        &[Path::new("/nonexistent/verification_core/nope")],
    );
    assert!(matches!(v, Verdict::DidNotRun(NotRun::TargetMissing(_), _)));
    assert_ne!(v.into_exit(), 0, "a check that did not run must not exit 0");
}

#[test]
fn probe_propagates_did_not_run_instead_of_short_circuiting_clean() {
    // The compound-condition footgun: an unreadable input must not answer `false`.
    let got: Result<bool, NotRun> = probe_files(&pat("x"), &[Path::new("/nonexistent/tbd/x")]);
    assert!(matches!(got, Err(NotRun::TargetMissing(_))));
}
