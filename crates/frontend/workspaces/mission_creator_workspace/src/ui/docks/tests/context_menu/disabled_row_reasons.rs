use super::*;

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
