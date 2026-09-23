use std::path::PathBuf;
pub(crate) enum DocumentEvent {
    CloseViewer,
    OpenPath(PathBuf),
}
