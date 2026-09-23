use super::*;
use crate::core::ui::*;
use crate::document_viewer::services::document_loading as viewer;
use crate::execution_metrics::estimated::{self as estimates, StampCell, TokensCell};
use crate::ticket_browser::events::BrowserEvent as Action;
use crate::ticket_registry::models::projection as board;
use eframe::egui::{RichText, Ui};
use std::{collections::HashMap, path::PathBuf};

// ---- detail panel ----

/// One detail-row value. Identifiers and paths are links; stamp and token cells
/// retain their measured or estimated provenance.
pub(crate) enum Cell {
    Text(String),
    IdRef(String),
    PathRef {
        label: String,
        path: PathBuf,
    },
    /// Scope breadcrumb — the detail tier renders the "(no surface)"
    /// marker and the estimated-scope glyph.
    Scope(board::Breadcrumb),
    /// Lifecycle stamp — measured vs estimated as distinct render
    /// states; the estimated states carry the `~` glyph + verbatim-note tooltip.
    Stamp(StampCell),
    /// The "tokens (estimated)" row — value/source/factor/inputs off
    /// the estimate file, or the explicit marked-but-no-file hole.
    TokensEstimate(TokensCell),
    Missing,
}

pub(crate) fn cell_ui(
    ui: &mut Ui,
    cell: &Cell,
    ids: &HashMap<String, usize>,
    actions: &mut Vec<Action>,
) {
    match cell {
        Cell::Text(s) => {
            ui.label(s);
        }
        Cell::IdRef(id) => id_link_ui(ui, id, ids, actions),
        Cell::PathRef { label, path } => {
            // `.md` paths (spec/plan) open the in-app viewer; anything
            // else (the.toml file row) keeps the external OS-handler hop.
            let in_app = viewer::wants_viewer(label);
            let hover = if in_app {
                "open in the board viewer"
            } else {
                "open with the OS handler (xdg-open)"
            };
            if ui
                .link(RichText::new(label).monospace())
                .on_hover_text(hover)
                .clicked()
            {
                if in_app {
                    actions.push(Action::OpenDoc(label.clone()));
                } else {
                    actions.push(Action::OpenPath(path.clone()));
                }
            }
        }
        Cell::Scope(bc) => scope_breadcrumb_ui(ui, bc),
        Cell::Stamp(stamp) => stamp_cell_ui(ui, stamp),
        Cell::TokensEstimate(cell) => tokens_cell_ui(ui, cell),
        Cell::Missing => missing_marker(ui),
    }
}

/// The amber provenance glyph (`~`) with its tooltip — shared by the stamp and
/// tokens rows so every estimated value reads in one visual language.
pub(crate) fn estimate_glyph_ui(ui: &mut Ui, tip: &str) {
    ui.label(
        RichText::new(estimates::ESTIMATE_GLYPH)
            .strong()
            .color(SCOPE_ESTIMATED_COLOR),
    )
    .on_hover_text(tip);
}

/// One lifecycle-stamp cell: measured renders as a bare mono value —
/// NO glyph, NO tooltip; estimated renders the glyph + the estimate_note
/// verbatim on hover; absent-but-marked renders the explicit
/// "— (estimated absent)" marker (a SHA is never invented).
pub(crate) fn stamp_cell_ui(ui: &mut Ui, cell: &StampCell) {
    match cell {
        StampCell::Measured(value) => {
            ui.monospace(value);
        }
        StampCell::Estimated { value, tip } => {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                estimate_glyph_ui(ui, tip);
                ui.monospace(value).on_hover_text(tip);
            });
        }
        StampCell::AbsentEstimated { tip } => {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                estimate_glyph_ui(ui, tip);
                ui.label(
                    RichText::new(estimates::ABSENT_ESTIMATED_MARKER)
                        .weak()
                        .italics(),
                )
                .on_hover_text(tip);
            });
        }
        StampCell::Absent => missing_marker(ui),
    }
}

/// The "tokens (estimated)" row: value · source ×factor · inputs, all
/// from the estimate file, tooltip naming source/factor/inputs/generated_at —
/// or the explicit marked-but-no-file hole. Never a measured figure.
pub(crate) fn tokens_cell_ui(ui: &mut Ui, cell: &TokensCell) {
    match cell {
        TokensCell::Estimated(detail) => {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                estimate_glyph_ui(ui, &detail.tip);
                ui.monospace(&detail.value_str).on_hover_text(&detail.tip);
                ui.label(
                    RichText::new(format!(
                        "· {} ×{} · {}",
                        detail.source, detail.factor, detail.inputs_str
                    ))
                    .weak()
                    .small(),
                )
                .on_hover_text(&detail.tip);
            });
        }
        TokensCell::MissingFile { tip } => {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                estimate_glyph_ui(ui, tip);
                ui.label(RichText::new("— (no estimate file)").weak().italics())
                    .on_hover_text(tip);
            });
        }
    }
}

/// Detail-tier breadcrumb: per-level muted accents, the explicit
/// "(no surface)" marker when a component carries no surface (detail ONLY —
/// cards omit it), and the ~ glyph with its owns-inferred tooltip. Single line
/// (the table row is fixed-height); hovering the row shows the plain-text path.
pub(crate) fn scope_breadcrumb_ui(ui: &mut Ui, bc: &board::Breadcrumb) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if bc.estimated {
            ui.label(
                RichText::new(board::SCOPE_ESTIMATED_GLYPH)
                    .small()
                    .color(SCOPE_ESTIMATED_COLOR),
            )
            .on_hover_text(board::SCOPE_ESTIMATED_TIP);
        }
        for (i, seg) in bc.segs.iter().enumerate() {
            if i > 0 {
                ui.label(RichText::new(board::SCOPE_SEP).weak().small());
            }
            ui.label(
                RichText::new(&seg.text)
                    .small()
                    .color(scope_level_color(seg.level)),
            );
        }
        if bc.no_surface {
            ui.label(
                RichText::new(board::NO_SURFACE_MARKER)
                    .weak()
                    .small()
                    .italics(),
            );
        }
    })
    .response
    .on_hover_text(bc.label());
}
