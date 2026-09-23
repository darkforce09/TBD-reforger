use super::*;
use crate::core::ui::*;
use crate::document_viewer::services::document_loading as viewer;
use crate::ticket_browser::{events::BrowserEvent as Action, models::detail_sections as detail};
use eframe::egui::{Align, Frame, Layout, RichText, Stroke, Ui};

pub(crate) fn section_header(ui: &mut Ui, title: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(title).strong().small());
}

/// The explicit muted em-dash an absent field renders — the label always stays;
/// absence is data, never a silently skipped section.
pub(crate) fn missing_marker(ui: &mut Ui) {
    ui.label(RichText::new(detail::ABSENT_MARKER).weak());
}

/// One of the ten pinned body sections: the label row — with the entry
/// count on nonempty lists — carries the field's one-line anti-blend definition
/// as its hover tooltip (the acceptance-vs-verify distinction lives exactly
/// here); content renders as wrapped text (scalars), numbered monospace lines
/// (lists), or the muted em-dash when absent. in the citations section
/// a `.md` entry renders its numbered line as a link into the in-app viewer;
/// non-`.md` citations keep the plain monospace line they always were.
pub(crate) fn body_section_ui(
    ui: &mut Ui,
    field: detail::BodyField,
    content: &detail::SectionContent,
    actions: &mut Vec<Action>,
) {
    let label = match content {
        detail::SectionContent::Lines(lines) => format!("{} ({})", field.as_str(), lines.len()),
        detail::SectionContent::Absent | detail::SectionContent::Text(_) => {
            field.as_str().to_owned()
        }
    };
    ui.add_space(10.0);
    ui.label(RichText::new(label).strong().small())
        .on_hover_text(field.definition());
    let md_links = field == detail::BodyField::Citations;
    match content {
        detail::SectionContent::Absent => missing_marker(ui),
        detail::SectionContent::Text(text) => {
            ui.label(text);
        }
        detail::SectionContent::Lines(lines) => {
            for (numbered, entry) in detail::numbered_lines(lines).into_iter().zip(lines) {
                if md_links && viewer::wants_viewer(entry) {
                    if ui
                        .link(RichText::new(numbered).monospace().small())
                        .on_hover_text("open in the board viewer")
                        .clicked()
                    {
                        actions.push(Action::OpenDoc(entry.clone()));
                    }
                } else {
                    ui.label(RichText::new(numbered).monospace().small());
                }
            }
        }
    }
}

/// The `migration_legacy` quarantine — rendered ONLY when parked prose
/// exists (no quarantine is the healthy state, unlike the ten body fields whose
/// absence is an explicit em-dash). Visually fenced with the amber tint + border
/// so unprocessed v1 wall text never reads as authored body: verbatim lines,
/// collapsed by default beyond [`detail::LEGACY_COLLAPSE_THRESHOLD`], and the
/// "Copy for triage" affordance — the Program T drain feed (id + verbatim legacy
/// + the empty ten-field skeleton) onto the clipboard.
pub(crate) fn quarantine_section_ui(
    ui: &mut Ui,
    id: &str,
    legacy: &[String],
    expanded: bool,
    actions: &mut Vec<Action>,
) {
    if legacy.is_empty() {
        return;
    }
    ui.add_space(10.0);
    Frame::new()
        .fill(QUARANTINE_TINT)
        .stroke(Stroke::new(1.0, QUARANTINE_BORDER))
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("migration_legacy ({})", legacy.len()))
                        .strong()
                        .small(),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .small_button("Copy for triage")
                        .on_hover_text(
                            "copy ticket id + verbatim legacy + the empty ten-field \
                             skeleton — the Program T drain block",
                        )
                        .clicked()
                    {
                        actions.push(Action::CopyText(detail::triage_block(id, legacy)));
                    }
                });
            });
            ui.label(
                RichText::new(detail::QUARANTINE_LABEL)
                    .small()
                    .color(SCOPE_ESTIMATED_COLOR),
            )
            .on_hover_text(
                "verbatim v1 wall prose parked byte-reversibly at migration — \
                 Program T decomposes it into the ten typed fields and deletes \
                 this section in the same edit",
            );
            let (visible, hidden) = detail::legacy_visible(legacy.len(), expanded);
            for line in &legacy[..visible] {
                ui.label(RichText::new(line).monospace().small());
            }
            if hidden > 0 {
                if ui
                    .small_button(format!("expand — {hidden} more line(s)"))
                    .clicked()
                {
                    actions.push(Action::ToggleLegacyExpand);
                }
            } else if legacy.len() > detail::LEGACY_COLLAPSE_THRESHOLD
                && ui.small_button("collapse").clicked()
            {
                actions.push(Action::ToggleLegacyExpand);
            }
        });
}
