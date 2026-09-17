use super::*;
use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};

/// The one place the tooltip text is decided. A disabled row's title is the blocking ticket if
/// it has one, else the item's [`ContextItem::why`] reason — so this mirrors `menu_row`.
fn title_source(entry: &MenuEntry) -> String {
    if entry.enabled {
        return String::new();
    }
    if let Some(t) = entry.blocked {
        return format!("Not available yet — {t}");
    }
    entry
        .item
        .and_then(ContextItem::why)
        .unwrap_or_default()
        .to_string()
}

#[test]
fn every_disabled_row_in_both_takes_has_a_nonempty_title() {
    for take in [MenuTake::EmptyGround, MenuTake::OnEntity] {
        for entry in take.entries() {
            // Separators (item == None) are not rows; skip them.
            if entry.item.is_none() || entry.enabled {
                continue;
            }
            let title = title_source(&entry);
            assert!(
                !title.is_empty(),
                "F-37: disabled row {:?} ({:?}) renders with no tooltip",
                entry.item,
                entry.label,
            );
        }
    }
}

/// The specific regression: `Play from Here` / `Play as the Character` had NO title anywhere in
/// their chain. Both are ticket-less, so their tooltip must come from `why()`.
#[test]
fn play_rows_now_have_a_reason() {
    for item in [ContextItem::PlayFromHere, ContextItem::PlayAsCharacter] {
        let why = item.why();
        assert!(
            why.is_some_and(|w| !w.is_empty()),
            "F-37: {item:?} must expose a why-tooltip (it had none before T-807)"
        );
    }
}

/// `menu_row` must actually READ `item.why()` — a model reason nothing renders is the hollow
/// half of this fix. Source-pinned (literals kept) so removing the wiring turns this red.
#[test]
fn render_row_wires_why_into_the_title() {
    let code = live_code(super::test_source::raw_context_menu());
    let body = only_body(&code, "fn render_row");
    assert!(
        body.contains("item.why()"),
        "F-37: render_row must fall back to item.why() for the disabled-row tooltip"
    );
}
