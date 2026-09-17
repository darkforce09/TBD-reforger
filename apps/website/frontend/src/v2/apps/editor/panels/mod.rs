//! The editor's inspectors, drawers and dialogs — the panels that edit a subject rather than
//! frame the map. All ungated: they hold no wasm-only types (doc-driving on:click bodies are
//! cfg-gated inside the closures), so the native view shell compiles them too.

pub mod attributes_modal;
pub mod env;
// T-692 — the Help menu's Controls Hint.
pub mod help_modal;
pub mod settings_modal;
// T-655 — mission validation panel.
pub mod validation_panel;
pub mod vehicles_panel;
// T-936.1 — the Win conditions card: mode picker, per-mode field, endOn checklist. Its section
// belongs beside `settings_modal::render_flow_section` in the Mission Settings dialog; that mount
// is one line in `settings_modal.rs`, which is outside T-936.1's owned files. See the card's own
// header.
pub mod win_conditions_card;
// T-936.2 — the Tasks panel: tier / trigger / marker pickers, undoable list ops. Mounts beside
// win_conditions_card in Mission Settings; that one line lives in settings_modal.rs (outside this
// slice's owns), same as T-936.1's card.
pub mod tasks_panel;
// T-936.3 — the Radio nets panel: authored frequencies, faction assignment, Reset-to-derived.
// Mounts beside tasks_panel in Mission Settings; that one line lives in settings_modal.rs
// (T-946.33, outside this slice's owns), same as T-936.1/.2.
pub mod radio_panel;
// T-936.4 — the Weather timeline panel: keyframes, undoable list ops. Mounts beside radio_panel
// in Mission Settings; that one line lives in settings_modal.rs (outside this slice's owns),
// same as T-936.1/.2/.3.
pub mod weather_timeline;
// T-936.5 — positional audio emitters + music cues. Mounts beside weather_timeline
// in Mission Settings; that one line lives in settings_modal.rs (outside this slice's owns),
// same as T-936.1/.2/.3/.4.
pub mod audio_emitters;
// T-936.6 — wave / garrison spawn modules. Mounts in Mission Settings (this slice owns
// settings_modal.rs) so the panel is not a registered-but-unmounted dead mechanism.
pub mod spawn_modules;
pub mod zones_panel;
