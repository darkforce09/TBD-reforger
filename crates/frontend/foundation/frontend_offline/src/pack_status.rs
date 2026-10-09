//! The offline pack's status words: where the pack stands, its progress, whether its optional
//! files are cached and whether the last download refreshed the saved copy.
//!
//! **Role:** the value types the page-wide signals of [`crate::status_signals`] carry, and the
//! four document-element attributes each one is mirrored onto, with the word each value writes.
//! **Position:** pure; the pack download ([`crate::offline_pack`]) builds these values, the
//! signals publish them, and the mortar calculator and the offline browser gate read them back —
//! the page through the signals, the gate through the attributes.
//! **Signals & state:** none; plain values.
//! **Invariants:** the state is `ready` only when the pack lists every essential file and every
//! essential file is cached, and an incomplete pack or an essential file missing from the cache is
//! never `ready`; a refresh the server could not answer keeps a complete pack `ready`
//! ([`PackRefresh::KeptSavedCopy`]) while every essential file is still cached; the optional files
//! (the cross-origin icon font) never decide the state, only [`OptionalFiles`]; each attribute
//! word is fixed, since the offline browser gate reads it.

/// The document-element attribute that carries [`OfflineState::attribute_value`].
pub const OFFLINE_STATE_ATTRIBUTE: &str = "data-offline-state";

/// The document-element attribute that carries [`OfflineStatus::progress_percent`], 0 to 100.
pub const OFFLINE_PROGRESS_ATTRIBUTE: &str = "data-offline-progress";

/// The document-element attribute that carries [`OptionalFiles::attribute_value`]; absent while
/// no download has finished.
pub const OFFLINE_OPTIONAL_ATTRIBUTE: &str = "data-offline-optional";

/// The document-element attribute that carries [`PackRefresh::attribute_value`]; absent while
/// no download has finished.
pub const OFFLINE_REFRESH_ATTRIBUTE: &str = "data-offline-refresh";

/// Where the offline pack stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfflineState {
    /// Nothing has been attempted in this page lifetime.
    Idle,
    /// The pack is downloading; the progress is meaningful.
    Downloading,
    /// Every essential file of a complete pack is cached; [`OptionalFiles`] says whether the
    /// optional ones are too.
    Ready,
    /// Every listed file is cached, but the server lists fewer files than the pack needs (no or
    /// a partial map tile index).
    Incomplete,
    /// The origin's free storage is smaller than the files still to download.
    QuotaShort,
    /// The browser offers no service worker or no cache storage (or the page is not a secure
    /// context).
    Unsupported,
    /// An essential file or the pack list could not be fetched or stored.
    Failed,
}

impl OfflineState {
    /// The value of [`OFFLINE_STATE_ATTRIBUTE`].
    pub fn attribute_value(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Downloading => "downloading",
            Self::Ready => "ready",
            Self::Incomplete => "incomplete",
            Self::QuotaShort => "quota-short",
            Self::Unsupported => "unsupported",
            Self::Failed => "failed",
        }
    }
}

/// The offline pack's state with its download progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfflineStatus {
    /// Where the pack stands.
    pub state: OfflineState,
    /// Share of the pack cached, 0 to 100; 100 only once every listed file is cached.
    pub progress_percent: u8,
}

impl OfflineStatus {
    /// The status before anything is attempted.
    pub const IDLE: Self = Self {
        state: OfflineState::Idle,
        progress_percent: 0,
    };
}

/// Whether the optional files of the offline pack — the cross-origin Material Symbols icon font's
/// stylesheet and font files — are cached. Without them the calculator works offline, but its
/// icons may show as their ligature text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionalFiles {
    /// No download has finished in this page lifetime.
    Unknown,
    /// Every optional file is cached.
    Complete,
    /// An optional file could not be listed, fetched or stored.
    Missing,
}

impl OptionalFiles {
    /// The value of [`OFFLINE_OPTIONAL_ATTRIBUTE`]; `None` removes the attribute.
    pub fn attribute_value(self) -> Option<&'static str> {
        match self {
            Self::Unknown => None,
            Self::Complete => Some("complete"),
            Self::Missing => Some("missing"),
        }
    }
}

/// Whether the last finished download refreshed the saved copy from the server or kept it
/// because the server could not answer (unreachable, or a gateway or server failure).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackRefresh {
    /// No download has finished in this page lifetime.
    Unknown,
    /// The server answered every file the download read.
    Refreshed,
    /// The server could not answer and the saved copy stays in use; `saved_on` is when the server
    /// dated the saved catalog list (see
    /// [`offline_cache_policy::network_fallback::saved_on_from_date_header`]), `None` when it
    /// carried no readable date.
    KeptSavedCopy {
        /// The saved catalog list's date, worded for a person.
        saved_on: Option<String>,
    },
}

impl PackRefresh {
    /// The value of [`OFFLINE_REFRESH_ATTRIBUTE`]; `None` removes the attribute.
    pub fn attribute_value(&self) -> Option<&'static str> {
        match self {
            Self::Unknown => None,
            Self::Refreshed => Some("refreshed"),
            Self::KeptSavedCopy { .. } => Some("kept-saved-copy"),
        }
    }
}
