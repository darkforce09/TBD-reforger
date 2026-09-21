//! `verification-core` — the four-outcome static-check library.
//!
//! Every quality gate in the repository is written against this crate: one implementation of what
//! a check can conclude, shared by all of them, so the next gate cannot be born with the hole
//! that a search which never ran reads as a pass.
//!
//! ── THE THREE DEFECT CLASSES IT MAKES UNREPRESENTABLE ────────────────────────────────────────
//!
//! 1. **"The check did not run" folded into "the check passed."** [`Verdict`] has no `bool`
//!    conversion of any kind, so the four outcomes cannot collapse into two by accident. Adding a
//!    `NotRun` variant breaks every incomplete `match` in the workspace, which is what makes the
//!    rule reach gates nobody has written yet.
//! 2. **The search tool going absent.** The matcher is the `regex` crate, compiled in. Exit 127
//!    is not a reachable state for pattern matching at all, so a gate cannot report clean because
//!    its search binary was missing.
//! 3. **Compound conditions short-circuiting clean.** [`gate::probe_files`] returns
//!    `Result<bool, NotRun>`, so `?` propagates "did not run" instead of leaving it to a caller
//!    who must remember that an unreadable input is not a `false`.
//!
//! ── THE MODULES ──────────────────────────────────────────────────────────────────────────────
//!
//! [`verdict`] holds the outcome type and its rendering; [`gate`] the six assertions written
//! against it; [`pattern`] the compiled search patterns; [`scan`] the fail-closed tree walk that
//! reports offending lines; [`report`] the accumulation and the process exit contract; [`lock`]
//! the `flock`-based serialisation of expensive steps; [`proc`] child processes that never lose
//! the reason they stopped.
//!
//! ── OUTPUT IS A CONTRACT ─────────────────────────────────────────────────────────────────────
//!
//! Failures render as one headline plus six-space continuation lines, and `cargo xtask platform
//! wave` scrapes gate logs to build its step table. The text is part of the interface.
//!
//! ── USAGE ────────────────────────────────────────────────────────────────────────────────────
//!
//! ```no_run
//! use std::path::Path;
//! use verification_core::{gate, Pattern, Report};
//!
//! let mut report = Report::new("verify-example");
//! let src = [Path::new("src/lib.rs")];
//!
//! report.check(gate::ban(
//!     "no stray dbg! in committed code",
//!     &Pattern::literal("dbg!("),
//!     &src,
//! ));
//! report.check(gate::require(
//!     "the module must keep its safety comment",
//!     &Pattern::regex(r"^// SAFETY:").unwrap(),
//!     &src,
//! ));
//!
//! std::process::exit(report.finish());
//! ```

pub mod gate;
pub mod lock;
pub mod pattern;
pub mod proc;
pub mod report;
pub mod scan;
pub mod verdict;

pub use lock::{GateLock, flock_exclusive};
pub use pattern::Pattern;
pub use report::Report;
pub use verdict::{Finding, Kind, NotRun, Verdict};
