use super::*;
use crate::core::ui::*;
use crate::execution_metrics::{
    events::MetricsEvent as Action,
    measured::{self as metrics, TableKind},
    models::MetricsView,
};
use eframe::egui::{RichText, Ui};
use egui_extras::{Column as TableColumn, TableBuilder};

/// The acceptance-1 surface: an ABSENT (or empty) receipts directory is
/// this explicit state — never a table of zeros.
pub(crate) fn metrics_empty_ui(ui: &mut Ui) {
    ui.add_space(24.0);
    ui.heading("No receipts yet");
    ui.add_space(8.0);
    ui.label(
        RichText::new(metrics::no_receipts_text())
            .monospace()
            .size(14.0),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "The dashboard renders real run files only — tokens are never invented, \
             so an empty tree is this message, not zeros.",
        )
        .weak()
        .small(),
    );
    ui.add_space(4.0);
    ui.label(RichText::new(metrics::COVERAGE_NOTE).weak().small());
}

/// The measured receipts body — content unchanged by; only
/// the scroll container moved up to `metrics_ui` so the estimated panel shares
/// one scroll surface (never one table).
pub(crate) fn metrics_body_ui(
    ui: &mut Ui,
    b: &MetricsView<'_>,
    m: &metrics::MetricsModel,
    actions: &mut Vec<Action>,
) {
    ui.add_space(4.0);
    // Grand-total strip (precomputed at load; with zero valid runs it
    // says "no valid receipts", never a zeros row).
    ui.label(RichText::new(&m.grand.strip).monospace().strong());
    ui.label(RichText::new(metrics::COVERAGE_NOTE).weak().small());
    if !m.errors.is_empty() {
        ui.label(
            RichText::new(format!(
                "{} malformed receipt file(s) — excluded from every sum; listed below",
                m.errors.len()
            ))
            .color(VERDICT_COLLIDE)
            .strong(),
        );
    }
    ui.separator();
    ui.label(RichText::new("Per agent").strong());
    metrics_table_ui(ui, b, &m.per_agent, TableKind::Agent, actions);
    ui.add_space(10.0);
    ui.separator();
    ui.label(RichText::new("Per ticket").strong());
    metrics_table_ui(ui, b, &m.per_ticket, TableKind::Ticket, actions);
    if !m.errors.is_empty() {
        ui.add_space(10.0);
        ui.separator();
        ui.label(RichText::new(format!("Malformed receipts ({})", m.errors.len())).strong());
        ui.label(
            RichText::new(
                "named per file, reason verbatim — observations are listed broken, \
                 never silently skipped and never coerced to numbers",
            )
            .weak()
            .small(),
        );
        for error in &m.errors {
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

/// One aggregation table (per agent / per ticket): egui_extras striped, the
/// runs / tokens / elapsed headers sort on click. Ticket ids link into the
/// detail panel through the existing selection plumbing.
pub(crate) fn metrics_table_ui(
    ui: &mut Ui,
    b: &MetricsView<'_>,
    rows: &[metrics::MeasuredRow],
    table: TableKind,
    actions: &mut Vec<Action>,
) {
    if rows.is_empty() {
        ui.label(RichText::new("—").weak());
        return;
    }
    let (sort, key_header, salt) = match table {
        TableKind::Ticket => (b.metrics_sort.ticket, "ticket", "metrics_per_ticket"),
        TableKind::Agent => (b.metrics_sort.agent, "agent", "metrics_per_agent"),
    };
    ui.push_id(salt, |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .vscroll(false)
            .column(TableColumn::auto().at_least(96.0)) // key
            .column(TableColumn::auto().at_least(52.0)) // runs
            .column(TableColumn::auto().at_least(150.0)) // tokens
            .column(TableColumn::auto().at_least(96.0)) // elapsed
            .column(TableColumn::auto().at_least(130.0)) // in flight / unfinished
            .column(TableColumn::auto().at_least(170.0)) // first started
            .column(TableColumn::remainder()) // last finished
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label(RichText::new(key_header).weak().small());
                });
                header.col(|ui| {
                    sort_header_ui(ui, "runs", metrics::SortKey::Runs, sort, table, actions);
                });
                header.col(|ui| {
                    sort_header_ui(
                        ui,
                        "tokens_consumed.total",
                        metrics::SortKey::Tokens,
                        sort,
                        table,
                        actions,
                    );
                });
                header.col(|ui| {
                    sort_header_ui(
                        ui,
                        "elapsed",
                        metrics::SortKey::Elapsed,
                        sort,
                        table,
                        actions,
                    );
                });
                header.col(|ui| {
                    ui.label(RichText::new("in flight / unfinished").weak().small())
                        .on_hover_text(
                            "runs with no finished stamp — counted, never folded into elapsed",
                        );
                });
                header.col(|ui| {
                    ui.label(RichText::new("first started").weak().small());
                });
                header.col(|ui| {
                    ui.label(RichText::new("last finished").weak().small());
                });
            })
            .body(|mut body| {
                for agg in rows {
                    body.row(20.0, |mut row| {
                        row.col(|ui| match table {
                            TableKind::Ticket => {
                                id_link_ui(ui, &agg.key, b.id_to_index, actions);
                            }
                            TableKind::Agent => {
                                ui.monospace(&agg.key);
                            }
                        });
                        row.col(|ui| {
                            ui.monospace(&agg.runs_str);
                        });
                        row.col(|ui| {
                            ui.monospace(&agg.tokens_str);
                        });
                        row.col(|ui| {
                            // "—" while nothing finished — an all-in-flight key
                            // has UNKNOWN elapsed, and 0s would fabricate one.
                            ui.monospace(&agg.elapsed_str);
                        });
                        row.col(|ui| {
                            ui.monospace(&agg.unfinished_str);
                        });
                        row.col(|ui| {
                            ui.monospace(&agg.min_started);
                        });
                        row.col(|ui| match &agg.max_finished {
                            Some(fin) => {
                                ui.monospace(fin);
                            }
                            None => {
                                ui.label(RichText::new("—").weak());
                            }
                        });
                    });
                }
            });
    });
}

/// A sortable column header: click toggles direction on the active column and
/// starts descending on a new one; the arrow marks the active sort.
pub(crate) fn sort_header_ui(
    ui: &mut Ui,
    label: &str,
    key: metrics::SortKey,
    sort: metrics::Sort,
    table: TableKind,
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
        actions.push(Action::SortMetrics(table, key));
    }
}
