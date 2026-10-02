//! The `--timeout` run deadline, kept in process.
//!
//! **Role:** parse `--timeout` and decide, at each turn of the two wait loops, whether the run's
//! deadline has expired or how long the loop may nap before it looks again.
//!
//! **Position:** used by `on_stop_signal.rs`: `wait_for_verdict` ends the boot wait when the
//! deadline passes before registration, and `tail_until_the_server_stops` runs the pre-stop hook
//! (`BootCtx::before_stop`) and then the normal stop through `kill_run` when it passes after.
//!
//! **Signals & state:** [`RunDeadline`] holds the launch instant; [`deadline_check`] and
//! [`parse_run_timeout`] are pure.
//!
//! **Invariants:** the deadline counts from the launch; an empty `--timeout` or a zero duration
//! means no deadline (the `timeout(1)` convention); a nap never runs past the deadline.

use std::time::{Duration, Instant};

/// What a wait loop does next against the run deadline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum DeadlineCheck {
    /// The deadline has passed: stop now.
    Expired,
    /// Nap this long, then look again.
    NapFor(Duration),
}

/// The deadline decision: `run_timeout` since the launch, `elapsed` so far, `poll` the loop's
/// usual pause.
pub(super) fn deadline_check(
    run_timeout: Option<Duration>,
    elapsed: Duration,
    poll: Duration,
) -> DeadlineCheck {
    match run_timeout {
        None => DeadlineCheck::NapFor(poll),
        Some(limit) if elapsed >= limit => DeadlineCheck::Expired,
        Some(limit) => DeadlineCheck::NapFor(poll.min(limit - elapsed)),
    }
}

/// `--timeout=<duration>`: a non-negative number with an optional `s`, `m`, `h` or `d` suffix, as
/// `timeout(1)` reads it. Empty or zero is no deadline.
pub(super) fn parse_run_timeout(text: &str) -> Result<Option<Duration>, String> {
    if text.is_empty() {
        return Ok(None);
    }
    let (number, unit_seconds) = match text.char_indices().last() {
        Some((at, 's')) => (&text[..at], 1.0),
        Some((at, 'm')) => (&text[..at], 60.0),
        Some((at, 'h')) => (&text[..at], 3600.0),
        Some((at, 'd')) => (&text[..at], 86400.0),
        _ => (text, 1.0),
    };
    let value: f64 = number
        .parse()
        .ok()
        .filter(|value: &f64| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| format!("--timeout={text} is not a duration (seconds, or 30s/5m/1h/1d)"))?;
    let seconds = value * unit_seconds;
    Ok((seconds > 0.0).then(|| Duration::from_secs_f64(seconds)))
}

/// The run's deadline, counted from the launch.
pub(super) struct RunDeadline {
    run_timeout: Option<Duration>,
    launched: Instant,
}

impl RunDeadline {
    /// A deadline of `run_timeout` from now.
    pub(super) fn starting_now(run_timeout: Option<Duration>) -> Self {
        Self {
            run_timeout,
            launched: Instant::now(),
        }
    }

    /// [`deadline_check`] at this instant.
    pub(super) fn check(&self, poll: Duration) -> DeadlineCheck {
        deadline_check(self.run_timeout, self.launched.elapsed(), poll)
    }

    /// The configured limit in whole seconds, for the operator's line.
    pub(super) fn limit_seconds(&self) -> u64 {
        self.run_timeout.map(|limit| limit.as_secs()).unwrap_or(0)
    }
}
