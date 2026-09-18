//! The two gate drivers: the cheap per-slice gate and the full wave gate.
//!
//! TIERED GATES (correction 3). A slice pays only the cheap gate (~10 s). The expensive suite runs
//! once per wave on merged main. `cargo xtask ci ci-local` is deliberately NOT used: it is 15-40 minutes, not
//! the 22.7 s the docs still claim.
//!
//! T-843 option (b): **no chromium / editor-suite here.** Rect smokes and the rest of
//! `gate editor-suite` run only via `cargo xtask mk leptos-gates`, which is a required
//! editor-factory pre-close step (see `docs/platform/EDITOR_FACTORY_FOR_CURSOR.md` §5).
//!
//! Every step is one `run "<label>" <cmd>` line. The runner captures stdout+stderr, prints PASS or
//! FAIL, shows `tail -15` indented six spaces on failure, and accumulates `fail` — the gate is NOT
//! fail-fast, every step runs.

use super::{
    Ctx, base, changed, db, git_stdout_lossy, host, lock::GateState, migrate, schema, touch, trunk,
    verdict,
};
use crate::{wprint, wprintln};

/// One step. `f` returns the step's rc; its output is captured exactly as `$( … 2>&1 )` did.
struct Runner {
    fail: bool,
    /// The wave gate's runner has a distinct arm for `timeout`'s 124 so the most expensive step's
    /// deadline is not relabelled as a code error. The slice gate's runner never had one.
    timeout_arm: Option<u64>,
}

impl Runner {
    fn run(&mut self, label: &str, f: impl FnOnce() -> i32) {
        wprint!("  {label:<24} ");
        let (out, rc) = super::capture_step(f);
        if rc == 0 {
            wprintln!("PASS");
            return;
        }
        if let Some(secs) = self.timeout_arm {
            if rc == 124 {
                wprintln!("FAIL (TIMEOUT after {secs}s)");
                self.fail = true;
                return;
            }
        }
        wprintln!("FAIL");
        // `printf '%s\n' "$out" | tail -15 | sed 's/^/      /'`
        let lines: Vec<&str> = out.lines().collect();
        for l in lines.iter().skip(lines.len().saturating_sub(15)) {
            wprintln!("      {l}");
        }
        self.fail = true;
    }
}

/// The ten `xtask verify` Class-R steps both gates share, in order.
///
/// T-462. Shell Class-R near schema: verify scripts that exist but were never invoked by the cold
/// gate (wave 24 adversarial — T-439 unwired; T-444 pin absent).
/// T-463. Same pattern for T-438 deploy-staging compose path + T-456 REST size gate (wave 25 —
/// scripts existed, cold gate never executed them).
/// T-468. Tripwire: ci.yml schema job must stay on `cargo xtask ci ci-local-schema`.
/// T-478. verify-t440 pins BOTH the gate_slice run and the cmd_gate run (comment-strip + redirect
/// recipe + dual-path); deleting either run must FAIL the verify script.
/// T-556. The T-462/T-463 pattern once more, and the worst instance of it: T-296 and T-452 existed,
/// carried the fail-open `if rg …; then fail; fi` shape, AND were invoked by nothing — not the
/// gate, not ci.yml, not the Makefile. So a reader who found them would have trusted a pair of bans
/// that had never compared anything. Wired into both halves so neither path can drift green alone.
const VERIFY_STEPS: &[(&str, &str)] = &[
    ("T-439 objects aliases", "t439"),
    ("T-444 wiki seed", "t444"),
    ("T-440 faction library seed", "t440"),
    ("T-438 deploy-staging", "t438"),
    ("T-456 REST size gate", "t456"),
    ("T-468 CI schema parity", "t468"),
    ("T-437 destroy inert", "t437"),
    ("T-586 route tags", "route-tags"),
    ("T-296 reporter identity", "t296"),
    ("T-452 player identity", "t452"),
];

#[cfg(test)]
#[path = "tests/gate/tests.rs"]
mod tests;

mod checkrun;
use checkrun::checkrun;
pub use checkrun::gate_slice;
use checkrun::hostrun;

mod gate_dispatch;
pub use gate_dispatch::cmd_gate;
