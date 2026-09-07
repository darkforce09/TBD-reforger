//! Mission compiler — shared between the Axum backend (`/compiled` + event ORBAT derivation) and
//! the wasm client. Ported from the backend `services/mission_payload.rs` + `contract/kit_aliases.rs`
//! (T-145 Phase 2); the backend now re-exports these from here. Gated behind the `mission` feature
//! (serde/serde_json) so the DEM-only wasm/backend builds don't pull it. The mod-document flatten
//! (`flatten_to_mod_document`) lands here next, once decoupled from the backend `Mission` model.

/// T-936.5 — the authored `audio` block: emitters, music cues, radius/event/id gates, and the
/// validator [`extensions::AUTHORED_BLOCKS`] registers for it.
pub mod audio;
pub mod compile;
/// T-936 — the AUTHORED_BLOCKS passthrough: the one list `compile.rs` copies from and `flatten.rs`
/// reads back, so the seven T-936 blocks land without seven pairs of edits to those two contested
/// files. See its header for the list-not-open-passthrough rule.
pub mod extensions;
pub mod flatten;
pub mod kit;
pub mod orbat;
/// T-936.3 — the authored `radioPlan` block: nets, frequencies, duplicate/range gates, and the
/// validator [`extensions::AUTHORED_BLOCKS`] registers for it.
pub mod radio_plan;
/// T-936.6 — the authored `spawnModules[]` block: wave/garrison modules, exclusive placement,
/// known factions, positive counts, and the validator [`extensions::AUTHORED_BLOCKS`] registers
/// for it.
pub mod spawn_modules;
/// T-936.2 — the authored `tasks[]` block: tiers, the assigned→succeeded|failed table, and the
/// validator [`extensions::AUTHORED_BLOCKS`] registers for it.
pub mod tasks;
pub mod validate;
/// T-936.4 — the authored `weatherTimeline` block: keyframes, strictly increasing `atMinutes`,
/// the preset vocabulary shared with `environment.weatherPreset`, and the validator
/// [`extensions::AUTHORED_BLOCKS`] registers for it.
pub mod weather;
/// T-936.1 — the authored `winConditions` block: the five modes the editor may author, the
/// per-mode params, and the validator [`extensions::AUTHORED_BLOCKS`] registers for it.
pub mod win_conditions;
pub mod wire_safety;
