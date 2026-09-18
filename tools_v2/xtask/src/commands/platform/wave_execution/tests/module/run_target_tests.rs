use super::*;

const SHA: &str = "4b2cca4a5880ee8a0e8fbcbbedd476534db5b0ac";

fn v(s: &[&str]) -> Vec<String> {
    s.iter().map(|x| (*x).to_string()).collect()
}

/// THE FORMULA, and the disk claim with it. `run-main` must be a CHILD of the shared cache
/// and never the cache itself — collapse the two and every worktree's check/test/clippy
/// traffic is back in the run lane's fingerprint set with every other test still green. One
/// answer for every checkout is what makes it one extra target, not one per worktree.
#[test]
fn run_target_is_one_child_of_the_shared_cache_and_never_the_cache_itself() {
    let shared = "/home/Samuel/.cache/tbd-target";
    let run = run_target_dir_for(shared);
    assert_eq!(run, format!("{shared}/{RUN_TARGET_SUBDIR}"));
    assert_eq!(run, "/home/Samuel/.cache/tbd-target/run-main");
    assert_ne!(
        run, shared,
        "the run target collapsed onto the shared cache"
    );
    assert!(Path::new(&run).starts_with(shared));
    assert_eq!(
        run,
        run_target_dir_for(shared),
        "not one answer per checkout"
    );
}
/// Fail-closed. Every one of these is a shape a "tolerant" parser accepts, and each would
/// let preflight report agreement about a binary it cannot actually identify.
#[test]
fn stamp_parse_refuses_what_it_cannot_read() {
    for bad in [
        "",
        "\n",
        SHA,
        "/run/media/system/Disk_2 \n",
        "4b2cca4a5 /run/media/system/Disk_2",
        "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz /run/media",
        "4b2cca4a5880ee8a0e8fbcbbedd476534db5b0ac ",
    ] {
        assert_eq!(RunStamp::parse(bad), None, "accepted {bad:?}");
    }
}

#[test]
fn split_run_args_puts_program_arguments_only_on_run() {
    assert_eq!(
        split_run_args(&v(&[
            "-p",
            "developer-tools",
            "--bin",
            "world",
            "--",
            "reclassify"
        ])),
        (
            v(&["-p", "developer-tools", "--bin", "world"]),
            v(&["reclassify"])
        )
    );
    assert_eq!(
        split_run_args(&v(&["-p", "website-api", "--bin", "api"])),
        (v(&["-p", "website-api", "--bin", "api"]), v(&[]))
    );
}

/// THE MAIN GOAL, as a unit: a worktree may not build into the run target at all, the main
/// checkout may, and the refusal names both trees so the operator knows which one to look at.
#[test]
fn run_lane_refuses_a_worktree_and_names_both_checkouts() {
    let main = Path::new("/repo");
    let wt = Path::new("/repo/.ai/artifacts/worktrees/T-300");
    let args = v(&["-p", "website-api"]);
    let text = run_lane_refusal(wt, main, "/cache/run-main", &args)
        .expect("must refuse")
        .join("\n");
    assert!(text.contains("REFUSING"), "{text}");
    assert!(
        text.contains("/repo/.ai/artifacts/worktrees/T-300"),
        "{text}"
    );
    assert!(text.contains("main checkout = /repo"), "{text}");
    assert!(text.contains("/cache/run-main"), "{text}");
    assert_eq!(run_lane_refusal(main, main, "/cache/run-main", &args), None);
}

#[test]
fn run_lane_refuses_an_empty_argv_and_a_caller_supplied_target_dir() {
    let main = Path::new("/repo");
    assert!(run_lane_refusal(main, main, "/cache/run-main", &[]).is_some());
    for bad in [
        "--target-dir",
        "--target-dir=/tmp/x",
        "CARGO_TARGET_DIR=/tmp/x",
    ] {
        let args = v(&["-p", "website-api", bad]);
        let lines = run_lane_refusal(main, main, "/cache/run-main", &args)
            .unwrap_or_else(|| panic!("accepted {bad}"));
        assert!(lines.join("\n").contains(bad));
    }
}
