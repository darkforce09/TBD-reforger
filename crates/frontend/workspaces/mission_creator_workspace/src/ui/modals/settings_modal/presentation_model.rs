//! Mission presentation for the mission settings interface.

#[cfg(any(test, target_arch = "wasm32"))]
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// A mission row presentation column.
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) enum PresentationField {
    Briefing,
    Thumbnail,
}

#[cfg(any(test, target_arch = "wasm32"))]
impl PresentationField {
    /// Provides column for mission settings.
    #[cfg(any(test, target_arch = "wasm32"))]
    pub(super) fn column(self) -> &'static str {
        match self {
            Self::Briefing => "briefing",
            Self::Thumbnail => "thumbnail_url",
        }
    }

    /// Provides what for mission settings.
    #[cfg(any(test, target_arch = "wasm32"))]
    pub(super) fn what(self) -> &'static str {
        match self {
            Self::Briefing => "The briefing",
            Self::Thumbnail => "The thumbnail link",
        }
    }

    /// Provides read for mission settings.
    #[cfg(any(test, target_arch = "wasm32"))]
    pub(super) fn read(self, row: &RowShape) -> &str {
        match self {
            Self::Briefing => &row.briefing,
            Self::Thumbnail => &row.thumbnail_url,
        }
    }

    /// Provides write for mission settings.
    #[cfg(any(test, target_arch = "wasm32"))]
    pub(super) fn write(self, row: &mut RowShape, value: String) {
        match self {
            Self::Briefing => row.briefing = value,
            Self::Thumbnail => row.thumbnail_url = value,
        }
    }
}

#[must_use]
/// Checks links accepted by the thumbnail URL column.
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) fn is_acceptable_thumbnail_url(raw: &str) -> bool {
    let trimmed = raw.trim();
    trimmed.is_empty() || http_url_guard::is_http_url(trimmed)
}

/// Explains the library briefing field.
#[cfg(target_arch = "wasm32")]
pub(super) const BRIEFING_NOTE: &str = "The library blurb — what the mission browser, the dossier and the \
                             approval queue show before anyone joins, and what an exported mission \
                             carries. It is not the in-game briefing screen: that is written per \
                             faction, on the faction.";

/// Explains the thumbnail link field.
#[cfg(target_arch = "wasm32")]
pub(super) const THUMBNAIL_URL_NOTE: &str = "An absolute http:// or https:// link to an image — the picture the \
                                  mission library card shows. There is no upload here: the mission \
                                  row stores a link, so host the image and paste its address. Clear \
                                  the box to remove the picture.";

/// Explains a locally rejected thumbnail link.
#[cfg(target_arch = "wasm32")]
pub(super) const THUMBNAIL_REJECTED_NOTE: &str = "That is not an absolute http:// or https:// link, so it was \
                                       not saved. A site path like /uploads/x.png is not enough — \
                                       the mission row needs the whole address.";

/// Explains why presentation controls are unavailable.
#[cfg(target_arch = "wasm32")]
pub(super) const PRESENTATION_UNAVAILABLE_NOTE: &str = "The mission row has not loaded, so the briefing and thumbnail cannot be edited here. A draft \
     that has never been saved to the library has no row yet.";

/// Formats a failed presentation update.
#[cfg(target_arch = "wasm32")]
pub(super) fn presentation_failure_message(
    field: PresentationField,
    err: &frontend_transport::Error,
) -> String {
    let what = field.what();
    if err.status() == 403 {
        return format!(
            "{what} was not saved — you are not this mission's author. It has been put back to the \
             stored value."
        );
    }
    format!(
        "Could not save {}: {}. It has been put back to the stored value.",
        what.to_lowercase(),
        err.message_or("the server did not respond")
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
    let Some(handle) = mission_creator_engine_bridge::bridge::document_host::history::doc_handle()
    else {
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
