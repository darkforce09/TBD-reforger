use super::{menu_axis_position, menu_scroll_top};

#[test]
fn measured_menu_fits_at_edges_after_expansion_and_resize() {
    // The V12 panel was 309.15625 x 1044 at (720, 450) in a 1440 x 900 viewport.
    // The DOM caps its extent before passing the measured box to this placement function.
    for (width, height) in [(1440.0_f64, 900.0_f64), (320.0, 240.0), (180.0, 120.0)] {
        for natural_height in [512.0_f64, 1044.0] {
            let panel_width = 309.15625_f64.min(width - 16.0);
            let panel_height = natural_height.min(height - 16.0);
            for (x, y) in [
                (0.0, 0.0),
                (width, 0.0),
                (0.0, height),
                (width, height),
                (width / 2.0, height / 2.0),
                (1390.0, 830.0), // retained open-menu anchor after a smaller resize
            ] {
                let left = menu_axis_position(x, panel_width, width);
                let top = menu_axis_position(y, panel_height, height);
                assert!(left >= 8.0 && left + panel_width <= width - 8.0);
                assert!(top >= 8.0 && top + panel_height <= height - 8.0);
            }
        }
    }
    assert_eq!(menu_axis_position(40.0, 300.0, 1440.0), 40.0);
}

#[test]
fn keyboard_scroll_exposes_rows_in_both_directions_and_after_resize() {
    assert_eq!(menu_scroll_top(0.0, 400.0, 191.0, 28.0), 0.0);
    assert_eq!(menu_scroll_top(0.0, 224.0, 723.0, 28.0), 527.0);
    assert_eq!(menu_scroll_top(527.0, 224.0, 219.0, 28.0), 219.0);
    assert_eq!(menu_scroll_top(219.0, 104.0, 303.0, 28.0), 227.0);
    // Walk all 19 children in both directions through a short panel. The same helper runs
    // against live DOM rects; the compiled-UI probe separately verifies CSS, clipping and hits.
    let mut scroll = 0.0;
    for row in (0..19).chain((0..19).rev()) {
        let top = 219.0 + f64::from(row) * 28.0;
        scroll = menu_scroll_top(scroll, 104.0, top, 28.0);
        assert!(top >= scroll && top + 28.0 <= scroll + 104.0);
    }
}
