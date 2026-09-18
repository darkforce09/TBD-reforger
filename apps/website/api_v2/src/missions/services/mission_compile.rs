//! Backend adapter for the shared mod-document flatten. The compile logic lives in
//! `website_map_engine::data::scenario::flatten`; this builds the core `MissionMeta` from the
//! backend `Mission` model **plus the environment authored into the saved version payload**, and
//! re-exports the output types so callers name one path.
//!
//! Locked coordinate mapping (in core): editor `position.x → x`, `position.y → z`,
//! `position.z → y` (optional, 1.2), `position.rotation → headingDeg`.
//!
//! @contract mission.schema.json#/

use crate::missions::models::mission::Mission;
use website_map_engine::data::scenario::flatten::{self, MissionMeta};
use website_map_engine::data::scenario::wire_safety::{self, CargoPhysCatalog};

pub use website_map_engine::data::scenario::flatten::{
    CompileError, ModMissionDocument, ModSlot, mission_terrain_key,
};
/// The compile's structured diagnostics ride out of core on
/// [`ModMissionDocument::diagnostics`]. Re-exported here for the same reason every other output
/// type is: callers name one path, not two.
pub use website_map_engine::data::scenario::validate::{
    Finding as CompileFinding, Severity as FindingSeverity,
};

/// Header carrying how many structured findings the compile produced (always present, `0` included).
///
/// **Why a header and not the body.** `mission.schema.json` closes the document root with
/// `additionalProperties: false` and
/// [`crate::missions::handlers::mission_export::get_compiled_mission`] holds the body to it, so a
/// `diagnostics` key in the JSON would take `/compiled` down for every mission. The findings
/// therefore ride ALONGSIDE the bytes, in the only channel HTTP offers for that, and the body stays
/// byte-identical to the contract.
pub const COMPILE_DIAGNOSTICS_COUNT_HEADER: &str = "x-compile-diagnostics-count";

/// Header naming WHICH rules fired, comma-separated and de-duplicated. Omitted when nothing fired.
///
/// Rule ids only — never messages. Ids are `&'static str` ASCII constants
/// (`website_map_engine::data::scenario::flatten::COMPILE_DIAGNOSTIC_RULE_IDS`), so this value is always a
/// legal header value; a message carries author text of arbitrary length and encoding and would make
/// the header a second, worse copy of the log line below it.
pub const COMPILE_DIAGNOSTICS_RULES_HEADER: &str = "x-compile-diagnostics-rules";

/// The `x-compile-diagnostics-rules` value for a finding list: each rule id once, in first-fired
/// order (which is the compile's own deterministic walk order), comma-separated. `None` when the
/// list is empty, so the caller omits the header rather than sending a blank one.
#[must_use]
pub fn compile_diagnostics_rules_header(findings: &[CompileFinding]) -> Option<String> {
    let mut seen: Vec<&str> = Vec::new();
    for f in findings {
        if !seen.contains(&f.rule_id) {
            seen.push(f.rule_id);
        }
    }
    if seen.is_empty() {
        None
    } else {
        Some(seen.join(","))
    }
}

/// Build the compiled mod mission document from a mission row + its version payload. Thin wrapper
/// over the shared [`website_map_engine::data::scenario::flatten::flatten_to_mod_document`].
///
/// **Time/weather come from the payload first, the row second.** The Mission Settings dialog and
/// the top-strip scrubber author `meta.environment.{time,weather}` into the editor document, and
/// the save compiler carries them out to the payload's top-level `environment`. Building
/// `MissionMeta` from `missions.time_of_day` / `missions.weather` alone would hand the game server
/// the values the mission was *created* with while the author is looking at the ones they set — an
/// edit that never reaches the wire. The row is the fallback, for versions saved before the editor
/// authored an environment and for any field the payload leaves blank or malformed.
///
/// That is one half of the rule. The other half is the editor's `PATCH /missions/{id}` mirror
/// (`eden_chrome::RowMirror`), because `mission_hydrate::apply_row_meta` re-applies the row over the
/// document on every load — without it the authored value would still be reverted locally, and this
/// preference would only paper over a row the editor had gone out of sync with. Neither half ships
/// alone.
///
/// **The precedence itself lives in core** ([`flatten::apply_authored_environment`]) rather than
/// privately here, which would put it out of reach of the browser: the editor's server-truth Export
/// preview calls `flatten::flatten_mod_document_json`, and a second hand-written copy of this rule
/// over there would let the preview disagree with this route on the one field the precedence exists
/// to fix. This function is purely the **row → [`MissionMeta`] adapter**; everything downstream of
/// it is shared code, which is what makes the twin honest.
///
/// **Cargo capacity.** Routes through [`flatten_to_mod_document_with_catalog`] with an **empty**
/// catalog (cargo walk is a no-op — never invent limits). The no-arg entry is for unit tests and
/// callers without a pool. Live boundaries that hold the registry phys table must call
/// [`flatten_to_mod_document_with_catalog`] instead:
///
/// * Save — `missions::handlers::mission_versions::validate_payload` → `load_cargo_phys_catalog` →
///   `validate_mission_editor_payload_with_catalog`
/// * `GET /missions/:id/compiled` — `load_cargo_phys_catalog` → this catalogued gate
///
/// so a stored row that predates the capacity walk (and any write that bypassed Save) cannot
/// compile either.
pub fn flatten_to_mod_document(
    m: &Mission,
    payload: &[u8],
) -> Result<ModMissionDocument, CompileError> {
    flatten_to_mod_document_with_catalog(m, payload, &CargoPhysCatalog::new())
}

/// Same compile as [`flatten_to_mod_document`], but the cargo-capacity walk uses `catalog` (the
/// same `resource_name →` phys table Save builds via `load_cargo_phys_catalog`).
///
/// Over-capacity findings become [`CompileError::Parse`] carrying the same `/editor/...` strings
/// Save puts in its 400 `details` — one helper ([`wire_safety::scan_cargo_capacity`]), two
/// boundaries. Empty catalog stays silent (never invent), matching Save.
pub fn flatten_to_mod_document_with_catalog(
    m: &Mission,
    payload: &[u8],
    catalog: &CargoPhysCatalog,
) -> Result<ModMissionDocument, CompileError> {
    if let Ok(instance) = serde_json::from_slice::<serde_json::Value>(payload) {
        let findings = wire_safety::scan_cargo_capacity(&instance, catalog);
        if !findings.is_empty() {
            return Err(CompileError::Parse(findings.join("; ")));
        }
    }

    let mut meta = MissionMeta {
        id: m.id.to_string(),
        title: m.title.clone(),
        author: m.author_id.clone(),
        terrain: m.terrain.as_str().to_string(),
        custom_terrain_name: m.custom_terrain_name.clone(),
        max_players: m.max_players,
        time_of_day: m.time_of_day.clone(),
        weather_preset: m.weather.as_str().to_string(),
    };
    flatten::apply_authored_environment(&mut meta, payload);
    flatten::flatten_to_mod_document(&meta, payload)
}

#[cfg(test)]
#[path = "tests/mission_compile_flatten.rs"]
mod flatten_tests;

#[cfg(test)]
#[path = "tests/mission_compile_diagnostics.rs"]
mod diagnostics_tests;
