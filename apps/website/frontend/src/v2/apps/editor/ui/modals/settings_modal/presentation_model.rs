//! Mission presentation for the mission settings interface.

use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// A mission row presentation column.
pub(super) enum PresentationField {
    Briefing,
    Thumbnail,
}

impl PresentationField {
    /// Provides column for mission settings.
    pub(super) fn column(self) -> &'static str {
        match self {
            Self::Briefing => "briefing",
            Self::Thumbnail => "thumbnail_url",
        }
    }

    /// Provides what for mission settings.
    pub(super) fn what(self) -> &'static str {
        match self {
            Self::Briefing => "The briefing",
            Self::Thumbnail => "The thumbnail link",
        }
    }

    /// Provides read for mission settings.
    pub(super) fn read(self, row: &RowShape) -> &str {
        match self {
            Self::Briefing => &row.briefing,
            Self::Thumbnail => &row.thumbnail_url,
        }
    }

    /// Provides write for mission settings.
    pub(super) fn write(self, row: &mut RowShape, value: String) {
        match self {
            Self::Briefing => row.briefing = value,
            Self::Thumbnail => row.thumbnail_url = value,
        }
    }
}

#[must_use]
/// Checks links accepted by the thumbnail URL column.
pub(super) fn is_acceptable_thumbnail_url(raw: &str) -> bool {
    let trimmed = raw.trim();
    trimmed.is_empty() || crate::v2::core::auth::url_guard::is_http_url(trimmed)
}

/// Explains the library briefing field.
pub(super) const BRIEFING_NOTE: &str =
    "The library blurb — what the mission browser, the dossier and the \
                             approval queue show before anyone joins, and what an exported mission \
                             carries. It is not the in-game briefing screen: that is written per \
                             faction, on the faction.";

/// Explains the thumbnail link field.
pub(super) const THUMBNAIL_URL_NOTE: &str = "An absolute http:// or https:// link to an image — the picture the \
                                  mission library card shows. There is no upload here: the mission \
                                  row stores a link, so host the image and paste its address. Clear \
                                  the box to remove the picture.";

/// Explains a locally rejected thumbnail link.
pub(super) const THUMBNAIL_REJECTED_NOTE: &str =
    "That is not an absolute http:// or https:// link, so it was \
                                       not saved. A site path like /uploads/x.png is not enough — \
                                       the mission row needs the whole address.";

/// Explains why presentation controls are unavailable.
pub(super) const PRESENTATION_UNAVAILABLE_NOTE: &str =
    "The mission row has not loaded, so the briefing and thumbnail cannot be edited here. A draft \
     that has never been saved to the library has no row yet.";

/// Formats a failed presentation update.
pub(super) fn presentation_failure_message(
    field: PresentationField,
    err: &crate::v2::core::api::client::ApiErr,
) -> String {
    let what = field.what();
    if err.0 == 403 {
        return format!(
            "{what} was not saved — you are not this mission's author. It has been put back to the \
             stored value."
        );
    }
    format!(
        "Could not save {}: {}. It has been put back to the stored value.",
        what.to_lowercase(),
        crate::v2::core::api::client::api_error_message(err, "the server did not respond")
    )
}

#[cfg(target_arch = "wasm32")]
/// Commits the active control before the dialog closes.
pub(super) fn blur_focused_control() {
    use wasm_bindgen::JsCast;
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element())
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        el.blur().ok();
    }
}

#[cfg(target_arch = "wasm32")]
/// Copies a saved row briefing into document metadata.
pub(super) fn mirror_briefing_into_document(briefing: &str) {
    let Some(handle) = crate::v2::apps::editor::bridge::document_host::history::doc_handle() else {
        return;
    };
    let doc = handle.borrow();
    let Some(doc) = doc.as_ref() else {
        return;
    };
    if briefing.trim().is_empty() {
        doc.clear_meta_briefing();
    } else {
        doc.apply_row_meta("", "", None, None, Some(briefing.to_string()));
    }
}
