//! Role: group a multi-transaction edit into ONE undo step.
//! Position: `editing` in the map engine.
//! Signals & state: none of its own; it opens and closes a group on the hosted document.
//! Invariants: the group closes on the way out whatever `f` does — a panic or an early return
//! must not leave the document permanently grouping, because every later edit would then fold
//! into a step nobody can undo past.
use crate::data::store::MissionDocCore;
use crate::editing::host::{with_doc, with_doc_mut};

/// Run `f` inside one undo group labelled `label`.
///
/// The label is for call-site intent; the document's stack items carry no per-item metadata. The
/// group is closed by a drop guard, so an early return or an unwind closes it too.
pub fn with_batch<F, R>(label: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    let _ = label;
    begin_group();
    struct EndOnDrop;
    impl Drop for EndOnDrop {
        fn drop(&mut self) {
            end_group();
        }
    }
    let _guard = EndOnDrop;
    f()
}

fn begin_group() {
    with_doc(MissionDocCore::begin_group);
}

fn end_group() {
    with_doc_mut(MissionDocCore::end_group);
}
