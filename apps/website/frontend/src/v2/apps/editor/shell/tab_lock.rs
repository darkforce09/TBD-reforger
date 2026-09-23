//! Coordinates the writer role for tabs editing the same mission.
//! A browser lock selects the writer, presence messages inform peer tabs, and write stamps
//! identify the record that each tab last saved.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// The BroadcastChannel name for one mission's tabs. Per mission id, not per account: two accounts
/// on one machine would still be two tabs writing one physical key's neighbourhood, and the record
/// scoping () is what keeps their *bytes* apart  this channel only has to keep their
/// *presence* visible.
pub const CHANNEL_PREFIX: &str = "tbd-mission-tabs:";

/// The Web Lock whose holder is the writer for one mission.
pub const WRITER_LOCK_PREFIX: &str = "tbd-mission-writer:";

/// `localStorage` prefix for the write stamp  see [`Stamp`].
pub const STAMP_PREFIX: &str = "tbd-mission-draft:";

/// What this tab is allowed to do with the shared IndexedDB record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabRole {
    /// This tab holds the writer role: its debounced saves land.
    #[default]
    Writer,
    /// Another tab holds it. This tab keeps editing in memory, shows [`TabLockBanner`], and does
    /// not write  its edits reach the record when it inherits the role and its first save merges.
    ReadOnly,
}

/// One tab's announcement of itself: a per-page random id and the instant it opened the mission.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Presence {
    pub tab: String,
    pub since: f64,
}

/// A channel message. Deliberately one flat struct rather than a tagged enum, for the reason
/// `client.rs` gives about its own peer messages: a message from a build with different fields is
/// then ignored rather than half-read. `kind` is `hello` / `here` / `bye` / `saved`; `since` is the
/// open instant for the first three and the save instant for `saved`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Msg {
    pub kind: String,
    pub tab: String,
    #[serde(default)]
    pub since: f64,
}

/// Who wrote the record that is on disk right now, and when.
///
/// A `localStorage` sidecar, not a field inside the record, and that is a decision: the record's
/// value is a `Uint8Array` and `persist::read_raw` treats anything else as *unreadable*, so
/// wrapping the blob to carry a timestamp would make every pre- record unreadable to this
/// build and every post- record unreadable to any older one. A sidecar key changes no format.
///
/// It answers two questions with one fact. **"Must I merge before I write?"** no, if the last
/// writer was me: a CRDT document only ever grows, so a record I wrote is a subset of my own
/// current state and re-reading it costs an O(document) decode to learn nothing. Any other answer
/// (another tab, or no stamp at all) means the record may hold blocks I have never seen. And
/// **"when was the local copy last written?"** the conflict modal's local timestamp, which F-32
/// says the author was choosing without.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stamp {
    pub tab: String,
    pub at: f64,
}

/// What [`super::persist::run_save`](super::persist) does with a pending write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveDecision {
    /// Read the record and apply it into the live document first, then encode and write the union.
    Merge,
    /// The record on disk is this tab's own last write, so it holds nothing this document lacks.
    WriteThrough,
    /// Another tab holds the writer role. Re-arm rather than write, and rather than drop: the
    /// document is still in memory and inherits the role when that tab closes.
    Defer,
}

/// The policy `run_save` obeys, as a pure function so it can be tested on the host.
///
/// The `None` stamp deliberately means **merge**, not write-through. A record nobody can attribute
/// is exactly the record that may hold someone else's only copy  the same posture  takes for
/// an unowned orphan and  takes for an unreadable one. Reading it costs one decode; assuming
/// it is ours costs a document.
#[must_use]
pub fn decide_save(role: TabRole, stamp: Option<&Stamp>, me: &str) -> SaveDecision {
    if role == TabRole::ReadOnly {
        return SaveDecision::Defer;
    }
    match stamp {
        Some(s) if s.tab == me => SaveDecision::WriteThrough,
        _ => SaveDecision::Merge,
    }
}

/// The BroadcastChannel-only fallback election: the oldest tab writes, ties broken by id so every
/// tab computes the same answer from the same set. Used only where `navigator.locks` is absent —
/// see the module header.
#[must_use]
pub fn elect(me: &Presence, peers: &[Presence]) -> TabRole {
    let key = |p: &Presence| (p.since.to_bits(), p.tab.clone());
    let mine = key(me);
    if peers
        .iter()
        .filter(|p| p.tab != me.tab)
        .any(|p| key(p) < mine)
    {
        TabRole::ReadOnly
    } else {
        TabRole::Writer
    }
}

/// The banner copy. It says what is true and no more: this tab still edits, it just is not the one
/// writing the shared draft, and its work is not stranded by that.
///
/// It also names the *cause*, which is the third thing F-32 asked for — "it misattributes the
/// cause: it frames a two-tab divergence as a local-vs-server drift, so the author never learns
/// that a second tab is what ate their work, and will do it again."
#[must_use]
pub fn banner_copy(peers: usize) -> String {
    let others = if peers <= 1 {
        "Another tab".to_string()
    } else {
        format!("{peers} other tabs")
    };
    format!(
        "{others} already has this mission open, so this tab is not writing the local draft. \
         Keep editing — your changes stay in this tab and are merged in when the other tab closes. \
         Close the other tab to take over."
    )
}

/// `now - then` as a short human recency. Pure, so the conflict modal's "when" is testable.
#[must_use]
pub fn ago(now_ms: f64, then_ms: f64) -> String {
    let secs = ((now_ms - then_ms) / 1000.0).round();
    if !secs.is_finite() || secs < 0.0 {
        return "just now".to_string();
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let s = secs as u64;
    match s {
        0..=9 => "just now".to_string(),
        10..=59 => format!("{s}s ago"),
        60..=3599 => format!("{}m ago", s / 60),
        3600..=86_399 => format!("{}h ago", s / 3600),
        _ => format!("{}d ago", s / 86_400),
    }
}

/// An RFC 3339 instant from the API as `YYYY-MM-DD HH:MM UTC`. Anything that is not shaped like one
/// comes back untouched  a timestamp the modal cannot parse is still better shown than dropped.
#[must_use]
pub fn short_utc(iso: &str) -> String {
    let (date, rest) = match iso.split_once('T') {
        Some(p) => p,
        None => return iso.to_string(),
    };
    if date.len() != 10 || rest.len() < 5 {
        return iso.to_string();
    }
    format!("{date} {} UTC", &rest[..5])
}

/// The `localStorage` key carrying the [`Stamp`] for one **physical** IndexedDB key.
#[must_use]
pub fn stamp_key(physical_key: &str) -> String {
    format!("{STAMP_PREFIX}{physical_key}")
}

/* ─────────────────────────── live state (both targets) ─────────────────────────── */

thread_local! {
    /// This page's id. Minted once, never persisted: it identifies a *tab*, and a tab does not
    /// outlive its page.
    static TAB_ID: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    /// The role, as a plain cell. Source of truth, because `persist.rs` reads it from detached
    /// timers and async tasks that hold no reactive owner  the `save_status` / `LAST_FLUSH_MS`
    /// idiom, for the same reason.
    static ROLE: std::cell::Cell<TabRole> = const { std::cell::Cell::new(TabRole::Writer) };
    /// The banner's reactive mirror, handed over by [`TabLockBanner`] which has an owner.
    static ROLE_SIG: std::cell::RefCell<Option<RwSignal<TabRole>>> =
        const { std::cell::RefCell::new(None) };
    /// Peers that have announced themselves and not said `bye`.
    static PEERS: std::cell::RefCell<Vec<Presence>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// The reactive peer count, so the banner re-renders when a tab joins or leaves.
    static PEERS_SIG: std::cell::RefCell<Option<RwSignal<usize>>> =
        const { std::cell::RefCell::new(None) };
    /// What to do when a peer announces it just wrote the record. Installed by
    /// `persist::register_tab_sync`, so this module never has to reach into the writer  and the
    /// dependency stays one-way.
    static ON_PEER_SAVED: std::cell::RefCell<Option<Box<dyn Fn()>>> =
        const { std::cell::RefCell::new(None) };
    /// Set once `navigator.locks` has granted (or refused) the writer lock, so a fallback election
    /// can never demote a tab the browser has actually made the writer.
    static LOCK_DECIDED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// This tab's id.
#[must_use]
pub fn tab_id() -> String {
    TAB_ID.with(|t| t.borrow_mut().get_or_insert_with(mint_tab_id).clone())
}

/// This tab's role right now. Cheap and owner-free  `run_save` calls it from a timer.
#[must_use]
pub fn role() -> TabRole {
    ROLE.with(std::cell::Cell::get)
}

/// May this tab write the shared record? Only the writer may, and never while the tab shows a
/// review workspace, which writes nothing whatever the election says.
#[must_use]
pub fn may_write() -> bool {
    role() == TabRole::Writer && super::review_mode::writes_mission()
}

/// How many other tabs have this mission open.
#[must_use]
pub fn peer_count() -> usize {
    PEERS.with(|p| p.borrow().len())
}

/// Install the "a peer just wrote the record" reaction. One installer
/// (`persist::register_tab_sync`); overwriting is how a remount hands over a fresh closure.
pub fn set_peer_saved_handler(f: Box<dyn Fn()>) {
    ON_PEER_SAVED.with(|h| *h.borrow_mut() = Some(f));
}

fn set_role(next: TabRole) {
    ROLE.set(next);
    ROLE_SIG.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(next);
        }
    });
}

fn publish_peer_count() {
    let n = peer_count();
    PEERS_SIG.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(n);
        }
    });
}

/// The read-only banner. Renders no DOM while this tab is the writer  the same
/// "`None` renders nothing" discipline the conflict dialog uses, so it is V-capture-safe. A review
/// workspace carries its own banner, so this one stays silent there.
///
/// It creates both signals and parks them for the cells to push to, **seeded from those cells**,
/// because this component has a reactive owner and the channel callbacks that drive them do not
/// (`persist`'s `set_last_flush_signal` hand-over idiom). The seeding is what stops a role decided
/// before the banner mounted from being missed. Idempotent-by-overwrite: a remount replaces them.
#[component]
pub fn TabLockBanner() -> impl IntoView {
    let role_sig = RwSignal::new(role());
    let peers_sig = RwSignal::new(peer_count());
    ROLE_SIG.with(|s| *s.borrow_mut() = Some(role_sig));
    PEERS_SIG.with(|s| *s.borrow_mut() = Some(peers_sig));
    move || {
        (role_sig.get() == TabRole::ReadOnly && super::review_mode::writes_mission()).then(|| {
            view! {
                <div
                    role="status"
                    aria-live="polite"
                    class="pointer-events-auto mx-auto flex max-w-2xl items-start gap-2 rounded-lg border border-error/40 bg-error/10 px-4 py-2 text-label-md text-error shadow-lg"
                >
                    <span class="font-medium">"Read-only tab"</span>
                    <span class="text-on-surface-variant">{move || banner_copy(peers_sig.get())}</span>
                </div>
            }
        })
    }
}

/* ─────────────────────────── wasm: the channel and the lock ─────────────────────────── */

#[cfg(target_arch = "wasm32")]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn mint_tab_id() -> String {
    // 64 random bits, formatted hex. Not a credential and never persisted  it distinguishes tabs
    // of one browser, so `Math::random` is the right size of tool.
    let a = (js_sys::Math::random() * f64::from(u32::MAX)) as u32;
    let b = (js_sys::Math::random() * f64::from(u32::MAX)) as u32;
    format!("{a:08x}{b:08x}")
}

#[cfg(not(target_arch = "wasm32"))]
fn mint_tab_id() -> String {
    "native-test-tab".to_string()
}

#[cfg(target_arch = "wasm32")]
mod live;

#[cfg(target_arch = "wasm32")]
pub use live::{announce_saved, join, leave, read_stamp, register_bridge, write_stamp};

/// Native stand-ins, so `persist.rs`'s call sites need no second `cfg` and the host build of this
/// crate still links. The IO half of this module is a browser API by construction.
#[cfg(not(target_arch = "wasm32"))]
mod live {
    use super::Stamp;

    /// No channel to join off the browser, so joining a mission does nothing.
    pub fn join(_mission_id: &str) {}
    /// No channel to leave off the browser, so departing does nothing.
    pub fn leave() {}
    /// No peers to tell off the browser, so announcing a fresh record does nothing.
    pub fn announce_saved(_at: f64) {}
    /// No `window` to hang the probe on off the browser, so registering it does nothing.
    pub fn register_bridge() {}
    /// Always `None`: there is no local storage to have stamped, and the save policy reads a
    /// missing stamp as "merge", which is the safe verdict.
    #[must_use]
    pub fn read_stamp(_physical_key: &str) -> Option<Stamp> {
        None
    }
    /// No local storage to stamp off the browser, so recording a write does nothing.
    pub fn write_stamp(_physical_key: &str, _at: f64) {}
}

// `pub` does not make these reachable from outside  this crate is a `bin`, so an unused re-export
// is still an unused import. Their one caller (`persist.rs`) is wasm32-gated in `shell/mod.rs`, so
// on the host nothing calls them and nothing should: they exist to keep that file's call sites free
// of a second `cfg` each.
#[cfg(not(target_arch = "wasm32"))]
#[allow(unused_imports)]
pub use live::{announce_saved, join, leave, read_stamp, register_bridge, write_stamp};

#[cfg(test)]
#[path = "tests/tab_lock/writer_election_and_conflict.rs"]
mod tests;
