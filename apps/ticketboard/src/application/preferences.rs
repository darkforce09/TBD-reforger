/// eframe Storage key for the picked repo root (user config dir — never the repo).
pub(super) const REPO_ROOT_KEY: &str = "repo_root";

/// eframe Storage key for the viewer-column width — same user-config
/// store as the repo root; the app still writes nothing under the repo.
pub(super) const VIEWER_W_KEY: &str = "viewer_w";

/// Default document-column width. The column is resizable within the saved-width
/// bounds and persists through eframe Storage independently of ticket selection.
pub(super) const VIEWER_W: f32 = 560.0;

/// Viewer-column drag + persistence bounds: a corrupt or hand-edited
/// preference clamps here on load ([`parse_viewer_width`]) — never an
/// invisible or board-swallowing column.
pub(super) const VIEWER_W_MIN: f32 = 280.0;

pub(super) const VIEWER_W_MAX: f32 = 1600.0;

/// The persisted viewer width, revalidated on load: parseable + finite +
/// clamped to the drag bounds; anything else falls back to the default —
/// symmetric with the repo-root preference, which is also revalidated rather
/// than trusted.
pub(super) fn parse_viewer_width(saved: Option<String>) -> f32 {
    saved
        .and_then(|s| s.trim().parse::<f32>().ok())
        .filter(|w| w.is_finite())
        .map_or(VIEWER_W, |w| w.clamp(VIEWER_W_MIN, VIEWER_W_MAX))
}
