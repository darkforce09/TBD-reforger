use super::*;
use crate::ticket_browser::services::{
    filtering::{Filters, KindFilter},
    scope_facets::{FacetOption, FacetOptions},
};
use crate::ticket_registry::models::{projection as board, projection::Class};
use eframe::egui::{Button, ComboBox, RichText, TextEdit, Ui};

/// One scope-facet dropdown. Options are the narrowed vocab ∪ corpus
/// union from `facets::compute`; corpus values the vocabulary does not know are
/// marked — display-only marking, `ticket check` stays the validation authority.
pub(crate) fn facet_combo(
    ui: &mut Ui,
    salt: &str,
    any_label: &str,
    sel: &mut Option<String>,
    options: &[FacetOption],
) {
    ComboBox::from_id_salt(salt)
        .selected_text(sel.clone().unwrap_or_else(|| any_label.to_owned()))
        .show_ui(ui, |ui| {
            ui.selectable_value(sel, None, any_label);
            for opt in options {
                let text = if opt.vocab_unknown {
                    RichText::new(format!("{} (not in vocab)", opt.value)).italics()
                } else {
                    RichText::new(&opt.value)
                };
                ui.selectable_value(sel, Some(opt.value.clone()), text);
            }
        });
}

/// Composable filter bar — mutates `filters` in place; returns true when anything
/// changed this frame (the caller then refilters once, outside the paint).
pub(crate) fn filter_bar_ui(
    ui: &mut Ui,
    filters: &mut Filters,
    executors: &[String],
    facet_options: &FacetOptions,
) -> bool {
    let before = filters.clone();
    ui.horizontal_wrapped(|ui| {
        ui.add(
            TextEdit::singleline(&mut filters.text)
                .desired_width(190.0)
                .hint_text("filter id / title / summary"),
        );
        ComboBox::from_id_salt("executor_filter")
            .selected_text(filters.executor.as_deref().unwrap_or("any executor"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut filters.executor, None, "any executor");
                for executor in executors {
                    ui.selectable_value(&mut filters.executor, Some(executor.clone()), executor);
                }
            });
        ComboBox::from_id_salt("kind_filter")
            .selected_text(filters.kind.label())
            .show_ui(ui, |ui| {
                for kind in KindFilter::ALL {
                    ui.selectable_value(&mut filters.kind, kind, kind.label());
                }
            });
        // scope facets — higher selections narrow the lower dropdowns
        // (recomputed in refilter, where stale lower picks are also cleared).
        facet_combo(
            ui,
            "domain_facet",
            "any domain",
            &mut filters.scope.domain,
            &facet_options.domains,
        );
        facet_combo(
            ui,
            "layer_facet",
            "any layer",
            &mut filters.scope.layer,
            &facet_options.layers,
        );
        facet_combo(
            ui,
            "component_facet",
            "any component",
            &mut filters.scope.component,
            &facet_options.components,
        );
        facet_combo(
            ui,
            "surface_facet",
            "any surface",
            &mut filters.scope.surface,
            &facet_options.surfaces,
        );
        // class facet — the closed 5-value set, chip-colored.
        ComboBox::from_id_salt("class_facet")
            .selected_text(filters.class.map_or("any class", Class::as_str))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut filters.class, None, "any class");
                for class in Class::ALL {
                    ui.selectable_value(
                        &mut filters.class,
                        Some(class),
                        RichText::new(class.as_str()).color(class_color(class)),
                    );
                }
            });
        for (i, status) in board::STATUS_ORDER.iter().enumerate() {
            let on = filters.statuses[i];
            let text = if on {
                RichText::new(status.as_str())
                    .small()
                    .color(status_color(*status))
            } else {
                RichText::new(status.as_str()).small().weak()
            };
            if ui.selectable_label(on, text).clicked() {
                filters.statuses[i] = !on;
            }
        }
        ui.add(
            TextEdit::singleline(&mut filters.parent)
                .desired_width(90.0)
                .hint_text("parent id"),
        );
        // One-click clear — restores the full measured count.
        if ui
            .add_enabled(filters.is_active(), Button::new("clear"))
            .clicked()
        {
            filters.clear();
        }
    });
    *filters != before
}
