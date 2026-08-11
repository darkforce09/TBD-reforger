//! `tbd-gate` — the four-outcome static-check library.
//!
//! T-853. The Rust port of [`scripts/mod/lib/gate-grep.sh`], built FIRST in the shell→`xtask`
//! migration and for the same reason that file was extracted from its callers in T-556.
//!
//! ── WHY THIS IS THE FIRST THING PORTED ───────────────────────────────────────────────────────
//!
//! `gate-grep.sh` states its own purpose:
//!
//! > T-216 fixed exactly this defect in `scripts/verify-t180-coherency.sh` in wave 5, inline. The
//! > fix did not propagate: every `scripts/mod/verify-t*.sh` written afterwards was born with the
//! > same two holes. This file is the propagation mechanism — one implementation, sourced by
//! > every mod gate, so the next gate cannot be born broken by copy-paste.
//!
//! Porting the gates one at a time without this library landing first would have each of ~20
//! ports invent its own verdict shape — destroying the propagation mechanism in the very act of
//! modernising it. So the library goes first, and every ported gate is written against it.
//!
//! ── WHAT THE TYPE SYSTEM BUYS OVER THE BASH ──────────────────────────────────────────────────
//!
//! Three defect classes that bash could only ask callers to remember stop being representable:
//!
//! 1. **"The check did not run" folded into "the check passed."** [`Verdict`] has no `bool`
//!    conversion of any kind, so the four outcomes cannot collapse into two by accident. Adding a
//!    [`NotRun`] variant later breaks every incomplete `match` in the workspace — propagation the
//!    bash library wanted and could not enforce.
//! 2. **The search tool going absent.** The matcher is the `regex` crate, compiled in. Exit 127 —
//!    the T-620 defect that kept `verify-no-python` green for four waves — is no longer reachable
//!    for pattern matching at all.
//! 3. **Compound conditions short-circuiting clean.** [`gate::probe_files`] returns
//!    `Result<bool, NotRun>`, so `?` propagates "did not run" instead of leaving it to a caller
//!    who must remember that a status above 1 is not a `false`.
//!
//! ── OUTPUT IS A CONTRACT ─────────────────────────────────────────────────────────────────────
//!
//! Failures render byte-for-byte as the bash helpers did, six-space continuation indent and em
//! dash included. Ports are accepted by diffing old and new stdout on both a clean tree and a
//! deliberately broken one, and `wave.sh` scrapes these logs.
//!
//! ── USAGE ────────────────────────────────────────────────────────────────────────────────────
//!
//! ```no_run
//! use std::path::Path;
//! use tbd_gate::{gate, Pattern, Report};
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
//!
//! [`scripts/mod/lib/gate-grep.sh`]: ../../../scripts/mod/lib/gate-grep.sh

pub mod gate;
pub mod lock;
pub mod pattern;
pub mod proc;
pub mod report;
pub mod verdict;

pub use lock::{GateLock, flock_exclusive};
pub use pattern::Pattern;
pub use report::Report;
pub use verdict::{Finding, Kind, NotRun, Verdict};
