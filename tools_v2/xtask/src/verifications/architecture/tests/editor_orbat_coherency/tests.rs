use super::*;
use std::path::PathBuf;

fn repo() -> PathBuf {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    here.parent()
        .expect("tools_v2")
        .parent()
        .expect("repository root")
        .to_path_buf()
}

/// A scratch repo root holding copies of the five files the static half reads.
fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("tbd-t180-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for rel in [ORBAT_RS, SLOTS_GPU]
        .into_iter()
        .chain(ORBAT_UI_BAN_TARGETS.iter().copied())
        .chain(EDITOR_OPS_SPLIT.iter().copied())
    {
        let dst = root.join(rel);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(repo().join(rel), &dst).unwrap();
    }
    root
}

/// Run the static half over a perturbed tree and hand back the FAIL text bash would print.
fn red(root: PathBuf) -> String {
    let got = static_checks(&root).expect_err("this perturbation must go red");
    let _ = std::fs::remove_dir_all(&root);
    got
}

/// Append to a target, so a banned pattern appears where it must not.
fn red_append(name: &str, rel: &str, extra: &str, want: &str) {
    let root = scratch(name);
    let p = root.join(rel);
    std::fs::write(&p, std::fs::read_to_string(&p).unwrap() + extra).unwrap();
    assert_eq!(red(root), want);
}

/// Perturb `slots_gpu.rs` in place, so a pinned literal stops matching.
fn red_sub(name: &str, from: &str, to: &str, want: &str) {
    let root = scratch(name);
    let p = root.join(SLOTS_GPU);
    let body = std::fs::read_to_string(&p).unwrap();
    assert!(body.contains(from), "fixture text `{from}` is gone");
    std::fs::write(&p, body.replace(from, to)).unwrap();
    assert_eq!(red(root), want);
}

fn red_gone(name: &str, rel: &str) -> String {
    let root = scratch(name);
    std::fs::remove_file(root.join(rel)).unwrap();
    red(root)
}

#[test]
fn the_real_tree_holds() {
    assert_eq!(static_checks(&repo()), Ok(()));
}

/// Anti-vacuity: a gate that cannot fail checks nothing. One case per static arm,
/// asserted against the tables so the right row is pinned as having fired; the exact bash
/// wording — which is the diff contract — is asserted below.
#[test]
fn every_static_arm_can_go_red() {
    red_append("ban1", EDITOR_OPS, "\nensure_default_squad\n", BANS[0].0);
    red_append(
        "ban1-domain",
        "apps/website/map-engine/src/data/store/operations/entity/placement.rs",
        "\nensure_default_squad\n",
        BANS[0].0,
    );
    red_append(
        "ban1-host",
        "apps/website/frontend/src/v2/apps/editor/bridge/host_state/armed_placement/map_release.rs",
        "\nensure_default_squad\n",
        BANS[0].0,
    );
    red_append("ban2", ORBAT_RS, "\nloadout: String::new()\n", BANS[1].0);
    // `-i`, and in the SECOND target file: a ban over two files must read both of them.
    red_append("ban3", EDEN_CHROME, "\n// ifak pouch\n", BANS[2].0);
    red_append("ban3b", ORBAT_MGR, "\n// Grenade Complement\n", BANS[2].0);
    // One pin per side, perturbed three different ways: a value drift, a spacing drift (the
    // literal formatting is part of the lock) and a rename.
    red_sub("blufor", "173, 198, 255", "173, 198, 254", PINS[0].0);
    red_sub("opfor", "248, 113, 113,", "248,113,113,", PINS[1].0);
    red_sub("indfor", "SIDE_INDFOR_RGBA", "SIDE_INDEP_RGBA", PINS[2].0);
}

/// THE DEFECT THIS PORT INHERITS ITS EXISTENCE FROM. A target nobody read is not a clean
/// target — and the two sentences differ because a ban and a pin send a reader elsewhere.
#[test]
fn a_missing_target_never_reads_as_a_pass() {
    let head = BANS[0].0;
    let want = format!("{head} — target file missing: {EDITOR_OPS}. {BAN_MISSING}");
    assert_eq!(red_gone("gone-ban", EDITOR_OPS), want);
    // The second file of the two-file ban, so the loop's ORDER is pinned as well.
    let second = red_gone("gone-ban2", EDEN_CHROME);
    let want = format!("missing: {EDEN_CHROME}");
    assert!(second.contains(&want), "{second}");
    let tail = "The pin could not be checked.";
    let want = format!("{} — target file missing: {SLOTS_GPU}. {tail}", PINS[0].0);
    assert_eq!(red_gone("gone-pin", SLOTS_GPU), want);
}

/// The three `cargo_test_pin` arms, including the one where cargo exits 0 and nothing ran — plus
/// the two shapes that must still HOLD, so the classifier is not merely red on everything.
#[test]
fn every_cargo_pin_arm_can_go_red() {
    let label = "-p website-map-engine --lib zzz -- --quiet";
    let red = |status, out: &str| classify(label, status, out).unwrap_err();
    let empty = "\nrunning 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; \
                 0 measured; 277 filtered out; finished in 0.00s\n\n";
    let want = format!(
        "cargo test {label} — 0 tests passed (selector matched nothing). A renamed/typo'd pin \
         must not silently empty."
    );
    assert_eq!(red(0, empty), want);
    // A compile error: cargo exits non-zero and never reaches libtest. Reported first, as bash
    // reported it — "it did not build" outranks "it printed no result line".
    let boom = "error[E0433]: cannot find `mission` in `crate`\n";
    assert_eq!(red(101, boom), format!("cargo test {label} exited 101"));
    // Exit 0 and no result line at all: cargo said nothing, and silence is not a pass.
    let want = format!(
        "cargo test {label} — no 'test result: N passed' line. Refusing to report OK on a \
         check that did not execute."
    );
    assert_eq!(red(0, "Finished `test` profile\n"), want);
    let one = "test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 513 filtered out\n";
    assert_eq!(classify(label, 0, one), Ok(()));
    // Two result lines with `0 passed` first: `wc -l` sees 2 and `awk` sums to 17, so it HOLDS.
    let two = format!("test result: ok. 0 passed; 1 filtered out\n{one}");
    assert_eq!(passed_counts(&two), vec![0, 17]);
    assert_eq!(classify(label, 0, &two), Ok(()));
    // Lines without the `test result:` prefix are not counted, whatever else they claim.
    assert!(passed_counts("all 9 passed; nothing to see\n").is_empty());
}

/// `$*` loses the quoting around `--features "scenario store"`, and the failure text must too.
/// The pin table is also the gate's whole scope, and a silently shortened one is a silently
/// weakened gate: 25 rows and 5 section `ok` lines, exactly as the script had.
#[test]
fn the_argv_rendering_and_the_pin_table_match_the_script() {
    let args = [
        "test",
        "-p",
        MC,
        "--features",
        "scenario store",
        "--lib",
        "x",
    ];
    let want = "-p website-map-engine --features scenario store --lib x";
    assert_eq!(shown(&args), want);
    assert_eq!(CARGO_PINS.len(), 25);
    assert_eq!(CARGO_PINS.iter().filter(|p| p.4.is_some()).count(), 5);
    // The vehicle-floor pin must keep `store`, or it matches zero tests.
    let veh = CARGO_PINS
        .iter()
        .find(|p| p.3 == "the_vehicle_row_still_has_the_shape_this_module_reads");
    assert_eq!(veh.expect("the vehicle-floor pin is still listed").1, MSN);
    // No pin may ask for `store` without `scenario`.
    assert!(!CARGO_PINS.iter().any(|p| p.1 == Some("store")));
    // T-0xx Phase 2A: the crate default is `scenario` alone now. A map-engine graphics row
    // left on `NOF` would compile none of its modules and report "0 tests" as a pass.
    assert!(
        !CARGO_PINS
            .iter()
            .any(|p| p.0 == "website-map-engine" && p.1.is_none()),
        "every website-map-engine pin must name its feature tier"
    );
}

/// `2>&1` is one pipe, not two strings glued together — the interleaving is the contract. The
/// large case runs well past a 64 KiB pipe buffer on both streams at once: the six
/// website-frontend pins each replay ~110 lines of warnings, so a wedged capture is not
/// hypothetical. And bash reported an absent cargo as `exited 127` and a SIGKILLed one as
/// `exited 137`; neither is an exit code, and neither may reach a caller as a plain failure.
#[test]
fn merged_capture_keeps_order_and_never_invents_an_exit_code() {
    let tmp = Path::new("/tmp");
    let path = std::env::var("PATH").unwrap_or_default();
    // The local `merged()` this used to exercise now lives in the library as
    // `Run::merged_output`, so a second cargo-running port inherits it instead of re-deriving
    // it. The assertions stay here because THIS gate is the one whose 803-line diff depends
    // on them.
    let sh = |script: &str| {
        verification_core::proc::Run::new("sh")
            .arg("-c")
            .arg(script)
            .cwd(tmp)
            .env("PATH", &path)
            .merged_output()
    };
    let m = sh("echo one; echo two >&2; echo three; exit 7").unwrap();
    assert_eq!(m.code, 7, "raw exit codes must never be collapsed");
    assert_eq!(m.text, "one\ntwo\nthree\n");
    let big = sh("seq 1 40000; seq 1 40000 >&2").unwrap();
    assert_eq!(big.code, 0);
    assert_eq!(big.text.lines().count(), 80000);
    let absent = verification_core::proc::Run::new("tbd-not-a-real-program-t180")
        .cwd(tmp)
        .env("PATH", "")
        .merged_output();
    assert!(matches!(absent, Err(NotRun::ToolAbsent(_))));
    let clause = not_run_clause(&absent.unwrap_err());
    assert!(clause.contains("is ABSENT"), "{clause}");
    assert!(clause.ends_with("did not execute."), "{clause}");
    match sh("kill -9 $$") {
        Err(NotRun::Signalled { signal, .. }) => assert_eq!(signal, 9),
        other => panic!("expected Signalled, got {other:?}"),
    }
}
