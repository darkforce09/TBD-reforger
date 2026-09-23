use eframe::egui::{Color32, RichText, Ui};
/// Collision verdict / `ready`-family accent colors (dark-theme legible).
/// pub(crate): the mutation UI reuses the same green/red pair.
pub(crate) const VERDICT_OK: Color32 = Color32::from_rgb(120, 205, 130);

pub(crate) const VERDICT_COLLIDE: Color32 = Color32::from_rgb(235, 110, 100);

/// Amber accent for the estimated-scope `~` glyph (the trust banner's "busy"
/// tone — provenance flags read as attention, not error). reuses it for
/// EVERY estimated surface: stamp glyphs, the tokens (estimated) row, and the
/// Estimated (historical) panel header — one provenance color language.
pub(crate) const SCOPE_ESTIMATED_COLOR: Color32 = Color32::from_rgb(245, 175, 80);

pub(crate) const OUTPUT_ROW_H: f32 = 15.0;

/// A monospace identifier becomes a link only when its target exists.
pub(crate) fn identifier_link(ui: &mut Ui, label: &str, enabled: bool) -> bool {
    if enabled {
        ui.link(RichText::new(label).monospace()).clicked()
    } else {
        ui.monospace(label);
        false
    }
}
