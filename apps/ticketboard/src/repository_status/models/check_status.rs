//! Strict-check progress, coalesced reruns, and verbatim completion status.
//!
//! Only an observed successful exit is green. Build output, check output, process
//! failures, and cancellation remain distinct states.

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

/// One-line doc-tooltip on the STRICT label.
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

/// Banner tone — pure model; the status UI maps it to colors.
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
/// `ticket_engine::validation::cmd_check`.
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

    /// Test-only observer: the app tracks in-flight runs by `ProcessHandle`
    /// presence; the state-machine tests assert single-flight through this.
    #[cfg(test)]
    pub fn running(&self) -> bool {
        self.running
    }
}

// ---- the banner's check state ----

/// Everything the banner needs about the strict check, fed line-by-line from the
/// subprocess stream. Pure — the app owns the `ProcessHandle` and the `BoundedLog`.
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

pub use crate::core::time::utc_hms;

#[cfg(test)]
#[path = "tests/check_status.rs"]
mod tests;
