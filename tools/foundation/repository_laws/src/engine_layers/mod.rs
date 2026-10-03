//! Engine-layer walls — `documentation/standards/engine_boundary_rules.md` §5 rules
//! **1, 2, 3a, 3b, 6 and 7**, plus the map engine's whole-crate UI-framework ban. Numbers 4 and 5
//! are unassigned: the mission crates' isolation is the crate-tier law's category matrix, and the
//! mission-editing crates' browser ban is the crate-tier law's firewall.
//!
//! **Role:** judges the one-way layering of the graphics layer (the parked graphics engine and
//! the `crates/graphics` members), the map engine and the frontend, and renders the report
//! `cargo xtask verify engine-layers` prints, with each rule's finding count alongside.
//!
//! **Position:** part of [`crate`]. [`check_engine_layers`] runs three steps:
//! `matcher_probes` proves every matcher, `crate_walks` walks the crates, `evaluation`
//! judges the rules; `rules` holds the matchers and pins, `report_text` the fixed report
//! lines and `scanning` the shared helpers. `ui_framework_ban` is judged separately and is
//! not part of the report.
//!
//! **Signals & state:** none; every function is pure over the checkout it is handed.
//!
//! **Invariants:** exit 0 means every rule ran and held, 1 a breach, a matcher that answered a
//! probe wrongly or a root with no file in it, 2 a root, manifest or file that could not be read.
//! The report text is byte-stable: it is the gate's output contract.
//!
//! ── WHAT THIS DEFENDS ────────────────────────────────────────────────────────────────────────
//!
//! The graphics layer is only a renderer: device, pipelines, shaders, draw batching, text
//! packing, the rAF pump in `graphics_engine`, and the GPU-free byte layouts, geometry, glyph
//! packing and WGSL source in the `crates/graphics` members it builds on. The map domain
//! (terrain, symbology, spatial, streaming) lives in `map_engine` and the crates it links; the
//! editing layer over the mission document is the crates of `crates/mission_editing/`. The
//! dependency arrow is one-way:
//!
//! ```text
//! frontend ──► map_engine ──► graphics_engine ──► crates/graphics
//!     └──────────────────────────────────────────► crates/graphics (targets = "any" only)
//! ```
//!
//! Nothing in the compiler enforces that: `graphics_engine` could add a dependency edge
//! back to `map_engine` and everything would still build. These rules are the wall. The
//! `crates/graphics` members' own edges are also judged by the crate-tier law (category matrix
//! and the strangler rule: a graphics crate depends on foundation and graphics crates only, and on
//! nothing under `legacy/`); rules 1 and 2 hold the same line in their sources.
//!
//! | # | rule | what rots without it |
//! |---|------|----------------------|
//! | 1 | `legacy/graphics_engine/**` and every `crates/graphics` member may not import `map_engine` | the arrow turns into a cycle |
//! | 2 | no `terrain` / `symbology` / `mission` / `orbat` / `arma` in a declared name there | "pure renderer" becomes a claim in a README rather than a property of the code |
//! | 3a | only one enumerated file of `legacy/map_engine/src` names `graphics_engine::frame` | the packet boundary scatters into imports across the crate |
//! | 3b | no module of `legacy/map_engine/src` names `graphics_engine::{device, pipeline, shaders, r#loop}` beyond the pin | GPU resource creation drifts back to the caller one import at a time |
//! | 6 | `apps/frontend/**` may not import `graphics_engine` or a wasm-only `crates/graphics` member | the frontend drives the GPU directly and the middle crate becomes optional |
//! | 7 | `legacy/map_engine/src/world/**` names neither `yrs`, nor a mission document crate, nor a mission-editing crate | the static world and the authored document fuse into one |
//!
//! ── RULES 1 AND 6 — A MANIFEST ARM BESIDE THE SOURCE ARM ─────────────────────────────────────
//!
//! A renamed dependency (`r = { package = "map_engine" }`) makes every `use r::…` invisible
//! to a source matcher, so both rules also read the crate's `Cargo.toml` for the package name; a
//! `#` comment line naming the other crate is prose, not an edge. Rule 6's source arm matches the
//! two shapes that are an import — a `graphics_engine::` path and an `extern crate` —
//! because the frontend's prose names the renderer by its Cargo spelling while describing the
//! boundary it respects, and a gate that turns correct comments red teaches people to delete them.
//!
//! Rule 6's purpose is that the frontend never reaches the GPU layer directly. A `crates/graphics`
//! member declaring `targets = "any"` is CPU code — byte layouts, geometry, glyph packing — that
//! the frontend may link, so rule 6 covers the parked graphics engine and the members declaring
//! `targets = "wasm32"` alone, each with the same two arms spelled with that crate's own names.
//!
//! ── RULE 2 — DECLARATIONS, NOT MENTIONS ──────────────────────────────────────────────────────
//!
//! The five nouns are ordinary English and ordinary graphics vocabulary: a doc comment may discuss
//! what the map engine hands over, a `.wgsl` path may be named after the feature it draws, and
//! "submission" contains "mission". So rule 2 matches a declaration keyword, whitespace, then an
//! identifier containing the noun, case-insensitively:
//!
//! ```text
//! \b(struct|enum|trait|type|fn|const|static|mod)\s+\w*(terrain|symbology|mission|orbat|arma)
//! ```
//!
//! `\w*` is what makes `submit_mission` a violation and `submission` not one. Enum variants and
//! struct fields are bare identifiers inside a block with no keyword in front of them; a line
//! matcher does not see them, and catching them needs block tracking.
//!
//! ── RULES 3a AND 3b — PINS, NOT DIRECTORIES OR THRESHOLDS ────────────────────────────────────
//!
//! The frame vocabulary is spelled on five `pub use` lines in `frame/mod.rs` and nowhere else;
//! every other module, `frame/`'s own submodules included, reads `use crate::frame::DrawBatch;`.
//! The value of that chokepoint is that the crate's whole graphics interface is a list a reviewer
//! reads in one screen, and widening it is a diff to that screen — which a per-file pin expresses
//! and a directory rule cannot. `frame/mod.rs`'s own prose does not spell the path, so the pinned
//! count is exactly the size of the interface list.
//!
//! Rule 3b's five pinned sites share one cause: `RenderEngine` is defined in `map_engine`
//! (`frame/engine.rs`) and holds every GPU resource the renderer owns. `impl FrameTarget for
//! RenderEngine` must live in the crate that defines the type (E0116) and `#[wasm_bindgen]`
//! refuses trait impls, so `frame/pump.rs` names `r#loop` and `frame/mod.rs` documents the
//! re-export the frontend reaches the pump through; `frame/mod.rs` aliases `device::buffers` and
//! `pipeline` at one seam each rather than at every call site.
//!
//! Both pins ratchet in both directions: an unpinned file naming the path fails, a pinned file
//! whose count grows or shrinks fails, and a pinned file that no longer matches at all fails —
//! the arm that tells a renamed or deleted directory apart from a clean boundary.
//!
//! ── NUMBER 4 — THE MISSION CRATES' ISOLATION, JUDGED ELSEWHERE ────────────────────────────────
//!
//! The mission compiler, the validator and the document are the crates of `crates/mission/`, not
//! modules of the map engine. That they reach no world, streaming or graphics code is a
//! dependency edge, and the crate-tier law judges it from the manifests: a `crates/mission` crate
//! depends on foundation, mission and geometry crates only (rule 5 of that law,
//! `crate_tiers_a_mission_crate_reaching_world_or_graphics_is_rule_5`) and on nothing under
//! `legacy/` (its rule 7, the same test). No line matcher here repeats it.
//!
//! ── NUMBER 5 — THE MISSION-EDITING CRATES' BROWSER BAN, JUDGED ELSEWHERE ─────────────────────
//!
//! The Mission Creator's decisions — tool state machines, the undo drive, the command formatting,
//! the draft decisions — are the crates of `crates/mission_editing/`, not a module of the map
//! engine. That they name no browser crate is the crate-tier law's firewall: a manifest edge on a
//! browser crate is a finding there, and a source scan refuses the bare words `web_sys`, `leptos`
//! and `wasm_bindgen` in any `.rs` file under that category, prose included. No line matcher here
//! repeats it.
//!
//! ── RULE 7 — THE IMPORT WALL BETWEEN WORLD AND DOCUMENT ──────────────────────────────────────
//!
//! `world/` is immutable, streamed, cacheable and never persisted; the mission document is
//! mutable, undoable, CRDT-synced and persisted. A `world/` type cannot acquire undo, persistence
//! or CRDT state without naming `yrs`, a mission document crate (`mission_crdt`,
//! `mission_document`, `mission_operations`) or a mission-editing crate that hosts the live
//! document (`mission_editing_session`, `mission_editing_commands`, `mission_persistence`,
//! `map_editing_tools`), so `world/**` may name none of them. The other direction is a dependency
//! edge: the mission and mission-editing crates cannot reach a map engine module because the
//! crate-tier law forbids the edge (see number 4 above). A hand-rolled `dirty: bool` on a world struct that imports nothing is not visible to a
//! line matcher; it becomes visible the moment anything persists it.
//!
//! ── WHY EXIT 2 EXISTS ────────────────────────────────────────────────────────────────────────
//!
//! A layer wall is exactly the kind of check a directory move renames out from under itself. If a
//! missing root read as "no violations found", the wall would evaporate on the very commit that
//! most needs it. So a root that is absent or unreadable is [`verification_core::NotRun`] and exits 2, and a
//! root that exists but is empty — `world/` included — is a failure, because "no
//! file under `world/` names the document" and "there is no `world/`" are the same sentence to a
//! matcher; the scanned counts are printed so the two never read alike.

use std::path::Path;

mod crate_walks;
mod evaluation;
mod matcher_probes;
mod report_text;
mod rules;
mod scanning;
mod ui_framework_ban;

/// Rule 2's map-noun declaration matcher, which the graphics-category firewall of the workspace
/// laws reuses so both judge a declared name the same way.
pub(crate) use rules::DECL_RE as MAP_NOUN_DECLARATION_PATTERN;
pub use ui_framework_ban::{
    UI_FRAMEWORK_IMPORT_RE, UI_FRAMEWORK_PACKAGE_RE, UiFrameworkScan,
    map_engine_ui_framework_findings,
};

/// One engine-layer rule of the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineLayerRule {
    /// Rule 1: neither `graphics_engine` nor a `crates/graphics` member imports or depends on
    /// `map_engine`.
    GraphicsEngineImportsNoMapEngine,
    /// Rule 2: no map noun in a name declared under `graphics_engine` or a `crates/graphics`
    /// member.
    GraphicsEngineDeclaresNoMapNoun,
    /// Rule 3a: only the enumerated packet boundary names `graphics_engine::frame`.
    FrameVocabularyStaysAtThePacketBoundary,
    /// Rule 3b: GPU-resource modules of the renderer are named only at the pinned sites.
    GpuResourceModulesStayPinned,
    /// Rule 6: the frontend neither imports nor depends on `graphics_engine` or a wasm-only
    /// `crates/graphics` member.
    FrontendImportsNoRenderer,
    /// Rule 7: `world/` names neither `yrs`, nor a mission document crate, nor a mission-editing
    /// crate.
    WorldAndDocumentShareNothing,
}

impl EngineLayerRule {
    /// The rule's number in `engine_boundary_rules.md` §5.
    pub fn number(self) -> &'static str {
        match self {
            Self::GraphicsEngineImportsNoMapEngine => "1",
            Self::GraphicsEngineDeclaresNoMapNoun => "2",
            Self::FrameVocabularyStaysAtThePacketBoundary => "3a",
            Self::GpuResourceModulesStayPinned => "3b",
            Self::FrontendImportsNoRenderer => "6",
            Self::WorldAndDocumentShareNothing => "7",
        }
    }
}

/// One rule's judgement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineLayerRuleResult {
    /// Which rule.
    pub rule: EngineLayerRule,
    /// How many findings a reader has to go and fix; zero when the rule holds.
    pub findings: usize,
    /// The report lines of those findings, as the report prints them.
    pub detail: Vec<String>,
}

impl EngineLayerRuleResult {
    fn new(rule: EngineLayerRule, findings: usize, detail: &[String]) -> Self {
        Self {
            rule,
            findings,
            detail: detail.to_vec(),
        }
    }
}

/// The whole engine-layer judgement of one checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineLayerReport {
    /// 0 every rule ran and held; 1 a breach, a wrong matcher probe or an empty root; 2 an input
    /// that could not be read.
    pub exit_code: u8,
    /// The report text, one line per entry.
    pub lines: Vec<String>,
    /// Every rule judged, in report order; fewer than six when the run stopped early.
    pub rule_results: Vec<EngineLayerRuleResult>,
}

impl EngineLayerReport {
    /// The judgement of `rule`, when the run reached it.
    pub fn rule(&self, rule: EngineLayerRule) -> Option<&EngineLayerRuleResult> {
        self.rule_results.iter().find(|result| result.rule == rule)
    }

    /// True when every rule was judged, whatever the outcome.
    pub fn judged_every_rule(&self) -> bool {
        self.rule_results.len() == 6
    }
}

/// Judge every engine-layer rule over the checkout at `repo_root`.
pub fn check_engine_layers(repo_root: &Path) -> EngineLayerReport {
    let mut rule_results = Vec::new();
    let mut o: Vec<String> = Vec::new();
    let outcome = matcher_probes::probe_matchers(&mut o).and_then(|patterns| {
        crate_walks::walk_crates(repo_root, &mut o)
            .map(|(sources, scanned)| (patterns, sources, scanned))
    });
    let (exit_code, lines) = match outcome {
        Ok((patterns, sources, scanned)) => {
            evaluation::evaluate(repo_root, patterns, sources, scanned, o, &mut rule_results)
        }
        Err(refusal) => refusal,
    };
    EngineLayerReport {
        exit_code,
        lines,
        rule_results,
    }
}

/// The report as `(exit code, lines)`, the shape the engine-layer tests assert on.
#[cfg(test)]
fn run(repo_root: &Path) -> (u8, Vec<String>) {
    let report = check_engine_layers(repo_root);
    (report.exit_code, report.lines)
}

#[cfg(test)]
use report_text::*;
#[cfg(test)]
use rules::*;
#[cfg(test)]
use scanning::is_source;
#[cfg(test)]
use verification_core::Pattern;

#[cfg(test)]
#[path = "tests/fixture_repository.rs"]
mod fixture_repository;

#[cfg(test)]
#[path = "tests/engine_layer_walls.rs"]
mod engine_layer_walls_tests;

#[cfg(test)]
#[path = "tests/engine_layer_rule_results.rs"]
mod engine_layer_rule_results_tests;

#[cfg(test)]
#[path = "tests/graphics_category_members.rs"]
mod graphics_category_members_tests;
