use super::super::*;
use super::*;
use crate::execution_metrics::estimated::{self as estimates, EstimatesState};
use crate::ticket_browser::{
    events::BrowserEvent as Action,
    models::{detail_sections as detail, view::BrowserView},
};
use crate::ticket_registry::models::{projection as board, projection::Class};
use eframe::egui::{Align, Layout, RichText, ScrollArea, Ui};
use egui_extras::{Column as TableColumn, TableBuilder};
use std::path::{Path, PathBuf};

pub(crate) fn detail_ui(
    ui: &mut Ui,
    repo_root: Option<&Path>,
    b: &BrowserView<'_>,
    selected: usize,
    action_strip: &mut TicketActionStrip<'_>,
    actions: &mut Vec<Action>,
) {
    let Some(loaded) = b.corpus.tickets.get(selected) else {
        return;
    };
    let v = board::view(&loaded.ticket);
    let ids = &b.board.id_to_index;

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(v.id).monospace().size(16.0).strong());
        ui.label(RichText::new(v.kind).weak().small());
        // class chip — colored accent, absent class renders nothing.
        if let Some(class) = v.class.and_then(Class::parse) {
            ui.label(
                RichText::new(class.as_str())
                    .small()
                    .color(class_color(class)),
            );
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.small_button("✕").clicked() {
                actions.push(Action::CloseDetail);
            }
        });
    });
    ui.label(RichText::new(v.title).size(14.0).strong());
    // Header fields appear without labels beneath the title: main_goal first in its
    // own tint, then summary. Absent header fields are omitted; missing body fields
    // use the explicit absence marker. The detail model owns field order.
    for (field, text) in detail::header_lines(&v) {
        ui.add_space(2.0);
        let rich = if field == detail::BodyField::MainGoal {
            RichText::new(text).size(13.5).color(MAIN_GOAL_TINT)
        } else {
            RichText::new(text)
        };
        ui.label(rich).on_hover_text(field.definition());
    }
    ui.add_space(4.0);
    ui.separator();
    // action strip: offered transitions + Add child + Advanced.
    actions.extend(action_strip(ui).into_iter().map(Action::TicketAction));
    ui.separator();

    ScrollArea::vertical()
        .id_salt("detail")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            match b.compare {
                Some(compare) if compare != selected => {
                    compare_ui(ui, b, selected, compare, actions);
                    ui.separator();
                }
                _ => {
                    // The compare affordance, discoverable where it acts.
                    ui.label(
                        RichText::new("shift-click another ticket to compare owns")
                            .weak()
                            .small(),
                    );
                }
            }
            let resolve = |p: &str| match repo_root {
                Some(root) => root.join(p),
                None => PathBuf::from(p),
            };
            let opt_id =
                |value: Option<&str>| value.map_or(Cell::Missing, |s| Cell::IdRef(s.to_owned()));
            // provenance: each stamp renders measured or estimated as a
            // distinct state, decided per name against estimated[]; the tooltip
            // is the ticket's estimate_note VERBATIM.
            let stamp = |name: &str, value: Option<&str>| {
                Cell::Stamp(estimates::stamp_cell(
                    name,
                    value,
                    v.estimated,
                    v.estimate_note,
                ))
            };
            let mut rows: Vec<(&str, Cell)> = vec![
                ("status", Cell::Text(board::status_label(v.status))),
                (
                    "executor",
                    Cell::Text(v.executor.map_or_else(
                        || format!("{} (default)", board::EXECUTOR_DEFAULT),
                        str::to_owned,
                    )),
                ),
                (
                    "priority",
                    v.priority
                        .map_or(Cell::Missing, |p| Cell::Text(p.to_string())),
                ),
                (
                    "spec",
                    v.spec.map_or(Cell::Missing, |s| Cell::PathRef {
                        label: s.to_owned(),
                        path: resolve(s),
                    }),
                ),
                // the per-ticket plan document — the in-app viewer's
                // primary click (makes it a ready-gate, so live ready
                // tickets always carry one).
                (
                    "plan",
                    v.plan.map_or(Cell::Missing, |p| Cell::PathRef {
                        label: p.to_owned(),
                        path: resolve(p),
                    }),
                ),
                ("parent", opt_id(v.parent)),
                ("active", opt_id(v.active)),
                ("shipped_at", stamp("shipped_at", v.shipped_at)),
                ("created_at", stamp("created_at", v.created_at)),
                ("completed_at", stamp("completed_at", v.completed_at)),
            ];
            // The tokens row exists ONLY when "tokens" ∈ estimated[] — measured
            // tokens live on the Metrics tab over receipts; this panel never
            // renders a figure that could be mistaken for one.
            let est_model = match &b.estimates {
                EstimatesState::Loaded(m) => Some(m),
                EstimatesState::NoEstimates => None,
            };
            if let Some(cell) = estimates::tokens_cell(v.id, v.estimated, est_model) {
                rows.push(("tokens (estimated)", Cell::TokensEstimate(cell)));
            }
            rows.extend([
                (
                    "pack_last",
                    v.pack_last
                        .map_or(Cell::Missing, |p| Cell::Text(p.to_string())),
                ),
                (
                    "scope",
                    v.scope.map_or(Cell::Missing, |s| {
                        Cell::Scope(board::breadcrumb(s, v.estimated))
                    }),
                ),
                (
                    "file",
                    Cell::PathRef {
                        label: loaded.path.display().to_string(),
                        path: loaded.path.clone(),
                    },
                ),
            ]);
            TableBuilder::new(ui)
                .striped(true)
                .vscroll(false)
                .column(TableColumn::auto().at_least(84.0))
                .column(TableColumn::remainder())
                .body(|mut body| {
                    for (label, cell) in &rows {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(RichText::new(*label).weak().small());
                            });
                            row.col(|ui| cell_ui(ui, cell, ids, actions));
                        });
                    }
                });

            // body region below the metadata table, shape:
            // the typed fields MINUS the header pair — separated labeled
            // sections in the PINNED order starting at context — then the
            // migration_legacy quarantine strictly after all of them.
            // `detail::body_region_order` is the one order authority
            // (test-pinned).
            for section in detail::body_region_order() {
                match section {
                    detail::BodySection::Field(field) => {
                        body_section_ui(ui, field, &detail::section_content(field, &v), actions);
                    }
                    detail::BodySection::Quarantine => {
                        quarantine_section_ui(
                            ui,
                            v.id,
                            v.migration_legacy,
                            b.legacy_expanded,
                            actions,
                        );
                    }
                }
            }
            id_list_section(ui, "depends_on", v.depends_on, ids, actions);
            id_list_section(ui, "unblocks", v.unblocks, ids, actions);
            id_list_section(ui, "children", v.children, ids, actions);
            owns_section(ui, v.owns);
            ui.add_space(12.0);
        });
}
