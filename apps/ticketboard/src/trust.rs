//! Trust banner model (T-915.3 §UI shape) — pure, unit-tested, no egui types.
//!
//! The banner is the result of `cargo xtask ticket check --strict` run as a
//! streamed subprocess (spawned in its alias-expanded form,
//! `cargo run --package xtask -- ticket check --strict`): the app NEVER
//! re-implements check, it invokes it. Green/red plus exit code, labeled STRICT —
//! the mutator preflight is non-strict (`require_check_ok` passes
//! `strict = false`), so banner-red does not always mean mutations refuse; the UI
//! never conflates the two.
//!
//! Phase split: cargo rebuilds xtask whenever the tree changed, and xtask is the
//! heavy bin — the first check after a `git pull` is a multi-minute compile that
//! must be surfaced honestly, not hidden behind a generic spinner. The heuristic
//! lives in [`phase_after`].

/// The one command the banner reports — shown to the operator verbatim.
pub const CHECK_COMMAND: &str = "cargo xtask ticket check --strict";

/// The alias-expanded argv actually spawned (`.cargo/config.toml`:
/// `xtask = "run --package xtask --"`) — byte-equivalent to [`CHECK_COMMAND`]
/// without depending on alias resolution.
pub const CHECK_ARGS: [&str; 7] = [
    "run",
    "--package",
    "xtask",
    "--",
    "ticket",
    "check",
    "--strict",
];

/// One-line doc-tooltip on the STRICT label (design §UI shape Trust banner).
pub const STRICT_TOOLTIP: &str = "runs `cargo xtask ticket check --strict`. The mutator \
     preflight is non-strict — banner-red does not always mean mutations refuse.";

/// In-flight phase of a check run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunPhase {
    /// Everything before proof that the xtask binary is running is cargo build
    /// noise ("building xtask…").
    Building,
    /// The binary is running ("checking…").
    Checking,
}

/// A finished run — the banner's green/red surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Process exit code; `None` means killed by a signal.
    pub code: Option<i32>,
    /// `ERROR: ` lines counted over the FULL stream (not just the retained ring).
    pub error_count: usize,
    /// Completion wall time, `"HH:MM:SS UTC"`.
    pub at: String,
    /// Set when the spawn itself failed — the check never ran.
    pub spawn_error: Option<String>,
}

impl Outcome {
    pub fn green(&self) -> bool {
        self.code == Some(0) && self.spawn_error.is_none()
    }
}

/// Banner tone — pure model; app.rs maps it to colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Busy,
    Green,
    Red,
}

// ---- line classification (the phase-split heuristic) ----

/// A line the check binary itself prints, as opposed to cargo build noise:
/// `check OK` (stdout, exit 0) or an `ERROR: ` line (stderr, exit 1) — see
/// `xtask/src/check.rs::cmd_check`.
pub fn is_check_output(line: &str) -> bool {
    line == "check OK" || is_error_line(line)
}

/// `cmd_check` prints one `ERROR: {e}` line per failure to stderr.
pub fn is_error_line(line: &str) -> bool {
    line.starts_with("ERROR: ")
}

/// Cargo's launch line (`     Running `target/…/xtask ticket check --strict``):
/// the build is over, the binary is starting.
fn is_cargo_launch(line: &str) -> bool {
    line.trim_start().starts_with("Running `")
}

/// The phase-split heuristic, applied per merged-stream line: stay `Building`
/// until the first line proving the build is over — cargo's ``Running ` ``
/// launch line (the normal transition; the strict check then runs silent for
/// seconds, visibly "checking…") or, as the fallback floor, actual check output
/// (`check OK` / `ERROR: ` — covers a quiet cargo). Process exit resolves the
/// run regardless, so a never-transitioning stream cannot wedge the banner.
pub fn phase_after(phase: RunPhase, line: &str) -> RunPhase {
    match phase {
        RunPhase::Building if is_cargo_launch(line) || is_check_output(line) => RunPhase::Checking,
        p => p,
    }
}

// ---- single-flight coalescer ----

/// Single-flight with a dirty flag: a trigger while a run is in flight is
/// remembered; when the run exits, exactly ONE follow-up starts — a burst of
/// triggers never queues a storm.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Coalescer {
    running: bool,
    dirty: bool,
}

impl Coalescer {
    /// A trigger arrived. `true` ⇒ start a run NOW; `false` ⇒ coalesced into the
    /// in-flight run's dirty flag.
    #[must_use]
    pub fn trigger(&mut self) -> bool {
        if self.running {
            self.dirty = true;
            false
        } else {
            self.running = true;
            true
        }
    }

    /// The in-flight run exited. `true` ⇒ start exactly one follow-up run
    /// (still single-flight); `false` ⇒ idle.
    #[must_use]
    pub fn finished(&mut self) -> bool {
        if self.dirty {
            self.dirty = false;
            true
        } else {
            self.running = false;
            false
        }
    }

    /// Test-only observer: the app tracks in-flight runs by `ProcHandle`
    /// presence; the state-machine tests assert single-flight through this.
    #[cfg(test)]
    pub fn running(&self) -> bool {
        self.running
    }
}

// ---- the banner's check state ----

/// Everything the banner needs about the strict check, fed line-by-line from the
/// subprocess stream. Pure — the app owns the `ProcHandle` and the `LogRing`.
#[derive(Debug, Default)]
pub struct CheckModel {
    /// `Some` while a run is in flight.
    pub run: Option<RunPhase>,
    /// `ERROR: ` lines seen in the CURRENT run (exact — counted as streamed).
    pub errors_so_far: usize,
    /// The last finished run; `None` until the first one completes.
    pub last: Option<Outcome>,
    pub coalescer: Coalescer,
}

impl CheckModel {
    /// A run just spawned.
    pub fn on_start(&mut self) {
        self.run = Some(RunPhase::Building);
        self.errors_so_far = 0;
    }

    /// One merged-stream line: advance the phase, count `ERROR: ` lines.
    pub fn on_line(&mut self, line: &str) {
        if let Some(phase) = self.run {
            self.run = Some(phase_after(phase, line));
        }
        if is_error_line(line) {
            self.errors_so_far += 1;
        }
    }

    /// The run exited (`code == None` ⇒ signal-killed).
    pub fn on_exit(&mut self, code: Option<i32>, at: String) {
        self.last = Some(Outcome {
            code,
            error_count: self.errors_so_far,
            at,
            spawn_error: None,
        });
        self.run = None;
    }

    /// The spawn failed — the check never ran.
    pub fn on_spawn_failed(&mut self, error: String, at: String) {
        self.last = Some(Outcome {
            code: None,
            error_count: 0,
            at,
            spawn_error: Some(error),
        });
        self.run = None;
    }

    /// The banner's headline + tone. States, in order: Idle/never-run →
    /// building xtask… → checking… → green/red with exit code + timestamp.
    pub fn banner(&self) -> (String, Tone) {
        match (self.run, &self.last) {
            (Some(RunPhase::Building), _) => ("building xtask…".to_owned(), Tone::Busy),
            (Some(RunPhase::Checking), _) => ("checking…".to_owned(), Tone::Busy),
            (None, None) => ("check not run yet".to_owned(), Tone::Neutral),
            (None, Some(outcome)) => outcome_label(outcome),
        }
    }
}

/// Green: `check OK — strict · exit 0 · HH:MM:SS UTC`. Red variants name the
/// exit honestly (code / killed / spawn failure) and never invent an error count.
fn outcome_label(o: &Outcome) -> (String, Tone) {
    if let Some(err) = &o.spawn_error {
        return (format!("check did not run — {err}"), Tone::Red);
    }
    if o.green() {
        return (
            format!("check OK — strict · exit 0 · {}", o.at),
            Tone::Green,
        );
    }
    let exit = match o.code {
        Some(code) => format!("exit {code}"),
        None => "killed".to_owned(),
    };
    let label = if o.error_count > 0 {
        format!(
            "check red — {} ERROR line(s) · {exit} · {}",
            o.error_count, o.at
        )
    } else {
        // Nonzero exit with zero ERROR: lines — e.g. the xtask BUILD failed.
        // Point at the verbatim output instead of claiming zero registry errors.
        format!(
            "check failed — {exit} (no ERROR: lines — see output) · {}",
            o.at
        )
    };
    (label, Tone::Red)
}

/// `"HH:MM:SS UTC"` from seconds since the Unix epoch — explicit-UTC on purpose:
/// no timezone dependency, and the registry's own timestamps are UTC.
pub fn utc_hms(secs_since_epoch: u64) -> String {
    let h = (secs_since_epoch / 3600) % 24;
    let m = (secs_since_epoch / 60) % 60;
    let s = secs_since_epoch % 60;
    format!("{h:02}:{m:02}:{s:02} UTC")
}

#[cfg(test)]
mod tests {
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
}
