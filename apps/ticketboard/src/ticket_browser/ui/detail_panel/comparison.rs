use super::*;
use crate::core::ui::*;
use crate::ticket_browser::{events::BrowserEvent as Action, models::view::BrowserView};
use crate::ticket_registry::models::projection as board;
use crate::wave_plan::services::lock_file as wavelock;
use eframe::egui::{Align, Layout, RichText, Ui};
use std::collections::HashSet;

/// Owns-collision explainer: with exactly two tickets selected,
/// both owns lists plus EVERY colliding pair under the prefix-containment rule
/// (`wavelock::paths_collide`, the `wave_lock::collides` mirror) and the verdict —
/// why these two can never share a wave, or that they can.
pub(crate) fn compare_ui(
    ui: &mut Ui,
    b: &BrowserView<'_>,
    selected: usize,
    compare: usize,
    actions: &mut Vec<Action>,
) {
    let a = board::view(&b.corpus.tickets[selected].ticket);
    let z = board::view(&b.corpus.tickets[compare].ticket);
    ui.horizontal(|ui| {
        ui.label(RichText::new("owns collision").strong().small());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.small_button("✕ stop comparing").clicked() {
                actions.push(Action::ClearCompare);
            }
        });
    });
    ui.monospace(format!("{}  vs  {}", a.id, z.id));
    // The verdict IS the mirrored rule; the pairs are its explanation.
    let pairs = wavelock::colliding_pairs(a.owns, z.owns);
    if !wavelock::collides(a.owns, z.owns) {
        ui.label(RichText::new("no collision").color(VERDICT_OK).strong());
        ui.label(
            RichText::new(
                "owns paths are disjoint — the packer may put these two in the same wave",
            )
            .weak()
            .small(),
        );
    } else {
        ui.label(
            RichText::new("never the same wave")
                .color(VERDICT_COLLIDE)
                .strong(),
        );
        ui.label(
            RichText::new(
                "colliding pairs — equal, or one prefix-contains the other on a '/' boundary:",
            )
            .weak()
            .small(),
        );
        for (x, y) in &pairs {
            ui.label(
                RichText::new(format!("{x}  ×  {y}"))
                    .monospace()
                    .small()
                    .color(VERDICT_COLLIDE),
            );
        }
    }
    let left: HashSet<&String> = pairs.iter().map(|(x, _)| x).collect();
    let right: HashSet<&String> = pairs.iter().map(|(_, y)| y).collect();
    owns_compare_list(ui, a.id, a.owns, &left);
    owns_compare_list(ui, z.id, z.owns, &right);
    ui.add_space(6.0);
}

pub(crate) fn owns_compare_list(
    ui: &mut Ui,
    id: &str,
    owns: &[String],
    colliding: &HashSet<&String>,
) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(format!("{id} owns ({})", owns.len()))
            .strong()
            .small(),
    );
    if owns.is_empty() {
        missing_marker(ui);
        return;
    }
    for path in owns {
        let mut text = RichText::new(path).monospace().small();
        if colliding.contains(path) {
            text = text.color(VERDICT_COLLIDE);
        }
        ui.label(text);
    }
}
