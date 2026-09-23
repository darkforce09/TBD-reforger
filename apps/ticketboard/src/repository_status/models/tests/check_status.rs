use super::*;

/// The full-rebuild stream: cargo noise (flock wait, compiles, warnings,
/// Finished) stays Building; the `Running` launch line flips to Checking;
/// check output keeps it there.
#[test]
fn phase_split_on_a_full_build_stream() {
    let stream = [
        (
            "    Blocking waiting for file lock on build directory",
            RunPhase::Building,
        ),
        ("   Compiling serde v1.0.219", RunPhase::Building),
        ("warning: unused variable: `x`", RunPhase::Building),
        (
            "   Compiling xtask v0.1.0 (/repo/xtask)",
            RunPhase::Building,
        ),
        (
            "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 12s",
            RunPhase::Building,
        ),
        (
            "     Running `target/debug/xtask ticket check --strict`",
            RunPhase::Checking,
        ),
        ("check OK", RunPhase::Checking),
    ];
    let mut phase = RunPhase::Building;
    for (line, expected) in stream {
        phase = phase_after(phase, line);
        assert_eq!(phase, expected, "after line {line:?}");
    }
}

/// The fallback floor: a quiet cargo (nothing before the binary's own
/// output) still transitions on the first check-output line — `ERROR: ` or
/// `check OK`.
#[test]
fn phase_split_fallback_on_check_output_without_cargo_lines() {
    assert_eq!(
        phase_after(RunPhase::Building, "ERROR: T-9 parent missing"),
        RunPhase::Checking
    );
    assert_eq!(
        phase_after(RunPhase::Building, "check OK"),
        RunPhase::Checking
    );
    // Prose merely CONTAINING the markers does not flip the phase…
    assert_eq!(
        phase_after(RunPhase::Building, "note: check OK is printed on success"),
        RunPhase::Building
    );
    // …and Checking never regresses to Building.
    assert_eq!(
        phase_after(RunPhase::Checking, "   Compiling foo v0.1.0"),
        RunPhase::Checking
    );
}

#[test]
fn error_lines_counted_from_a_red_fixture_stream() {
    let mut model = CheckModel::default();
    model.on_start();
    for line in [
        "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.08s",
        "     Running `target/debug/xtask ticket check --strict`",
        "ERROR: T-915.9 status ready without order",
        "ERROR: wave.lock stale — run `cargo xtask wave repack`",
        "error[E0308]: mismatched types", // rustc shape — NOT a check ERROR: line
        "ERRORS: not the prefix either",
        "ERROR: third failure",
    ] {
        model.on_line(line);
    }
    assert_eq!(model.errors_so_far, 3);
    model.on_exit(Some(1), "10:00:00 UTC".to_owned());
    let last = model.last.as_ref().unwrap();
    assert_eq!(last.error_count, 3);
    assert!(!last.green());
    let (label, tone) = model.banner();
    assert_eq!(label, "check red — 3 ERROR line(s) · exit 1 · 10:00:00 UTC");
    assert_eq!(tone, Tone::Red);
}

#[test]
fn banner_states_walk_idle_building_checking_green() {
    let mut model = CheckModel::default();
    assert_eq!(
        model.banner(),
        ("check not run yet".to_owned(), Tone::Neutral)
    );
    assert!(model.coalescer.trigger());
    model.on_start();
    assert_eq!(model.banner(), ("building xtask…".to_owned(), Tone::Busy));
    model.on_line("     Running `target/debug/xtask ticket check --strict`");
    assert_eq!(model.banner(), ("checking…".to_owned(), Tone::Busy));
    model.on_line("check OK");
    model.on_exit(Some(0), "12:34:56 UTC".to_owned());
    assert!(!model.coalescer.finished());
    let (label, tone) = model.banner();
    assert_eq!(label, "check OK — strict · exit 0 · 12:34:56 UTC");
    assert_eq!(tone, Tone::Green);
    assert!(model.last.as_ref().unwrap().green());
}

#[test]
fn red_without_error_lines_points_at_the_output() {
    let mut model = CheckModel::default();
    model.on_start();
    model.on_line("error[E0433]: failed to resolve: use of undeclared crate");
    model.on_exit(Some(101), "09:00:00 UTC".to_owned());
    let (label, tone) = model.banner();
    assert_eq!(
        label,
        "check failed — exit 101 (no ERROR: lines — see output) · 09:00:00 UTC"
    );
    assert_eq!(tone, Tone::Red);
}

#[test]
fn killed_and_spawn_failed_are_red_and_honest() {
    let mut model = CheckModel::default();
    model.on_start();
    model.on_exit(None, "08:00:00 UTC".to_owned());
    let (label, tone) = model.banner();
    assert!(label.contains("killed"), "{label}");
    assert_eq!(tone, Tone::Red);

    model.on_start();
    model.on_spawn_failed(
        "No such file or directory (os error 2)".to_owned(),
        "08:01:00 UTC".to_owned(),
    );
    let (label, tone) = model.banner();
    assert_eq!(
        label,
        "check did not run — No such file or directory (os error 2)"
    );
    assert_eq!(tone, Tone::Red);
}

/// The single-flight contract: a burst coalesces to one in-flight run plus
/// exactly one follow-up; a quiet exit goes idle.
#[test]
fn coalescer_burst_yields_one_run_and_one_followup() {
    let mut c = Coalescer::default();
    assert!(c.trigger(), "idle trigger starts a run");
    assert!(c.running());
    assert!(!c.trigger(), "trigger during run coalesces");
    assert!(!c.trigger(), "…no matter how many arrive");
    assert!(c.finished(), "dirty exit starts EXACTLY one follow-up");
    assert!(c.running(), "the follow-up is in flight");
    assert!(!c.finished(), "clean exit goes idle");
    assert!(!c.running());
    // Idle again: the next trigger starts a fresh run.
    assert!(c.trigger());
    assert!(!c.finished());
}

#[test]
fn utc_hms_formats_and_wraps_days() {
    assert_eq!(utc_hms(0), "00:00:00 UTC");
    assert_eq!(utc_hms(45_296), "12:34:56 UTC");
    // Day boundary wraps; only time-of-day is shown.
    assert_eq!(utc_hms(86_400 + 61), "00:01:01 UTC");
}

#[test]
fn check_command_matches_its_expanded_argv() {
    // `cargo` + CHECK_ARGS must stay the alias expansion of CHECK_COMMAND
    // (`xtask = "run --package xtask --"` in .cargo/config.toml).
    assert_eq!(CHECK_COMMAND, "cargo xtask ticket check --strict");
    assert_eq!(
        CHECK_ARGS.join(" "),
        "run --package xtask -- ticket check --strict"
    );
}
