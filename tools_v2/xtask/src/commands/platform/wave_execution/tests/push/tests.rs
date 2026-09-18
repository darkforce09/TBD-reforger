use super::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// A stand-in for `git check-attr --cached -z --stdin filter`, in the ONE shape that decides
/// whether T-943 reproduces: it answers INCREMENTALLY. It reads a chunk of stdin, emits
/// `<path>\0filter\0unspecified\0` for every complete path in it, and loops — so its stdout
/// fills while its stdin is still being written, which is what real check-attr does and what
/// makes a single-threaded fill-then-drain caller deadlock.
///
/// A stub rather than real check-attr on purpose: real git could one day buffer differently and
/// the regression pin would quietly stop reproducing anything — the vacuous-pass shape this
/// program keeps finding. This one is pinned by construction.
///
/// The 60 s watchdog is hygiene: when the parent deadlocks, its assertion fires at 30 s and
/// this process must not survive as an orphan holding a pipe.
const STUB_SRC: &str = r#"
use std::io::{Read, Write};

fn main() {
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(60));
        std::process::exit(0);
    });
    let mut stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = match stdin.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        buf.extend_from_slice(&chunk[..n]);
        while let Some(i) = buf.iter().position(|b| *b == 0) {
            let path: Vec<u8> = buf.drain(..=i).collect();
            let path = &path[..path.len() - 1];
            if path.is_empty() {
                continue;
            }
            if out.write_all(path).is_err() {
                return;
            }
            if out.write_all(b"\0filter\0unspecified\0").is_err() {
                return;
            }
        }
        if out.flush().is_err() {
            return;
        }
    }
}
"#;

/// The `rustc` that built THIS test, wherever possible.
///
/// PATH last, not first: a stub built by some other toolchain's rustc would still run and the
/// test would still pass, and it would be reporting on something other than this build.
fn rustc() -> PathBuf {
    if let Some(p) = std::env::var_os("RUSTC") {
        return PathBuf::from(p);
    }
    // Cargo bakes its own path in at compile time; under rustup rustc is its sibling.
    if let Some(sib) = Path::new(env!("CARGO")).parent().map(|d| d.join("rustc")) {
        if sib.is_file() {
            return sib;
        }
    }
    PathBuf::from("rustc")
}

/// Compile [`STUB_SRC`] into a private temp dir and return the binary.
fn build_stub(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tbd-t943-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("stub temp dir");
    let src = dir.join("stub_check_attr.rs");
    std::fs::write(&src, STUB_SRC).expect("write stub source");
    let bin = dir.join("stub_check_attr");
    let out = Command::new(rustc())
        .args(["--edition", "2021", "-O", "-o"])
        .arg(&bin)
        .arg(&src)
        .output()
        .expect("rustc must be runnable to build the stub check-attr");
    assert!(
        out.status.success(),
        "rustc could not build the stub check-attr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    bin
}

/// `git diff-tree -z --name-only` shape: NUL-separated, trailing NUL included.
fn path_list(n: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..n {
        v.extend_from_slice(
            format!("packages/map-assets/everon/prefabs/blas/obj{i:06}.bvh").as_bytes(),
        );
        v.push(0);
    }
    v
}

/// A Linux pipe holds 64 KB, both directions.
const PIPE_CAPACITY: usize = 64 * 1024;

/// `filter\0unspecified\0` — what the stub appends to every path it answers.
const ANSWER_OVERHEAD: usize = 19;

/// The smallest stdin list that can wedge a fill-then-drain caller.
///
/// THIS IS THE NUMBER THE FIRST VERSION OF THIS TEST GOT WRONG, and the perturbation caught it:
/// 2,000 paths (108 KB) passed the PERTURBED build — a green regression pin over the very
/// defect it was written for. "Bigger than one pipe buffer" is not the condition. The caller
/// only blocks once it has written everything the child can absorb, and the child absorbs the
/// 64 KB sitting in its stdin pipe PLUS every path it consumed before its own stdout pipe
/// filled — `PIPE_CAPACITY / per_out` paths, worth `× per_in` bytes of input. Below that sum
/// the whole list fits and both sides finish, defect or no defect.
fn deadlock_threshold(per_in: usize) -> usize {
    PIPE_CAPACITY + (PIPE_CAPACITY / (per_in + ANSWER_OVERHEAD)) * per_in
}

/// THE T-943 REGRESSION PIN.
///
/// The assertion is a receive timeout because that is the only honest red for a deadlock: the
/// unfixed code does not fail, it never returns. MEASURED with the inline `write_all` restored:
/// no output, killed at 30 s, red.
#[test]
fn a_path_list_larger_than_the_pipe_buffer_does_not_deadlock() {
    let bin = build_stub("deadlock");
    let n = 20_000usize;
    let list = path_list(n);
    let bytes = list.len();
    let threshold = deadlock_threshold(bytes / n);
    assert!(
        bytes > 4 * threshold,
        "a list of {bytes} bytes cannot wedge the pre-fix code (threshold {threshold}), so \
             this test would pass over the defect and prove nothing — see deadlock_threshold"
    );

    let (tx, rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let mut cmd = Command::new(&bin);
        let _ = tx.send(feed_and_capture(&mut cmd, &list).map(|o| o.stdout));
    });
    let out = match rx.recv_timeout(Duration::from_secs(30)) {
        Ok(Ok(out)) => out,
        Ok(Err(())) => panic!("the stub check-attr did not run"),
        Err(_) => panic!(
            "DEADLOCK: feed_and_capture did not finish within 30 s for a {bytes} byte list. \
                 The T-943 defect is back — stdin is being written before stdout is drained."
        ),
    };
    worker.join().expect("writer worker");

    // Finishing is only half of it. A caller that returned early with a truncated answer would
    // also "not deadlock", and lfs_paths_in_range reads a short answer as `nothing is LFS` —
    // the fail-OPEN direction this whole module exists to keep shut.
    let mut it = out.split(|b| *b == 0);
    let mut seen = 0usize;
    while let (Some(path), Some(name), Some(value)) = (it.next(), it.next(), it.next()) {
        assert_eq!(name, b"filter", "triple {seen} is not a filter answer");
        assert_eq!(value, b"unspecified", "triple {seen} has the wrong value");
        assert!(!path.is_empty(), "triple {seen} has an empty path");
        seen += 1;
    }
    assert_eq!(
        seen, n,
        "every path must come back; a short read is the same bug in a different mask"
    );
    let _ = std::fs::remove_dir_all(
        std::env::temp_dir().join(format!("tbd-t943-deadlock-{}", std::process::id())),
    );
}

/// The child's own failure must reach the caller as a failure, not as an empty answer.
#[test]
fn feed_and_capture_reports_a_child_that_exits_non_zero() {
    let mut cmd = Command::new("git");
    cmd.args(["t943-no-such-subcommand"]).stderr(Stdio::null());
    let out = feed_and_capture(&mut cmd, b"anything\0").expect("git spawns");
    assert!(
        !out.status.success(),
        "a failing child must not look like an empty LFS answer"
    );
}

/// `--no-verify` is the safety property; this pins exactly when it is carried.
///
/// Not a tautology over a bool: [`cmd_push`] pushes with THIS argv, so flipping either arm
/// here is a behaviour change and turns this red.
#[test]
fn push_plan_drops_no_verify_exactly_when_git_lfs_is_present() {
    let (mode, argv) = push_plan(true);
    assert_eq!(mode, "git-lfs present: normal push");
    assert_eq!(argv, ["push", "origin", "main"].as_slice());
    assert!(
        !argv.contains(&"--no-verify"),
        "with git-lfs installed the pre-push hook must run — that is what uploads the objects"
    );

    let (mode, argv) = push_plan(false);
    assert_eq!(mode, "git-lfs absent: guarded --no-verify push");
    assert_eq!(argv, ["push", "--no-verify", "origin", "main"].as_slice());
}

/// Only a zero exit is `true` — a probe that cannot run keeps the guard on.
///
/// Written against commands whose exit codes are fixed rather than against `git lfs version`
/// itself: this host HAS git-lfs, so asserting the absent case from the machine would assert
/// nothing here and something else in the container.
#[test]
fn probe_ok_is_true_only_for_a_zero_exit() {
    assert!(probe_ok(&mut Command::new("git"), &["--version"]));
    assert!(!probe_ok(
        &mut Command::new("git"),
        &["t943-no-such-subcommand"]
    ));
    assert!(!probe_ok(
        &mut Command::new("/nonexistent/t943-not-a-binary"),
        &[]
    ));
}
