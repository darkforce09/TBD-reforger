//! Destroy-target inert diagnostics must not claim `entities[]` never spawn.
//!
//! `TBD_MissionDocumentStruct` models `entities[]` and `SpawnMissionEntities` places resolvable
//! rows, so an operator-facing string that still blames a build that "does not spawn/model
//! entities[]" is a lie that costs the next reader a wasted investigation.
//!
//! Four false-green classes this gate is built to refuse: a paraphrased lie; a collapsed
//! `DiagnoseEmpty` whose returns are pinned only in comments; a renamed function whose old name
//! survives only in a comment; and an unresolved-alias registry pin moved into a comment. So the
//! gate strips `//` and `/* */` before every structural pin, requires a live fn definition plus
//! three return-string arms, bans a broad set of paraphrases, and RED→GREEN-proves each attack on
//! every run.
//!
//! ── RED ARMS CANNOT FAIL OPEN ────────────────────────────────────────────────────────────────
//!
//! Each RED proof perturbs the source in memory and requires the pin to reject it. Those arms
//! return a [`Verdict`], so Held / Failed / DidNotRun cannot be confused: a DidNotRun on a RED arm
//! fails the gate with its own message rather than reading as "the pin correctly rejected it".
//! Perturbations are in-memory string transforms — live files are never written — while FAIL lines
//! still print a `/tmp/tmp.*` display path so a clean-tree run reads like the file it describes.
//!
//! Output and the binary 0/1 status are a contract: the wave gate prints the last 15 lines of a
//! failed step.

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
pub use source_audit::verify_destroy_target_diagnostics;

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
