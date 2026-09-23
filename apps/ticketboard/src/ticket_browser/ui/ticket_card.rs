use super::status_board::CARD_H;
use super::*;
use crate::core::ui::*;
use crate::ticket_browser::models::status_board::Card;
use crate::ticket_registry::models::projection as board;
use eframe::egui::{self, Align2, FontId, Rect, Sense, StrokeKind, Ui, pos2, vec2};

/// Card paint: one allocated rect, painter-only text — no nested widgets, so a
/// virtualized column stays well inside the 17 ms frame budget. `draggable`
/// (idea cards, ) adds drag sense for the drag-onto-queued affordance.
pub(crate) fn card_ui(ui: &mut Ui, card: &Card, selected: bool, draggable: bool) -> egui::Response {
    let width = ui.available_width();
    let sense = if draggable {
        Sense::click_and_drag()
    } else {
        Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, CARD_H), sense);
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let visuals = ui.visuals();
    let bg = if selected {
        visuals.selection.bg_fill.gamma_multiply(0.35)
    } else if response.hovered() {
        visuals.widgets.hovered.weak_bg_fill
    } else {
        visuals.faint_bg_color
    };
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    painter.rect_filled(rect, 4.0, bg);
    if selected {
        painter.rect_stroke(rect, 4.0, visuals.selection.stroke, StrokeKind::Inside);
    }

    let pad = 8.0;
    let left = rect.left() + pad;
    // id — monospace, prominent.
    painter.text(
        pos2(left, rect.top() + 5.0),
        Align2::LEFT_TOP,
        &card.id,
        FontId::monospace(13.0),
        visuals.strong_text_color(),
    );
    // order — right-aligned.
    if !card.order_label.is_empty() {
        painter.text(
            pos2(rect.right() - pad, rect.top() + 6.0),
            Align2::RIGHT_TOP,
            &card.order_label,
            FontId::monospace(11.0),
            visuals.weak_text_color(),
        );
    }
    // title.
    painter.text(
        pos2(left, rect.top() + 21.0),
        Align2::LEFT_TOP,
        &card.title,
        FontId::proportional(12.0),
        visuals.text_color(),
    );
    // scope breadcrumb (work tickets; ) — compact chip path, per-level
    // muted accents, painter-clipped at the card edge. The card form omits the
    // "(no surface)" marker (detail-panel only). An owns-inferred scope is
    // PREFIXED with the ~ glyph (always visible even when the tail clips) and
    // gets a hover tooltip on the glyph.
    if let Some(bc) = &card.breadcrumb {
        let font = FontId::proportional(9.0);
        let y = rect.top() + 36.0;
        let mut x = left;
        if bc.estimated {
            let r = painter.text(
                pos2(x, y),
                Align2::LEFT_TOP,
                board::SCOPE_ESTIMATED_GLYPH,
                font.clone(),
                SCOPE_ESTIMATED_COLOR,
            );
            x = r.right() + 2.0;
            ui.interact(
                r.expand(2.0),
                response.id.with("scope_estimated"),
                Sense::hover(),
            )
            .on_hover_text(board::SCOPE_ESTIMATED_TIP);
        }
        for (i, seg) in bc.segs.iter().enumerate() {
            if i > 0 {
                let r = painter.text(
                    pos2(x, y),
                    Align2::LEFT_TOP,
                    board::SCOPE_SEP,
                    font.clone(),
                    visuals.weak_text_color(),
                );
                x = r.right() + 3.0;
            }
            let r = painter.text(
                pos2(x, y),
                Align2::LEFT_TOP,
                &seg.text,
                font.clone(),
                scope_level_color(seg.level),
            );
            x = r.right() + 3.0;
        }
    }
    // executor chip.
    let galley = painter.layout_no_wrap(
        card.executor.clone(),
        FontId::proportional(10.0),
        visuals.weak_text_color(),
    );
    let chip_pos = pos2(left, rect.bottom() - 5.0 - galley.size().y);
    let chip_rect = Rect::from_min_size(chip_pos, galley.size()).expand2(vec2(4.0, 1.5));
    painter.rect_filled(chip_rect, 6.0, visuals.extreme_bg_color);
    painter.galley(chip_pos, galley, visuals.weak_text_color());
    // class chip — accent-colored text on the same chip ground; a
    // ticket without a class (programs, pre-triage work) renders none.
    if let Some(class) = card.class {
        let color = class_color(class);
        let galley =
            painter.layout_no_wrap(class.as_str().to_owned(), FontId::proportional(10.0), color);
        let class_pos = pos2(
            chip_rect.right() + 6.0,
            rect.bottom() - 5.0 - galley.size().y,
        );
        let class_rect = Rect::from_min_size(class_pos, galley.size()).expand2(vec2(4.0, 1.5));
        painter.rect_filled(class_rect, 6.0, visuals.extreme_bg_color);
        painter.galley(class_pos, galley, color);
    }
    // hovering a card surfaces its main_goal (precomputed at load —
    // `board::card_tooltip`); an absent goal attaches nothing, never an empty
    // tooltip bubble.
    match &card.tooltip {
        Some(goal) => response.on_hover_text(goal.as_str()),
        None => response,
    }
}
