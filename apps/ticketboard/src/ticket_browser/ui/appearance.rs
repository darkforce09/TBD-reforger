use crate::ticket_registry::models::{
    palette::{scope_level_rgb, status_rgb},
    projection::{Class, ScopeLevel},
};
use eframe::egui::Color32;
use ticket_engine::StatusName;

pub(crate) fn status_color(status: StatusName) -> Color32 {
    let (r, g, b) = status_rgb(status);
    Color32::from_rgb(r, g, b)
}
pub(crate) fn scope_level_color(level: ScopeLevel) -> Color32 {
    let (r, g, b) = scope_level_rgb(level);
    Color32::from_rgb(r, g, b)
}

/// Class chip accent — the palette lives in `board::Class::accent_rgb` (pure,
/// test-pinned, total over the closed class set); this only lifts it to Color32.
pub(crate) fn class_color(class: Class) -> Color32 {
    let (r, g, b) = class.accent_rgb();
    Color32::from_rgb(r, g, b)
}

/// Quarantine fence for the `migration_legacy` block — the amber
/// attention family (same hue as the estimated-scope glyph) washed to a
/// background tint + border, so parked v1 wall prose reads as fenced-off
/// pending-triage material, never as authored body content.
pub(crate) const QUARANTINE_TINT: Color32 = Color32::from_rgb(54, 44, 26);

pub(crate) const QUARANTINE_BORDER: Color32 = Color32::from_rgb(150, 112, 52);

/// Header main_goal tint — a soft teal used nowhere else in the
/// palette, so the goal line under the title reads as its own surface: not a
/// status accent, not the amber provenance family, not body text.
pub(crate) const MAIN_GOAL_TINT: Color32 = Color32::from_rgb(150, 205, 190);
