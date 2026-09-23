use super::*;
use crate::core::ui::*;
use crate::execution_metrics::{
    estimated::{self as estimates, EstimatedTableKind, EstimatesState},
    events::MetricsEvent as Action,
    models::MetricsView,
};
use eframe::egui::{RichText, Ui};
use egui_extras::{Column as TableColumn, TableBuilder};

// ---- estimated (historical) panel ----

/// The ESTIMATED half of the Metrics tab: its own amber-tinted header, its own
/// grand strip, per-CLASS and per-DOMAIN tables (estimates have no agent), its
/// own malformed-file list — and no figure shared with the measured panel
/// above. The explicit no-estimates state renders instead of zeros.
pub(crate) fn estimated_panel_ui(ui: &mut Ui, b: &MetricsView<'_>, actions: &mut Vec<Action>) {
    panel_badge_ui(ui, " ESTIMATED (historical) ", SCOPE_ESTIMATED_COLOR);
    ui.label(
        RichText::new(estimates::NEVER_COMBINED_NOTE)
            .small()
            .color(SCOPE_ESTIMATED_COLOR),
    );
    match &b.estimates {
        EstimatesState::NoEstimates => {
            ui.add_space(8.0);
            ui.label(
                RichText::new(estimates::no_estimates_text())
                    .monospace()
                    .size(14.0),
            );
            ui.label(
                RichText::new(
                    "the panel renders real estimate files only — an empty tree is this \
                     message, not zeros",
                )
                .weak()
                .small(),
            );
        }
        EstimatesState::Loaded(e) => {
            ui.add_space(4.0);
            // The ESTIMATED grand strip — separate from the measured strip; the
            // two are never one figure.
            ui.label(RichText::new(&e.grand.strip).monospace().strong());
            if !e.errors.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "{} malformed estimate file(s) — excluded from every sum; listed below",
                        e.errors.len()
                    ))
                    .color(VERDICT_COLLIDE)
                    .strong(),
                );
            }
            ui.separator();
            ui.label(RichText::new("Per class").strong());
            est_table_ui(ui, b, &e.per_class, EstimatedTableKind::Class, actions);
            ui.add_space(10.0);
            ui.separator();
            ui.label(RichText::new("Per domain").strong());
            est_table_ui(ui, b, &e.per_domain, EstimatedTableKind::Domain, actions);
            if !e.errors.is_empty() {
                ui.add_space(10.0);
                ui.separator();
                ui.label(
                    RichText::new(format!("Malformed estimates ({})", e.errors.len())).strong(),
                );
                ui.label(
                    RichText::new(
                        "named per file, reason verbatim — never silently skipped, never \
                         coerced to numbers",
                    )
                    .weak()
                    .small(),
                );
                for error in &e.errors {
                    ui.label(
                        RichText::new(&error.rel)
                            .monospace()
                            .small()
                            .color(VERDICT_COLLIDE),
                    );
                    ui.label(RichText::new(&error.reason).monospace().small());
                }
            }
        }
    }
}

/// One ESTIMATED aggregation table (per class / per domain): tickets,
/// Σ tokens_estimated and the source split, headers sortable. Key cells are
/// plain text — classes and domains are buckets, not tickets, so nothing links
/// into the detail panel from here.
pub(crate) fn est_table_ui(
    ui: &mut Ui,
    b: &MetricsView<'_>,
    rows: &[estimates::EstimatedRow],
    table: EstimatedTableKind,
    actions: &mut Vec<Action>,
) {
    if rows.is_empty() {
        ui.label(RichText::new("—").weak());
        return;
    }
    let (sort, key_header, salt) = match table {
        EstimatedTableKind::Class => (b.est_sort.class, "class", "est_per_class"),
        EstimatedTableKind::Domain => (b.est_sort.domain, "domain", "est_per_domain"),
    };
    ui.push_id(salt, |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .vscroll(false)
            .column(TableColumn::auto().at_least(120.0)) // key
            .column(TableColumn::auto().at_least(64.0)) // tickets
            .column(TableColumn::auto().at_least(160.0)) // tokens_estimated
            .column(TableColumn::auto().at_least(80.0)) // diff_loc
            .column(TableColumn::remainder()) // cohort_median
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label(RichText::new(key_header).weak().small());
                });
                header.col(|ui| {
                    est_sort_header_ui(
                        ui,
                        "tickets",
                        estimates::EstimatedSortKey::Tickets,
                        sort,
                        table,
                        actions,
                    );
                });
                header.col(|ui| {
                    est_sort_header_ui(
                        ui,
                        "tokens_estimated (Σ)",
                        estimates::EstimatedSortKey::Tokens,
                        sort,
                        table,
                        actions,
                    );
                });
                header.col(|ui| {
                    est_sort_header_ui(
                        ui,
                        "diff_loc",
                        estimates::EstimatedSortKey::DiffLoc,
                        sort,
                        table,
                        actions,
                    );
                });
                header.col(|ui| {
                    est_sort_header_ui(
                        ui,
                        "cohort_median",
                        estimates::EstimatedSortKey::CohortMedian,
                        sort,
                        table,
                        actions,
                    );
                });
            })
            .body(|mut body| {
                for agg in rows {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.monospace(&agg.key);
                        });
                        row.col(|ui| {
                            ui.monospace(&agg.tickets_str);
                        });
                        row.col(|ui| {
                            // Amber-glyphed — an estimated figure never dresses
                            // as a measured one, even in its own table.
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                ui.label(
                                    RichText::new(estimates::ESTIMATE_GLYPH)
                                        .color(SCOPE_ESTIMATED_COLOR),
                                );
                                ui.monospace(&agg.tokens_str);
                            });
                        });
                        row.col(|ui| {
                            ui.monospace(&agg.diff_loc_str);
                        });
                        row.col(|ui| {
                            ui.monospace(&agg.cohort_str);
                        });
                    });
                }
            });
    });
}

/// Sortable header for the estimated tables — same click rule as the measured
/// tables (`sort_header_ui`), separate action + separate sort state.
pub(crate) fn est_sort_header_ui(
    ui: &mut Ui,
    label: &str,
    key: estimates::EstimatedSortKey,
    sort: estimates::EstimatedSort,
    table: EstimatedTableKind,
    actions: &mut Vec<Action>,
) {
    let active = sort.key == key;
    let text = if active {
        format!("{label} {}", if sort.desc { "▼" } else { "▲" })
    } else {
        label.to_owned()
    };
    if ui
        .selectable_label(active, RichText::new(text).small())
        .clicked()
    {
        actions.push(Action::SortEstimates(table, key));
    }
}
