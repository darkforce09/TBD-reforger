//! The offline pack download: on the first mortar calculator visit of a page lifetime, every
//! file of the [`super::offline_manifest`] lands in the cache the offline worker reads it from.
//!
//! **Role:** decides when the download runs ([`is_offline_pack_route`], once per page), counts
//! its progress ([`PackProgress`]), its failures ([`FailedFiles`]) and its outcome
//! ([`final_outcome`]); the browser download that applies them lives in `pack_download`.
//! **Position:** [`OfflinePackRouteWatcher`], mounted inside the router by `main.rs`, starts
//! [`ensure_offline_pack`] on the route, which runs the browser download and publishes its end
//! through [`super::publish_optional_files`], [`super::publish_pack_refresh`] and
//! [`super::publish_status`].
//! **Signals & state:** a thread-local once-flag (one download per page lifetime); the status,
//! optional-file and refresh signals of [`super`].
//! **Invariants:** files already cached are not fetched again; the quota check runs before the
//! first fetch; an essential file (or listing) missing from the cache after its fetch failed ends
//! in `failed`; an essential file whose refresh the server could not answer (unreachable or a
//! `5xx`) while its saved copy is still cached is kept ([`FailedFiles::kept_saved_copies`]) and
//! never downgrades the pack; an incomplete pack list ends in `incomplete`, and only a complete
//! list with every essential file cached in `ready`; an optional file
//! ([`OfflineTarget::is_optional`], the cross-origin icon
//! font) never changes the state, it only makes [`OptionalFiles::Missing`]; progress never reads
//! 100 before the last file is stored, so a `ready` pack missing an optional file stays below 100.

use super::offline_manifest::OfflineTarget;
use super::{OfflineState, OptionalFiles};

/// The route whose first visit downloads the offline pack.
pub const OFFLINE_PACK_ROUTE: &str = "/tools/mortar";

/// How many files download at once.
pub const PARALLEL_DOWNLOADS: usize = 6;

/// Whether `path` is [`OFFLINE_PACK_ROUTE`] or below it.
pub fn is_offline_pack_route(path: &str) -> bool {
    path.strip_prefix(OFFLINE_PACK_ROUTE)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// The download's progress over the files still to fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PackProgress {
    /// Files to fetch.
    pub total_files: usize,
    /// Files fetched and stored.
    pub stored_files: usize,
    /// Sum of the declared sizes of the files to fetch.
    pub declared_bytes: u64,
    /// Declared bytes of the stored files.
    pub stored_bytes: u64,
}

impl PackProgress {
    /// Progress over `total_files` files declaring `declared_bytes` in total.
    pub fn new(total_files: usize, declared_bytes: u64) -> Self {
        Self {
            total_files,
            declared_bytes,
            ..Self::default()
        }
    }

    /// Records one stored file that declared `expected_bytes`.
    pub fn record_stored(&mut self, expected_bytes: Option<u64>) {
        self.stored_files = (self.stored_files + 1).min(self.total_files);
        self.stored_bytes =
            (self.stored_bytes + expected_bytes.unwrap_or(0)).min(self.declared_bytes);
    }

    /// Share stored, 0 to 100: by declared bytes when any are declared, by files otherwise; 100
    /// only when every file is stored.
    pub fn percent(&self) -> u8 {
        if self.stored_files >= self.total_files {
            return 100;
        }
        let share = if self.declared_bytes > 0 {
            self.stored_bytes as f64 / self.declared_bytes as f64
        } else {
            self.stored_files as f64 / self.total_files as f64
        };
        ((share * 100.0).floor() as u8).min(99)
    }
}

/// The files a download could not list, fetch or store, split by [`OfflineTarget::is_optional`],
/// and the essential files whose saved copy stays in use because the server could not answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FailedFiles {
    /// Essential files that failed and are not cached.
    pub essential: usize,
    /// Optional files that failed, or whose listing (the icon font stylesheet) failed.
    pub optional: usize,
    /// Essential files (or listings) whose refresh the server could not answer while their saved
    /// copy is still cached.
    pub kept_saved_copies: usize,
}

impl FailedFiles {
    /// Records one failed file.
    pub fn record(&mut self, target: &OfflineTarget) {
        if target.is_optional() {
            self.optional += 1;
        } else {
            self.essential += 1;
        }
    }

    /// Records one file whose fetch failed: an essential file whose server could not answer
    /// (`server_unavailable`: unreachable or a `5xx`) while `saved_copy_cached` is kept; a
    /// refusal (`4xx`), a missing saved copy or an optional file is a failure.
    pub fn record_refresh_failure(
        &mut self,
        target: &OfflineTarget,
        server_unavailable: bool,
        saved_copy_cached: bool,
    ) {
        if !target.is_optional() && server_unavailable && saved_copy_cached {
            self.record_kept_saved_copy();
        } else {
            self.record(target);
        }
    }

    /// Records one essential listing (the catalog list, the terrain manifest or the tile index)
    /// that was read from its saved copy because the server could not answer.
    pub fn record_kept_saved_copy(&mut self) {
        self.kept_saved_copies += 1;
    }

    /// Records optional files that could not be listed, because the icon font stylesheet that
    /// names them could not be read.
    pub fn record_unlisted_optional_files(&mut self) {
        self.optional += 1;
    }
}

/// How a finished download ends: its state and the coverage of its optional files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackOutcome {
    /// `failed` on any essential failure, else `ready` for a complete pack list, else
    /// `incomplete`.
    pub state: OfflineState,
    /// [`OptionalFiles::Missing`] on any optional failure, else [`OptionalFiles::Complete`].
    pub optional_files: OptionalFiles,
    /// Whether any essential file or listing kept its saved copy because the server could not
    /// answer; it never changes [`PackOutcome::state`].
    pub kept_saved_copy: bool,
}

/// The outcome of a finished download over a pack list that is complete or not.
pub fn final_outcome(pack_complete: bool, failed: FailedFiles) -> PackOutcome {
    let state = if failed.essential > 0 {
        OfflineState::Failed
    } else if pack_complete {
        OfflineState::Ready
    } else {
        OfflineState::Incomplete
    };
    let optional_files = if failed.optional > 0 {
        OptionalFiles::Missing
    } else {
        OptionalFiles::Complete
    };
    PackOutcome {
        state,
        optional_files,
        kept_saved_copy: failed.kept_saved_copies > 0,
    }
}

/// Starts the route-triggered download whenever the router lands on [`OFFLINE_PACK_ROUTE`].
#[leptos::component]
pub fn OfflinePackRouteWatcher() -> impl leptos::IntoView {
    use leptos::prelude::{Effect, Get};
    let location = leptos_router::hooks::use_location();
    Effect::new(move |_| {
        if is_offline_pack_route(&location.pathname.get()) {
            ensure_offline_pack();
        }
    });
}

thread_local! {
    static DOWNLOAD_STARTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Starts the offline pack download unless it already ran in this page lifetime.
pub fn ensure_offline_pack() {
    if DOWNLOAD_STARTED.with(|started| started.replace(true)) {
        return;
    }
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async {
        let finished = super::pack_download::download().await;
        super::publish_optional_files(finished.optional_files);
        super::publish_pack_refresh(finished.refresh);
        super::publish_status(finished.status);
    });
}

#[cfg(test)]
#[path = "tests/offline_pack.rs"]
mod tests;
