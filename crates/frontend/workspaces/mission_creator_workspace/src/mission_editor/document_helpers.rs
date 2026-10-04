//! The page's document helpers: the subject resolver type the routes share, and the Arrange
//! chord the keyboard dispatch runs.
#[cfg(target_arch = "wasm32")]
use super::*;

#[cfg(target_arch = "wasm32")]
/// Maps a document subject to its route and world position.
pub(crate) type SubjectResolver = std::rc::Rc<dyn Fn(&str) -> Option<(RouteTarget, f64, f64)>>;

#[cfg(target_arch = "wasm32")]
/// Runs an Arrange command when enough entities are selected.
pub(crate) fn arrange_chord(kind: top_strip::ArrangeKind) -> bool {
    if mission_editing_session::host::selection_len() < top_strip::ARRANGE_MIN_SELECTION {
        return false;
    }
    top_strip::run_arrange(kind);
    true
}
