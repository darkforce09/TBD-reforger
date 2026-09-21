//! The two gate drivers: the cheap per-slice gate and the full wave gate.
//!
//! TIERED GATES. A slice pays only the cheap gate (~10 s). The expensive suite runs once per wave
//! on merged main. `cargo xtask ci ci-local` is deliberately NOT used here: it takes 15-40 minutes.
//!
//! No chromium and no editor suite in either driver. Rect smokes and the rest of
//! `gate editor-suite` run only via `cargo xtask mk leptos-gates`, which is a required
//! editor-factory pre-close step (see `docs/platform/EDITOR_FACTORY_FOR_CURSOR.md` §5).
//!
//! Every step is one `run "<label>" <cmd>` line. The runner captures stdout+stderr, prints PASS or
//! FAIL, shows the last 15 captured lines indented six spaces on failure, and accumulates `fail` —
//! the gate is NOT fail-fast, every step runs.

use super::{
    Ctx, base, changed, db, git_stdout_lossy, host, lock::GateState, migrate, schema, touch, trunk,
    verdict,
};
use crate::{wprint, wprintln};

/// One step. `f` returns the step's rc; stdout and stderr are captured together.
struct Runner {
    fail: bool,
    /// The wave gate's runner carries a distinct arm for a timeout's exit 124 so the most
    /// expensive step's deadline is not relabelled as a code error. The slice gate sets none.
    timeout_arm: Option<u64>,
}

impl Runner {
    fn run(&mut self, label: &str, f: impl FnOnce() -> i32) {
        wprint!("  {label:<28} ");
        let (out, rc) = super::capture_step(f);
        if rc == 0 {
            wprintln!("PASS");
            return;
        }
        if let Some(secs) = self.timeout_arm
            && rc == 124
        {
            wprintln!("FAIL (TIMEOUT after {secs}s)");
            self.fail = true;
            return;
        }
        wprintln!("FAIL");
        // The last 15 captured lines, indented six spaces.
        let lines: Vec<&str> = out.lines().collect();
        for l in lines.iter().skip(lines.len().saturating_sub(15)) {
            wprintln!("      {l}");
        }
        self.fail = true;
    }
}

/// The ten `xtask verify` Class-R steps both gates share, in order.
///
/// Both `gate_slice` and `cmd_gate` iterate this one table, so no step can be wired into a single
/// driver and drift green on the path that never runs it. A verification that exists but appears
/// in no row is invoked by nothing and proves nothing; `verify ci-schema-parity` is the tripwire
/// that reds when a row or its dispatch disappears.
const VERIFY_STEPS: &[(&str, &str)] = &[
    ("object registry aliases", "object-registry-aliases"),
    ("wiki seeds", "wiki-seeds"),
    ("faction library seeds", "faction-library-seeds"),
    ("staging compose paths", "staging-compose-paths"),
    ("mission REST size limits", "mission-rest-size-limits"),
    ("CI schema parity", "ci-schema-parity"),
    ("destroy target diagnostics", "destroy-target-diagnostics"),
    ("route tags", "route-tags"),
    ("reporter identity", "results-reporter-identity-comments"),
    ("player identity comments", "player-identity-comments"),
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
