use super::*;
use std::collections::HashMap;

#[test]
fn plate_keeps_topmost_in_window_entry_and_marks_occupancy() {
    let mut y_down: ScanMap = HashMap::new();
    // Column (1, 1): roof at 8.0 (out of window), slab lip at 3.15 and 3.05 (both in).
    y_down.insert((1, 1), vec![8.0, 3.15, 3.05]);
    // Column (2, 1): only a roof entry — stays void.
    y_down.insert((2, 1), vec![8.0]);
    let p = Params::default();
    let (grid, heights) = floor_plate(&y_down, 4, 4, 3.1, &p);
    assert!(grid.get(1, 1));
    assert_eq!(heights[4 + 1], Some(3.15), "topmost in-window entry wins");
    assert!(!grid.get(2, 1));
    assert_eq!(heights[2 * 4 + 1], None);
    assert_eq!(grid.count(), 1);
}
