#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DragSet {
    pub anchor: String,
    pub ids: Vec<String>,
}

/// Pure drop planning function that rejects dropping a parent/container into any of its dragged children.
/// Returns the list of ids to drop, or None if the drop is invalid.
pub fn plan_drop(
    drag: &DragSet,
    dest: &str,
    folder_descendants: impl Fn(&str) -> Vec<String>,
) -> Option<Vec<String>> {
    for id in &drag.ids {
        if id == dest {
            return None;
        }
        let descendants = folder_descendants(id);
        if descendants.contains(&dest.to_string()) {
            return None;
        }
    }
    Some(drag.ids.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_drop() {
        let drag = DragSet {
            anchor: "a".to_string(),
            ids: vec!["a".to_string(), "b".to_string()],
        };
        let descendants = |id: &str| -> Vec<String> {
            if id == "a" {
                vec!["c".to_string()]
            } else {
                vec![]
            }
        };
        // Drop into valid
        assert_eq!(
            plan_drop(&drag, "valid", descendants),
            Some(vec!["a".to_string(), "b".to_string()])
        );

        // Drop into self
        assert_eq!(plan_drop(&drag, "a", descendants), None);

        // Drop into child
        assert_eq!(plan_drop(&drag, "c", descendants), None);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerDrag {
    Folder(DragSet),
    Slot(DragSet),
    Comment(DragSet),
}

thread_local! {
    pub static PENDING_DRAG: std::cell::RefCell<Option<LayerDrag>> = const { std::cell::RefCell::new(None) };
}

pub fn begin_layer_drag(drag_set: DragSet) {
    PENDING_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Folder(drag_set)));
}

pub fn begin_layer_slot_drag(drag_set: DragSet) {
    PENDING_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Slot(drag_set)));
}

pub fn begin_layer_comment_drag(drag_set: DragSet) {
    PENDING_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Comment(drag_set)));
}

pub fn begin_refile(drag_set: DragSet) {
    // We can also store pending refile here
    PENDING_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Slot(drag_set)));
}

pub fn cancel_layer_drag() {
    PENDING_DRAG.with(|p| *p.borrow_mut() = None);
}
