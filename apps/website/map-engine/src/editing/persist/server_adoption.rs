//! Role: replace the authored document with a compiled payload, and stamp the mission row's own
//! fields onto it.
//! Position: `editing/persist` in the map engine.
//! Signals & state: none of its own — the document arrives as a handle and the host's post-edit
//! tail arrives as a closure.
//! Invariants: an adopt is a WHOLE-DOCUMENT replacement, so whether it is reachable by undo is
//! decided by the transaction origin it runs under and by nothing else ([`Adopt`]). A row whose
//! fields are all blank is never written, because writing blanks over a document that already
//! carries a title is the same loss as writing the wrong title. And the title the document ends up
//! with is the payload's own whenever it has one: the row is fetched separately and can be stale,
//! while the payload's title was authored into the very document being adopted.

use crate::editing::host::DocHandle;

/// The layer id a hydrate files a slot under when the slot's own layer is not in the payload.
///
/// Slots outlive layers — deleting a layer must not delete the work on it — so a hydrate needs one
/// destination that always exists. This is that destination, and it is the same id the placement
/// path mints its first layer under, so a re-hydrated document and a freshly authored one agree on
/// where an unlayered slot lives.
pub const DEFAULT_LAYER_ID: &str = "layer-1";

/// The mission-row fields that are not part of the authored payload: title, terrain, time of day,
/// weather, and the library blurb.
///
/// `briefing` here is the row's one-line blurb — the string a mission listing shows — and NOT the
/// per-faction briefing object the payload carries. The two are different fields with the same
/// English word on them, so the distinction is stated rather than left to the reader.
#[derive(Default)]
pub struct RowMeta {
    /// The mission's name on its row. Superseded by the payload's own title when it has one.
    pub title: String,

    /// The world the mission is authored on.
    pub terrain: String,

    /// The scenario clock, as the row spells it.
    pub time_of_day: String,

    /// The scenario weather, as the row spells it.
    pub weather: String,

    /// The library blurb.
    pub briefing: String,
}

impl RowMeta {
    /// Does this row carry nothing worth writing into a document?
    ///
    /// Title, terrain and blurb only: time of day and weather are enumerations with a meaningful
    /// default, so a row can legitimately carry them while carrying nothing else, and writing that
    /// row alone would replace a document's authored fields with the row's defaults.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.title.is_empty() && self.terrain.is_empty() && self.briefing.is_empty()
    }
}

/// Whether an adopt can be taken back with one undo.
///
/// The choice is purely the transaction origin the replacement runs under: the undo drive tracks
/// local edits and ignores initialization, so the same replacement is either one stack item or
/// none depending on this flag alone.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Adopt {
    /// A replacement over a document with nothing authored in it — a fresh seed, or a local record
    /// that decoded to an empty document. Initialization, so no undo step: a step here would make
    /// the operator's first undo resurrect the seed the adopt just removed.
    Init,

    /// A replacement over live local work. A local edit, so the replacement's single transaction
    /// becomes exactly one undo step and the work comes back on one keypress.
    ///
    /// **Partial by construction:** the undo drive scopes the roots the editor authors into, and a
    /// hydrate also clears roots outside that scope, which therefore still hold the adopted
    /// payload's rows after an undo. That partiality is why a whole-document snapshot is taken
    /// alongside this mode rather than instead of it.
    Undoable,
}

/// The payload's own top-level `title`, trimmed, when it is not blank.
///
/// Whitespace is not a title: a payload carrying `"   "` has no title to prefer, and treating it
/// as one would blank the document's name on every adopt.
#[must_use]
pub fn payload_title_nonblank(payload_json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(payload_json).ok()?;
    v.get("title")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// The title an adopt writes: the payload's own when it has one, else the row's.
///
/// The order matters because the two arrive from different places at different times. A hydrate
/// loads the payload's title into the document and a row write immediately follows it, so a row
/// that has not caught up would otherwise stomp the authored name with the stale one.
#[must_use]
pub fn prefer_payload_title(payload_json: &str, row_title: &str) -> String {
    payload_title_nonblank(payload_json).unwrap_or_else(|| row_title.trim().to_string())
}

/// A row field as an optional value: an empty string is an ABSENT field, not a blank one.
///
/// The document distinguishes the two — absent leaves whatever is there, blank overwrites it — so
/// the row's "this column is unset" has to arrive as absence or every unset column would erase its
/// counterpart in the document.
fn non_empty(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

/// Replace the document with `payload_json`, stamp `row` over the result, then run the host's
/// post-edit tail.
///
/// `mode` decides whether the replacement is one undo step (see [`Adopt`]). The tail is a closure
/// because what follows a local edit — rebinding what is drawn, arming the next write — is the
/// host's business; what belongs here is that it runs exactly once, after the transaction closes,
/// and not at all when there was no document to replace.
///
/// An `Undoable` adopt must stay exactly ONE step, and the hydrate and the row write are separate
/// transactions that each take their own stack item. A non-blank row under `Undoable` would
/// therefore need two undos to fully revert, so it is asserted against rather than assumed.
pub fn adopt_payload(
    doc: &DocHandle,
    payload_json: &str,
    row: &RowMeta,
    mode: Adopt,
    after_local_edit: &dyn Fn(),
) {
    debug_assert!(mode == Adopt::Init || row.is_empty());
    {
        let guard = doc.borrow();
        let Some(core) = guard.as_ref() else {
            return;
        };
        core.set_origin_init(mode == Adopt::Init);
        core.hydrate(payload_json, DEFAULT_LAYER_ID);
        if !row.is_empty() {
            let title = prefer_payload_title(payload_json, &row.title);
            core.apply_row_meta(
                &title,
                &row.terrain,
                non_empty(&row.time_of_day),
                non_empty(&row.weather),
                non_empty(&row.briefing),
            );
        }
        core.set_origin_init(false);
    }
    after_local_edit();
}

/// Stamp the mission row onto a document with no payload behind it — a mission that has never been
/// saved, whose only server-side truth is its row.
///
/// Initialization origin and no post-edit tail: nothing was replaced, so there is nothing to take
/// back and nothing new to draw. A blank row writes nothing at all.
pub fn apply_row_meta_only(doc: &DocHandle, row: &RowMeta) {
    if row.is_empty() {
        return;
    }
    let guard = doc.borrow();
    if let Some(core) = guard.as_ref() {
        core.set_origin_init(true);
        core.apply_row_meta(
            &row.title,
            &row.terrain,
            non_empty(&row.time_of_day),
            non_empty(&row.weather),
            non_empty(&row.briefing),
        );
        core.set_origin_init(false);
    }
}

#[cfg(test)]
#[path = "tests/server_adoption.rs"]
mod tests;
