//! T-437 / T-474 — Destroy-target inert diagnostics must not claim entities[] never spawn
//! (T-853 / T-854 port of `scripts/mod/verify-t437-destroy-inert-diagnostics.sh`).
//!
//! After T-254, `TBD_MissionDocumentStruct` models `entities[]` and `SpawnMissionEntities` places
//! resolvable rows. Operator-facing strings that still blame a build that "does not spawn/model
//! entities[]" are lies. T-474 closed four false-green classes (paraphrased lies, collapsed
//! DiagnoseEmpty returns with pins only in comments, renamed fn with name only in a comment,
//! unresolved-alias registry pin moved to a comment). This gate strips `//` / `/* */` before
//! structural pins, requires a live fn definition + three return-string arms, broadens forbidden
//! paraphrases, and RED→GREEN-proves each attack on every run.
//!
//! ── WHAT THE PORT REMOVES ────────────────────────────────────────────────────────────────────
//!
//! 1. **`python3`, entirely — seven call sites.** Two heredocs (scan + registry pins) and five RED
//!    setup transforms. The script was on `scripts/python-inventory.txt` solely for those; the
//!    inventory line goes with them.
//! 2. **Five `2>/dev/null` fail-opens on the RED arms.** Each RED proof read
//!    `if scan_forbidden_file|assert_registry_pins … 2>/dev/null; then "still passed" else
//!    "FAIL (expected)"`. A crash / unreadable TMP / absent `python3` (127) exited non-zero and
//!    was indistinguishable from "the pin correctly rejected the perturbation", with the traceback
//!    swallowed. Here checks return [`Verdict`]: Held / Failed / DidNotRun cannot be confused, and
//!    a DidNotRun on a RED arm fails the gate with a distinct message (not "expected").
//! 3. **`mktemp` + `trap` scribble risk.** Perturbations are in-memory string transforms; FAIL
//!    lines that named `$TMP` still print a `/tmp/tmp.*` display path so clean-tree acceptance
//!    normalises the same way bash-vs-bash did. Live files are never written.
//!
//! Output + binary 0/1 status are a contract (`wave.sh` tails failures; T-853 diffs stdout).

use std::path::{Path, PathBuf};

use anyhow::Result;
use developer_tools::repository_layout::definition_path;
use regex::Regex;
use verification_core::{Finding, NotRun, Pattern, Verdict, gate};

const REG_REL: &str =
    "apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/TBD_ObjectiveRegistry.c";
const COMP_REL: &str =
    "apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/TBD_ObjectivesComponent.c";
const RULES_REL: &str =
    "apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/TBD_ObjectiveRules.c";
const VALIDATOR_REL: &str =
    "apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/TBD_MissionValidator.c";

const EXACT_LIES: &[&str] = &[
    "This build does not spawn the mission document",
    "does not spawn the mission document",
    "nothing spawns the mission document",
    "nothing spawns mission `entities[]`",
    "on today's build nothing spawns mission",
    "TBD_MissionDocumentStruct does not model them",
    "TBD_MissionDocumentStruct ignore `entities[]`",
    "does not spawn mission entities",
    "this build cannot create it",
];

const PARAPHRASES: &[&str] = &[
    r"are never placed",
    r"never placed",
    r"struct ignores",
    r"ignores them",
    r"ignores entities",
    r"does not spawn entities",
    r"does not model entities",
];

const ARM_SIG: &str = "\tstatic void ArmDestroyTargets(notnull TBD_Objective objective)";
const DIAG_SIG: &str =
    "\tprotected static string DiagnoseEmptyDestroyTargets(notnull TBD_Objective objective)";
const REG_PIN: &str = "not in the registry, so there is no prefab to look for";
const REG_FORMAT_OLD: &str = "\t\t\tobjective.m_sInertReason = string.Format(\
\"rules.targetAlias '%1' is not in the registry, so there is no prefab to look for\", \
objective.m_sTargetAlias);";

// ── Paths / I/O ──────────────────────────────────────────────────────────────────────────────

struct Paths {
    reg: PathBuf,
    comp: PathBuf,
    rules: PathBuf,
    schema: PathBuf,
    validator: PathBuf,
}

struct Texts {
    reg: String,
    comp: String,
    rules: String,
    schema: String,
    validator: String,
}

impl Paths {
    fn resolve(root: &Path) -> Paths {
        Paths {
            reg: root.join(REG_REL),
            comp: root.join(COMP_REL),
            rules: root.join(RULES_REL),
            schema: definition_path(root, "mission.schema.json"),
            validator: root.join(VALIDATOR_REL),
        }
    }

    fn all(&self) -> [&Path; 5] {
        [
            self.reg.as_path(),
            self.comp.as_path(),
            self.rules.as_path(),
            self.schema.as_path(),
            self.validator.as_path(),
        ]
    }

    fn read_all(&self) -> Result<Texts, Verdict> {
        Ok(Texts {
            reg: read_text(&self.reg)?,
            comp: read_text(&self.comp)?,
            rules: read_text(&self.rules)?,
            schema: read_text(&self.schema)?,
            validator: read_text(&self.validator)?,
        })
    }
}

// ── scan_forbidden_file (Python → Rust) ──────────────────────────────────────────────────────

// ── assert_registry_pins ─────────────────────────────────────────────────────────────────────

// ── assert_other_pins (gate_require) ─────────────────────────────────────────────────────────

// ── RED helpers ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests/destroy_target_diagnostics/tests.rs"]
mod tests;

mod source_audit;
use source_audit::assert_registry_pins;
use source_audit::read_text;
use source_audit::scan_forbidden;
pub use source_audit::verify_t437;

mod strip_c_comments;
use strip_c_comments::assert_other_pins;
use strip_c_comments::collapse_diagnose_returns;
use strip_c_comments::comment_only_registry_pin;
use strip_c_comments::inject_historical_lie;
use strip_c_comments::inject_paraphrase_lie;
use strip_c_comments::red_registry;
use strip_c_comments::red_scan;
use strip_c_comments::rename_diagnose_fn;
use strip_c_comments::strip_c_comments;
