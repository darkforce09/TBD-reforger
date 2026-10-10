//! The four-outcome static-check library every repository gate is written against.
//!
//! **Role:** one implementation of what a check can conclude — held, failed, or did not run —
//! and the primitives a gate is written with: [`verdict`] (the outcome and its rendering),
//! [`gate`] (the six pattern assertions), [`pattern`] (compiled search patterns), [`scan`] (the
//! fail-closed tree walk), [`report`] (accumulation and the process exit contract) and [`lock`]
//! (the exclusive lock that serialises expensive steps).
//! **Position:** tier 0 of `tools/foundation`, over `regex` alone. `process_runner` and
//! `repository_laws` build on it; `xtask` writes every verification with it, and the `api`
//! engineering-law tests read it as a dev-dependency.
//! **Signals & state:** none; values, pure functions and one file lock its caller holds.
//! **Invariants:** "the check did not run" is never folded into "the check passed": [`Verdict`]
//! has no `bool` conversion, the matcher is compiled in so an absent search tool is not a
//! reachable state, and [`gate::probe_files`] returns `Result<bool, NotRun>` so `?` carries "did
//! not run" upward. Failures render as one headline plus six-space continuation lines, a text
//! contract the gate logs carry.
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

mod error;
pub mod gate;
pub mod lock;
pub mod pattern;
pub mod prelude;
pub mod report;
pub mod scan;
pub mod verdict;

pub use error::{Error, Result};
pub use lock::{GateLock, flock_exclusive};
pub use pattern::Pattern;
pub use report::Report;
pub use verdict::{Finding, Kind, NotRun, Verdict};
