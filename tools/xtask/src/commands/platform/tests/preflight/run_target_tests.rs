use super::*;
use crate::commands::platform::wave_execution::{RunStamp, write_run_stamp};
use std::os::unix::fs::PermissionsExt;

const HEAD: &str = "4b2cca4a5880ee8a0e8fbcbbedd476534db5b0ac";
const OTHER: &str = "1f486e5721f906619259768b8ae7b7ebfd625fb9";

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Tmp {
        let p = env::temp_dir().join(format!(
            "tbd-t300-pf-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).expect("mkdir scratch");
        Tmp(p)
    }
    /// A run target holding `bins` executables in `profile`, and the stamp when given.
    fn run_target(&self, profile: &str, bins: &[&str], stamp: Option<RunStamp>) -> PathBuf {
        let run = self.0.join("run-main");
        let d = run.join(profile);
        fs::create_dir_all(&d).expect("mkdir profile");
        for b in bins {
            let p = d.join(b);
            fs::write(&p, b"elf").expect("write bin");
            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).expect("chmod");
        }
        if let Some(s) = stamp {
            write_run_stamp(&d, &s).expect("stamp");
        }
        run
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stamp(sha: &str, checkout: &Path) -> RunStamp {
    RunStamp {
        sha: sha.to_string(),
        checkout: checkout.display().to_string(),
    }
}

#[test]
fn a_run_target_that_was_never_built_is_green() {
    let t = Tmp::new("empty");
    let run = t.0.join("run-main");
    let st = run_target_state(&run, HEAD, &t.0);
    assert_eq!(st, RunTargetState::Empty);
    assert!(!run_target_detail(&st, &run, HEAD).0);
}

#[test]
fn binaries_built_from_head_in_this_checkout_are_green() {
    let t = Tmp::new("fresh");
    let run = t.run_target("debug", &["api"], Some(stamp(HEAD, &t.0)));
    let st = run_target_state(&run, HEAD, &t.0);
    assert_eq!(
        st,
        RunTargetState::Fresh {
            profile: "debug".into(),
            bins: 1
        }
    );
    let (block, detail) = run_target_detail(&st, &run, HEAD);
    assert!(!block, "{detail}");
    assert!(detail.contains("4b2cca4"), "{detail}");
}

/// THE TICKET'S OWN PERTURBATION, as a unit: a stamp whose sha is not HEAD must go red and
/// the message must name the binary, the sha it came from and HEAD.
#[test]
fn a_stamp_that_disagrees_with_head_blocks_and_names_the_binary() {
    let t = Tmp::new("stale");
    let run = t.run_target("debug", &["api", "world"], Some(stamp(OTHER, &t.0)));
    let st = run_target_state(&run, HEAD, &t.0);
    let (block, detail) = run_target_detail(&st, &run, HEAD);
    assert!(block, "a wrong sha read as green: {detail}");
    assert!(detail.contains("api,world"), "{detail}");
    assert!(detail.contains("1f486e5"), "{detail}");
    assert!(detail.contains("4b2cca4"), "{detail}");
    assert!(detail.contains("cargo clean --target-dir"), "{detail}");
}

/// THE WAVE-1 INCIDENT, as a unit: right sha, wrong checkout. This is the case a sha-only
/// comparison passes — a worktree sitting on the same commit as main still holds UNCOMMITTED
/// slice code, which is precisely what `make api` served on :8080.
#[test]
fn a_stamp_from_a_worktree_at_the_same_sha_still_blocks_and_names_the_checkout() {
    let t = Tmp::new("foreign");
    let wt = t.0.join(".ai/artifacts/worktrees/T-300");
    let run = t.run_target("debug", &["api"], Some(stamp(HEAD, &wt)));
    let st = run_target_state(&run, HEAD, &t.0);
    let (block, detail) = run_target_detail(&st, &run, HEAD);
    assert!(block, "a foreign checkout read as green: {detail}");
    assert!(detail.contains("worktrees/T-300"), "{detail}");
}

/// Fail closed. Binaries with no stamp are the state every run target was in before this
/// ticket, and reporting that as green would make the whole check decorative.
#[test]
fn binaries_with_no_stamp_block_rather_than_pass() {
    let t = Tmp::new("unstamped");
    let run = t.run_target("release", &["api"], None);
    let st = run_target_state(&run, HEAD, &t.0);
    let (block, detail) = run_target_detail(&st, &run, HEAD);
    assert!(block, "unstamped binaries read as green: {detail}");
    assert!(detail.contains("tbd-built-from"), "{detail}");
    assert!(detail.contains("release/api"), "{detail}");
}

/// A preflight that cannot resolve HEAD certifies nothing.
#[test]
fn an_unresolvable_head_blocks_rather_than_certifies() {
    let t = Tmp::new("nohead");
    let run = t.run_target("debug", &["api"], Some(stamp(HEAD, &t.0)));
    let st = run_target_state(&run, "", &t.0);
    let (block, detail) = run_target_detail(&st, &run, "");
    assert!(block, "empty HEAD read as green: {detail}");
    assert!(detail.contains("(unresolved)"), "{detail}");
}

/// Both profile directories are read: a release run binary must not hide behind an empty
/// debug one.
#[test]
fn the_release_profile_is_checked_too() {
    let t = Tmp::new("release");
    let run = t.run_target("release", &["api"], Some(stamp(OTHER, &t.0)));
    assert!(run_target_detail(&run_target_state(&run, HEAD, &t.0), &run, HEAD).0);
}

/// The reader/writer contract, asserted from the READER's side: what
/// [`crate::commands::platform::wave_execution::write_run_stamp`] emits is exactly what this module accepts, and an absent
/// stamp reads as unknown rather than as agreement.
#[test]
fn the_stamp_round_trips_and_an_absent_one_reads_as_unknown() {
    let t = Tmp::new("roundtrip");
    let d = t.0.join("debug");
    let s = stamp(HEAD, &t.0);
    assert_eq!(s.render(), format!("{HEAD} {}\n", t.0.display()));
    assert_eq!(
        crate::commands::platform::wave_execution::read_run_stamp(&d),
        None,
        "absent read as agreement"
    );
    write_run_stamp(&d, &s).expect("write");
    assert_eq!(
        crate::commands::platform::wave_execution::run_stamp_path(&d),
        d.join("tbd-built-from")
    );
    assert_eq!(
        crate::commands::platform::wave_execution::read_run_stamp(&d),
        Some(s)
    );
}

/// The check NAMES the binary, so this must find binaries and nothing else.
#[test]
fn run_binaries_lists_executables_and_skips_the_stamp_and_depfiles() {
    let t = Tmp::new("bins");
    let d = t.0.join("debug");
    fs::create_dir_all(d.join("deps")).expect("mkdir");
    for (name, mode) in [("api", 0o755), ("world", 0o755), ("notes", 0o644)] {
        let p = d.join(name);
        fs::write(&p, b"x").expect("write");
        fs::set_permissions(&p, fs::Permissions::from_mode(mode)).expect("chmod");
    }
    fs::write(d.join("api.d"), b"dep").expect("write");
    write_run_stamp(&d, &stamp(HEAD, &t.0)).expect("stamp");
    assert_eq!(
        run_binaries(&d),
        vec!["api".to_string(), "world".to_string()]
    );
}
