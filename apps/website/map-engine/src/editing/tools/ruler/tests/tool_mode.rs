//! Role: which tool claims the left button, and which buttons capture a point.
//! Position: `editing/tools/ruler/tests` in the map engine.
//! Signals & state: explicit vertices and chains built in the test body.
//! Invariants: only the primary button captures; a secondary or middle press is the camera's, whatever the tool.

use super::*;

#[test]
fn should_begin_ruler_button_and_tool_gating() {
    // Ruler tool + LEFT button → a ruler press.
    assert!(should_begin_ruler(EditorTool::Ruler, 0));
    // Ruler tool + non-left buttons → NOT a ruler press (MMB pan / RMB menu untouched) — (c).
    assert!(
        !should_begin_ruler(EditorTool::Ruler, 1),
        "middle stays pan"
    );
    assert!(
        !should_begin_ruler(EditorTool::Ruler, 2),
        "right stays context menu"
    );
    // Select tool → never a ruler press, on any button (the Select machine is unchanged).
    assert!(!should_begin_ruler(EditorTool::Select, 0));
    assert!(!should_begin_ruler(EditorTool::Select, 1));
    assert_eq!(
        EditorTool::default(),
        EditorTool::Select,
        "Select is the default tool"
    );
    assert!(EditorTool::Ruler.is_ruler() && !EditorTool::Select.is_ruler());
    // T-643 — LoS is a point-capture tool too: it opens the SAME LG::Ruler arm on a left click,
    // so `should_begin_ruler` is true for LoS+button0 and false on non-left / under Select.
    assert!(
        should_begin_ruler(EditorTool::LoS, 0),
        "LoS left click captures a point"
    );
    assert!(
        !should_begin_ruler(EditorTool::LoS, 2),
        "LoS right stays context menu"
    );
    assert!(EditorTool::LoS.is_los() && !EditorTool::LoS.is_ruler());
    assert!(!EditorTool::Ruler.is_los() && !EditorTool::Select.is_los());
    // `captures_points` is exactly {Ruler, LoS}; Select never captures (its machine is unchanged).
    assert!(EditorTool::Ruler.captures_points() && EditorTool::LoS.captures_points());
    assert!(!EditorTool::Select.captures_points());
}
