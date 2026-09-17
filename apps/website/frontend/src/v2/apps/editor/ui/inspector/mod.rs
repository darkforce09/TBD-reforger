//! The inspectors: the surfaces that edit whatever the operator has selected or authored.
//!
//! **Role:** the panels, cards and dialogs that read one subject out of the document and write it
//! back — a slot's transform and identity, a zone's shape and rules, the placed vehicles, the
//! mission-wide environment bag (weather, radio nets, tasks, audio, spawn waves, win conditions)
//! and the validation drawer that reports what the compiler found.
//! **Position:** a leaf of `v2::apps::editor::ui`. The docks and the settings dialog mount these
//! sections; the sections themselves mount nothing outside this module.
//! **Signals & state:** none of their own beyond the validation drawer's published finding list.
//! Every field re-reads the document on each document-version bump, so an undo taken while a
//! panel is open refreshes the fields rather than stranding them.
//! **Invariants:** a commit is one undo step, and it reaches the document through the map
//! engine's hosted commands — never by mutating a row in place. All eleven modules are ungated:
//! the wasm-only bodies sit inside the views and event closures, so the native test build still
//! compiles the pure half of each panel.

/// The attributes modal opened from a slot on the map or an outliner row: the transform,
/// identity, states and arsenal tabs that edit one selected slot.
pub mod attributes_modal;
/// The audio panel: positional emitters and music cues, and the map gesture that places an
/// emitter by clicking the terrain.
pub mod audio_emitters;
/// The environment authoring policy: the table of `meta.environment` keys the editor is allowed
/// to write, each paired with the surface that reads it back, and the mission-flow block.
pub mod env;
/// The radio nets panel: authored frequencies with their faction assignment and range, plus the
/// action that resets the plan back to the derived one.
pub mod radio_panel;
/// The spawn modules panel: the wave and garrison spawners a mission arms.
pub mod spawn_modules;
/// The tasks panel: primary, secondary and optional assignments with their trigger and marker
/// pickers.
pub mod tasks_panel;
/// The validation drawer: the persistent issue list, its severity rollup and the router that
/// selects the subject a finding names.
pub mod validation_panel;
/// The placed-vehicles panel: every vehicle row with its position, heading, cargo editor and
/// delete.
pub mod vehicles_panel;
/// The weather timeline panel: the keyframes that drive preset, wind and fog over mission time.
pub mod weather_timeline;
/// The win conditions card: the mode picker, the per-mode field and the checklist that decides
/// how a round ends.
pub mod win_conditions_card;
/// The zones panel: the zone draw tool, the shape and rule vocabularies read from the mission
/// schema, and the per-zone attribute controls.
pub mod zones_panel;
