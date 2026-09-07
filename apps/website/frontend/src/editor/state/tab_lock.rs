//! T-190 (F-32) — two tabs on one mission stop clobbering each other.
//!
//! # The defect
//!
//! Every tab of `/missions/:id/edit` writes the *same* per-account IndexedDB record
//! ([`super::persist::save_state_as`]), and until this module nothing told any of them that the
//! others existed. The UX review's verified repro: tab B loaded the same `OBJ9`, deleted
//! everything (`OBJ0`) while tab A still showed `OBJ9`, both debounces fired, the last one won, and
//! the next reload blamed **server** drift for a divergence two *local* tabs had caused.
//!
//! # The three things that fix it, and which one does what
//!
//! 1. **A writer election, so the second tab cannot silently overwrite the first.** Exactly one tab
//!    per mission holds the writer role; every other tab is [`TabRole::ReadOnly`], shows
//!    [`TabLockBanner`], and its saves [`SaveDecision::Defer`] instead of landing.
//! 2. **A read-merge-write in `persist.rs`, so nothing is lost when writes DO interleave.**
//!    [`decide_save`] is the policy that function obeys; the merge itself is
//!    `MissionDocCore::apply_update`, a CRDT union — never a JSON diff.
//! 3. **A conflict modal that names both options** (`hydrate.rs` + `canvas/overlays.rs`).
//!
//! # A Web Lock for the role, a BroadcastChannel for the announcements
//!
//! Both reach the platform through `js_sys::Reflect`, following `core::client.rs` (`:422-430`,
//! `:509-532`) verbatim: `web_sys::BroadcastChannel` is not in this crate's web-sys feature list
//! and `web_sys::LockManager` is behind `--cfg=web_sys_unstable_apis`, a workspace-wide RUSTFLAGS
//! change. Reflect costs no dependency and no build flag, and this slice owns no `Cargo.toml`.
//!
//! They are not interchangeable and both are load-bearing. The **lock decides the role**, because
//! the browser releases it when the page *goes away* — the crash case a presence ping cannot cover
//! (see `live::claim_writer_lock`). The **channel carries what the lock cannot**: the peer count
//! the banner names, and `saved`, which is how a read-only tab learns to pull the writer's record
//! instead of discovering the divergence at the next reload. Where Web Locks are unavailable — they
//! need a secure context, and staging over plain http on a bare IP has none — the role falls back
//! to [`elect`] over the presence messages. That fallback is strictly weaker, and it is why item 2
//! is not optional: **the role is an ergonomic guarantee; the merge is the data guarantee.**
//!
//! # Ungated, deliberately
//!
//! Registered in `state/mod.rs` without a `target_arch` gate, for the reason `save_status` gives at
//! `:29-32`: `cargo test -p website-frontend` runs on the **host**, and this crate links
//! `map-engine-core` *without* the `doc` feature off wasm32 (`Cargo.toml:26` vs `:115`) — so there
//! is no `MissionDocCore` and no `yrs` natively, and this repo has no wasm-bindgen-test harness
//! either. Everything decidable without a browser therefore lives here as a pure function with a
//! native test ([`elect`], [`decide_save`], [`Stamp`], [`ago`], [`short_utc`], [`banner_copy`]),
//! and the wasm-only write path is pinned through `class_r_scrub` — the same oracle
//! `save_status.rs` already uses over `persist.rs`.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// The BroadcastChannel name for one mission's tabs. Per mission id, not per account: two accounts
/// on one machine would still be two tabs writing one physical key's neighbourhood, and the record
/// scoping (T-221) is what keeps their *bytes* apart — this channel only has to keep their
/// *presence* visible.
pub const CHANNEL_PREFIX: &str = "tbd-mission-tabs:";

/// The Web Lock whose holder is the writer for one mission.
pub const WRITER_LOCK_PREFIX: &str = "tbd-mission-writer:";

/// `localStorage` prefix for the write stamp — see [`Stamp`].
pub const STAMP_PREFIX: &str = "tbd-mission-draft:";

/// What this tab is allowed to do with the shared IndexedDB record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabRole {
    /// This tab holds the writer role: its debounced saves land.
    #[default]
    Writer,
    /// Another tab holds it. This tab keeps editing in memory, shows [`TabLockBanner`], and does
    /// not write — its edits reach the record when it inherits the role and its first save merges.
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
/// Kept in `localStorage` beside the IndexedDB record rather than inside it, and that is a decision
/// rather than convenience: the record's value is a `Uint8Array` and
/// [`super::persist::read_raw`](super::persist) treats anything else as *unreadable*, so wrapping
/// the blob in an object to carry a timestamp would make every pre-T-190 record unreadable and
/// every post-T-190 record unreadable to any older build. A sidecar key changes no format.
///
/// It answers two questions with one fact:
///   * **"do I have to merge before I write?"** — no, if the last writer was me: a CRDT document
///     only ever grows, so a record I wrote is a subset of my own current state and re-reading it
///     would cost an O(document) decode to learn nothing. Any other answer (another tab, or no
///     stamp at all) means the record may hold blocks I have never seen.
///   * **"when was the local copy last written?"** — the conflict modal's local timestamp, which
///     F-32 says the author was choosing without.
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
/// is exactly the record that may hold someone else's only copy — the same posture T-221 takes for
/// an unowned orphan and T-374 takes for an unreadable one. Reading it costs one decode; assuming
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
/// comes back untouched — a timestamp the modal cannot parse is still better shown than dropped.
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
    /// timers and async tasks that hold no reactive owner — the `save_status` / `LAST_FLUSH_MS`
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
    /// `persist::register_tab_sync`, so this module never has to reach into the writer — and the
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

/// This tab's role right now. Cheap and owner-free — `run_save` calls it from a timer.
#[must_use]
pub fn role() -> TabRole {
    ROLE.with(std::cell::Cell::get)
}

/// May this tab write the shared record?
#[must_use]
pub fn may_write() -> bool {
    role() == TabRole::Writer
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

/// The read-only banner. Renders no DOM while this tab is the writer — the same
/// "`None` renders nothing" discipline the conflict dialog uses, so it is V-capture-safe.
///
/// It creates the two signals and hands them over ([`set_role_signal`] / [`set_peers_signal`]),
/// seeded from the cells, because this component has a reactive owner and the channel callbacks
/// that drive them do not.
#[component]
pub fn TabLockBanner() -> impl IntoView {
    let role_sig = RwSignal::new(role());
    let peers_sig = RwSignal::new(peer_count());
    set_role_signal(role_sig);
    set_peers_signal(peers_sig);
    move || {
        (role_sig.get() == TabRole::ReadOnly).then(|| {
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

/// Hand over the banner's role signal, seeded from the cell so a role decided before the banner
/// mounted is not missed.
pub fn set_role_signal(sig: RwSignal<TabRole>) {
    sig.set(role());
    ROLE_SIG.with(|s| *s.borrow_mut() = Some(sig));
}

/// Hand over the banner's peer-count signal. Same seeding argument as [`set_role_signal`].
pub fn set_peers_signal(sig: RwSignal<usize>) {
    sig.set(peer_count());
    PEERS_SIG.with(|s| *s.borrow_mut() = Some(sig));
}

/* ─────────────────────────── wasm: the channel and the lock ─────────────────────────── */

#[cfg(target_arch = "wasm32")]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn mint_tab_id() -> String {
    // 64 random bits, formatted hex. Not a credential and never persisted — it distinguishes tabs
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
mod live {
    use super::{
        Msg, Presence, Stamp, TabRole, CHANNEL_PREFIX, LOCK_DECIDED, ON_PEER_SAVED, PEERS,
        WRITER_LOCK_PREFIX,
    };
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;

    thread_local! {
        /// The channel object, kept alive so its `message` listener keeps firing.
        static CHANNEL: std::cell::RefCell<Option<js_sys::Object>> =
            const { std::cell::RefCell::new(None) };
        /// The mission this tab joined, so a second `join` is a no-op and `leave` knows the name.
        static JOINED: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    }

    /// `window.navigator.locks`, or `None` outside a secure context. Reflect, for the reason
    /// `core::client::lock_manager` states: the web-sys bindings are behind
    /// `--cfg=web_sys_unstable_apis`, a workspace RUSTFLAGS change for one object.
    fn lock_manager() -> Option<js_sys::Object> {
        let nav = web_sys::window()?.navigator();
        let locks = js_sys::Reflect::get(nav.as_ref(), &"locks".into()).ok()?;
        (!locks.is_undefined() && !locks.is_null()).then(|| locks.unchecked_into())
    }

    /// Ask for the mission's writer lock and hold it for the life of the page.
    ///
    /// The callback returns a promise that is never resolved, so the browser keeps the lock until
    /// this page goes away — including a crash, which is the whole reason the role is a lock and
    /// not a message. The grant is the promotion; the release is the handover to whichever tab is
    /// next in the queue, and that tab's own pending callback fires with no traffic at all.
    fn claim_writer_lock(mission_id: &str) {
        let Some(locks) = lock_manager() else {
            return; // insecure context — the presence election in `on_message` decides instead
        };
        let Ok(request) = js_sys::Reflect::get(&locks, &"request".into()) else {
            return;
        };
        let Ok(request) = request.dyn_into::<js_sys::Function>() else {
            return;
        };
        // Not the writer until the browser says so. Set BEFORE the request so the window between
        // asking and being granted is read-only rather than a second writer.
        super::set_role(TabRole::ReadOnly);
        let cb = wasm_bindgen::closure::Closure::once_into_js(move |_lock: JsValue| -> JsValue {
            LOCK_DECIDED.set(true);
            super::set_role(TabRole::Writer);
            // A promise nobody ever settles: the lock is held for as long as it is pending, and
            // this page is the only thing that can end that.
            js_sys::Promise::new(&mut |_resolve, _reject| {}).into()
        });
        let name = format!("{WRITER_LOCK_PREFIX}{mission_id}");
        if request.call2(&locks, &name.into(), &cb).is_err() {
            // `request` can reject rather than invoke (a document that is not fully active, an
            // opaque origin). Refusing to write forever on that would be worse than the race it
            // closes, so fall back to the announcement election, exactly as `with_refresh_lock`
            // falls back to running unlocked.
            super::set_role(TabRole::Writer);
        }
    }

    /// Open the channel, announce this tab, and start tracking peers. Idempotent.
    pub fn join(mission_id: &str) {
        if JOINED.with(|j| j.borrow().is_some()) {
            return;
        }
        JOINED.with(|j| *j.borrow_mut() = Some(mission_id.to_string()));
        claim_writer_lock(mission_id);
        let Some(win) = web_sys::window() else { return };
        let Ok(ctor) = js_sys::Reflect::get(win.as_ref(), &"BroadcastChannel".into()) else {
            return;
        };
        let Ok(ctor) = ctor.dyn_into::<js_sys::Function>() else {
            return;
        };
        let args = js_sys::Array::of1(&format!("{CHANNEL_PREFIX}{mission_id}").into());
        let Ok(chan) = js_sys::Reflect::construct(&ctor, &args) else {
            return;
        };
        let listener =
            wasm_bindgen::closure::Closure::<dyn FnMut(JsValue)>::new(move |ev: JsValue| {
                let Ok(data) = js_sys::Reflect::get(&ev, &"data".into()) else {
                    return;
                };
                let Some(text) = data.as_string() else { return };
                // Serde over the structured clone, for `client.rs`'s reason: a message from a
                // future build with extra fields is ignored rather than half-read.
                let Ok(msg) = serde_json::from_str::<Msg>(&text) else {
                    return;
                };
                on_message(&msg);
            });
        let target: web_sys::EventTarget = chan.clone().unchecked_into();
        if target
            .add_event_listener_with_callback("message", listener.as_ref().unchecked_ref())
            .is_ok()
        {
            // One closure per page, alive for as long as the channel is — leaked deliberately, the
            // same way `client.rs` leaks its peer-rotation listener.
            listener.forget();
            CHANNEL.with(|c| *c.borrow_mut() = Some(chan.unchecked_into()));
        }
        post(&Msg {
            kind: "hello".to_string(),
            tab: super::tab_id(),
            since: js_sys::Date::now(),
        });
    }

    /// Handle one peer announcement.
    fn on_message(msg: &Msg) {
        let me = super::tab_id();
        if msg.tab == me {
            return; // BroadcastChannel does not echo to the sender, but a relay might
        }
        match msg.kind.as_str() {
            "hello" | "here" => {
                remember(&Presence {
                    tab: msg.tab.clone(),
                    since: msg.since,
                });
                if msg.kind == "hello" {
                    // Answer, so the newcomer can count us and (with no Web Locks) elect.
                    post(&Msg {
                        kind: "here".to_string(),
                        tab: me,
                        since: opened_at(),
                    });
                }
            }
            "bye" => {
                PEERS.with(|p| p.borrow_mut().retain(|q| q.tab != msg.tab));
                super::publish_peer_count();
                reelect();
            }
            "saved" => {
                // A peer put fresh bytes at the shared key. Pull them into this document instead of
                // discovering the divergence at the next reload.
                ON_PEER_SAVED.with(|h| {
                    if let Some(f) = h.borrow().as_ref() {
                        f();
                    }
                });
            }
            _ => {}
        }
    }

    fn remember(p: &Presence) {
        PEERS.with(|peers| {
            let mut peers = peers.borrow_mut();
            match peers.iter_mut().find(|q| q.tab == p.tab) {
                Some(slot) => slot.since = p.since,
                None => peers.push(p.clone()),
            }
        });
        super::publish_peer_count();
        reelect();
    }

    /// Re-run the fallback election. **A no-op once `navigator.locks` has spoken** — the browser's
    /// answer is authoritative and a message must never demote a tab it has made the writer.
    fn reelect() {
        if LOCK_DECIDED.get() || lock_manager().is_some() {
            return;
        }
        let me = Presence {
            tab: super::tab_id(),
            since: opened_at(),
        };
        let peers = PEERS.with(|p| p.borrow().clone());
        super::set_role(super::elect(&me, &peers));
    }

    thread_local! {
        static OPENED_AT: std::cell::Cell<f64> = const { std::cell::Cell::new(f64::NAN) };
    }

    /// The instant this tab opened the mission, minted once.
    fn opened_at() -> f64 {
        let at = OPENED_AT.get();
        if at.is_nan() {
            let now = js_sys::Date::now();
            OPENED_AT.set(now);
            return now;
        }
        at
    }

    /// Post one message. Best effort: a dropped announcement costs a banner, never a document —
    /// the merge is what protects the bytes.
    pub fn post(msg: &Msg) {
        let Some(chan) = CHANNEL.with(|c| c.borrow().clone()) else {
            return;
        };
        let Ok(post) = js_sys::Reflect::get(&chan, &"postMessage".into()) else {
            return;
        };
        let Ok(post) = post.dyn_into::<js_sys::Function>() else {
            return;
        };
        if let Ok(text) = serde_json::to_string(msg) {
            let _ = post.call1(&chan, &text.into());
        }
    }

    /// Announce departure so the surviving tabs re-elect immediately instead of waiting for the
    /// Web Lock release to propagate. The lock covers the crash case; this covers the ordinary one.
    pub fn leave() {
        post(&Msg {
            kind: "bye".to_string(),
            tab: super::tab_id(),
            since: 0.0,
        });
    }

    /// Tell the other tabs a fresh record is on disk. Called from `persist::run_save`'s success
    /// branch, after the bytes actually landed — never before.
    pub fn announce_saved(at: f64) {
        post(&Msg {
            kind: "saved".to_string(),
            tab: super::tab_id(),
            since: at,
        });
    }

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    /// Who wrote the record at `physical_key`, and when. `None` when there is no stamp — which
    /// [`super::decide_save`] treats as "merge", not "mine".
    pub fn read_stamp(physical_key: &str) -> Option<Stamp> {
        let raw = storage()?
            .get_item(&super::stamp_key(physical_key))
            .ok()??;
        serde_json::from_str::<Stamp>(&raw).ok()
    }

    /// Record that this tab just wrote `physical_key`.
    pub fn write_stamp(physical_key: &str, at: f64) {
        let stamp = Stamp {
            tab: super::tab_id(),
            at,
        };
        if let (Some(store), Ok(text)) = (storage(), serde_json::to_string(&stamp)) {
            let _ = store.set_item(&super::stamp_key(physical_key), &text);
        }
    }

    /// `window.__missionTabs` — the read-only probe the editor gate and the operator checklist use
    /// to see the role, the peer count and this tab's id without opening devtools on two windows.
    pub fn register_bridge() {
        let obj = js_sys::Object::new();
        let role_fn = wasm_bindgen::closure::Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(if super::may_write() {
                "writer"
            } else {
                "read-only"
            })
        })
            as Box<dyn FnMut() -> JsValue>);
        let peers_fn = wasm_bindgen::closure::Closure::wrap(Box::new(move || -> JsValue {
            #[allow(clippy::cast_precision_loss)]
            JsValue::from_f64(super::peer_count() as f64)
        })
            as Box<dyn FnMut() -> JsValue>);
        let id_fn = wasm_bindgen::closure::Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(&super::tab_id())
        }) as Box<dyn FnMut() -> JsValue>);
        let _ = js_sys::Reflect::set(&obj, &"role".into(), role_fn.as_ref());
        let _ = js_sys::Reflect::set(&obj, &"peers".into(), peers_fn.as_ref());
        let _ = js_sys::Reflect::set(&obj, &"tab_id".into(), id_fn.as_ref());
        if let Some(win) = web_sys::window() {
            let _ = js_sys::Reflect::set(&win, &"__missionTabs".into(), &obj);
        }
        role_fn.forget();
        peers_fn.forget();
        id_fn.forget();
    }
}

#[cfg(target_arch = "wasm32")]
pub use live::{announce_saved, join, leave, read_stamp, register_bridge, write_stamp};

/// Native stand-ins, so `persist.rs`'s call sites need no second `cfg` and the host build of this
/// crate still links. The IO half of this module is a browser API by construction.
#[cfg(not(target_arch = "wasm32"))]
mod live {
    use super::Stamp;

    pub fn join(_mission_id: &str) {}
    pub fn leave() {}
    pub fn announce_saved(_at: f64) {}
    pub fn register_bridge() {}
    #[must_use]
    pub fn read_stamp(_physical_key: &str) -> Option<Stamp> {
        None
    }
    pub fn write_stamp(_physical_key: &str, _at: f64) {}
}

// `pub` does not make these reachable from outside — this crate is a `bin`, so an unused re-export
// is still an unused import. Their one caller (`persist.rs`) is wasm32-gated in `state/mod.rs`, so
// on the host nothing calls them and nothing should: they exist to keep that file's call sites free
// of a second `cfg` each.
#[cfg(not(target_arch = "wasm32"))]
#[allow(unused_imports)]
pub use live::{announce_saved, join, leave, read_stamp, register_bridge, write_stamp};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::arsenal::class_r_scrub::{live_code, live_source, only_item};

    fn persist_live() -> String {
        live_code(include_str!("persist.rs"))
    }

    fn persist_src() -> String {
        live_source(include_str!("persist.rs"))
    }

    fn overlays_src() -> String {
        live_source(include_str!("../canvas/overlays.rs"))
    }

    fn p(tab: &str, since: f64) -> Presence {
        Presence {
            tab: tab.to_string(),
            since,
        }
    }

    /* ── the policy, behaviourally ── */

    /// The F-32 repro at the level this crate can execute on the host: two tabs, one key. Before
    /// T-190 every save was an unconditional write-through, so B's debounce landed on top of A's
    /// bytes having read nothing — "the last debounce wins". The policy now answers `Merge` for
    /// exactly the case that produced the loss (a record stamped by another tab) and `Defer` for
    /// the tab that should not be writing at all.
    #[test]
    fn t190_a_foreign_record_is_merged_not_overwritten() {
        let mine = Stamp {
            tab: "tab-a".to_string(),
            at: 10.0,
        };
        let theirs = Stamp {
            tab: "tab-b".to_string(),
            at: 20.0,
        };
        assert_eq!(
            decide_save(TabRole::Writer, Some(&theirs), "tab-a"),
            SaveDecision::Merge,
            "a record another tab wrote must be read and merged, never overwritten blind"
        );
        assert_eq!(
            decide_save(TabRole::Writer, None, "tab-a"),
            SaveDecision::Merge,
            "an unattributable record is the one that may be somebody's only copy (T-221 posture)"
        );
        assert_eq!(
            decide_save(TabRole::Writer, Some(&mine), "tab-a"),
            SaveDecision::WriteThrough,
            "a record this tab wrote is a subset of its own document — no decode needed"
        );
    }

    #[test]
    fn t190_a_read_only_tab_defers_instead_of_writing_or_dropping() {
        let theirs = Stamp {
            tab: "tab-b".to_string(),
            at: 20.0,
        };
        for stamp in [None, Some(&theirs)] {
            assert_eq!(
                decide_save(TabRole::ReadOnly, stamp, "tab-a"),
                SaveDecision::Defer,
                "the second tab must not write — and must not silently drop the pending either"
            );
        }
    }

    #[test]
    fn t190_the_oldest_tab_writes_and_the_election_is_total() {
        let a = p("tab-a", 100.0);
        let b = p("tab-b", 200.0);
        assert_eq!(elect(&a, &[a.clone(), b.clone()]), TabRole::Writer);
        assert_eq!(elect(&b, &[a.clone(), b.clone()]), TabRole::ReadOnly);
        assert_eq!(elect(&a, &[]), TabRole::Writer, "alone ⇒ writer");
        // A tie on the instant must still elect exactly one, or both tabs write.
        let x = p("aaa", 100.0);
        let y = p("bbb", 100.0);
        let both = [x.clone(), y.clone()];
        assert_eq!(elect(&x, &both), TabRole::Writer);
        assert_eq!(elect(&y, &both), TabRole::ReadOnly);
    }

    #[test]
    fn t190_stamp_round_trips_and_the_key_is_namespaced() {
        let s = Stamp {
            tab: "tab-a".to_string(),
            at: 1_725_000_000_000.0,
        };
        let text = serde_json::to_string(&s).expect("stamp serialises");
        assert_eq!(serde_json::from_str::<Stamp>(&text).expect("round trip"), s);
        let key = stamp_key("u4:1234|mission-9");
        assert!(key.starts_with(STAMP_PREFIX) && key.ends_with("u4:1234|mission-9"));
    }

    #[test]
    fn t190_the_modal_timestamps_are_readable() {
        assert_eq!(ago(1_000_000.0, 1_000_000.0), "just now");
        assert_eq!(ago(1_000_000.0, 970_000.0), "30s ago");
        assert_eq!(ago(1_000_000.0, 700_000.0), "5m ago");
        assert_eq!(ago(10_000_000.0, 2_800_000.0), "2h ago");
        // A clock that moved backwards must not print a negative age.
        assert_eq!(ago(1_000.0, 9_000.0), "just now");
        assert_eq!(short_utc("2026-09-07T11:22:33Z"), "2026-09-07 11:22 UTC");
        assert_eq!(short_utc("not a date"), "not a date");
    }

    #[test]
    fn t190_the_banner_names_the_cause_and_promises_only_what_is_true() {
        let one = banner_copy(1);
        assert!(
            one.contains("Another tab") && one.contains("merged"),
            "F-32: the banner must name the SECOND TAB as the cause, not server drift. {one}"
        );
        assert!(
            banner_copy(3).contains("3 other tabs"),
            "{}",
            banner_copy(3)
        );
    }

    /* ── the wasm write path, pinned ── */

    /// RED before T-190: `run_save` read nothing before `save_state_as` put the whole key.
    #[test]
    fn t190_run_save_merges_the_stored_record_before_it_writes() {
        let live = persist_live();
        let run = only_item(&live, "async fn run_save(").to_string();
        let at_write = run
            .find("save_state_as")
            .unwrap_or_else(|| panic!("run_save must still write. run={run}"));
        let at_merge = run.find("merge_before_write").unwrap_or_else(|| {
            panic!("run_save must READ the record it is about to overwrite. run={run}")
        });
        assert!(
            at_merge < at_write,
            "the merge must run BEFORE the put, not after it. run={run}"
        );
        let before = only_item(&live, "async fn merge_before_write(").to_string();
        assert!(
            before.contains("read_raw") && before.contains("merge_stored"),
            "merge_before_write must read the stored record and merge it. before={before}"
        );
        assert!(
            before.contains("get_bytes"),
            "after a merge the document is the UNION, so the bytes must be re-encoded — otherwise \
             the write lands the pre-merge blob and the read was decoration. before={before}"
        );
        let merge = only_item(&live, "fn merge_stored(").to_string();
        assert!(
            merge.contains("apply_update"),
            "the merge must be MissionDocCore::apply_update — a CRDT union, never a JSON diff. \
             merge={merge}"
        );
    }

    /// RED before T-190: nothing on the write path knew another tab existed.
    #[test]
    fn t190_a_second_tab_cannot_silently_overwrite_the_first() {
        let live = persist_live();
        let run = only_item(&live, "async fn run_save(").to_string();
        assert!(
            run.contains("tab_lock") || run.contains("may_write"),
            "run_save must consult the cross-tab role before writing. run={run}"
        );
        assert!(
            run.contains("Defer") && run.contains("install_pending"),
            "a deferred save must be RE-ARMED, not dropped: the read-only tab inherits the writer \
             role when the other tab closes and its work has to survive to that point. run={run}"
        );
    }

    /// The channel and the lock both go through `Reflect`, per `core::client.rs`. A `web_sys::`
    /// path for either is a `Cargo.toml` / RUSTFLAGS change this slice does not own.
    #[test]
    fn t190_the_channel_and_the_lock_follow_the_client_rs_precedent() {
        let src = live_source(include_str!("tab_lock.rs"));
        let prod = src.split("#[cfg(test)]").next().expect("test module");
        assert!(
            prod.contains("BroadcastChannel") && prod.contains("Reflect::get"),
            "the channel must be reached through js_sys::Reflect"
        );
        assert!(
            prod.contains("\"locks\"") && prod.contains("navigator"),
            "the writer role must be a navigator.locks lock, so a crashed tab releases it"
        );
        assert!(
            !prod.contains("web_sys::BroadcastChannel") && !prod.contains("web_sys::LockManager"),
            "neither binding is in this crate's web-sys feature list; adding one is a Cargo.toml \
             change T-190 does not own"
        );
    }

    /// RED before T-190: `ConflictInfo` was `payload_json` + `semver` and nothing else.
    #[test]
    fn t190_conflict_info_carries_counts_and_timestamps() {
        let info = only_item(&overlays_src(), "pub struct ConflictInfo").to_string();
        for field in [
            "local_objects",
            "server_objects",
            "local_saved",
            "server_saved",
        ] {
            assert!(
                info.contains(field),
                "ConflictInfo must describe BOTH options; missing `{field}`. info={info}"
            );
        }
    }

    /// RED before T-190: "Load server version" was the affirmative `bg-primary` button and no copy
    /// anywhere said it destroys the local document.
    #[test]
    fn t190_load_server_version_is_marked_destructive() {
        let dialog = only_item(&overlays_src(), "pub(crate) fn ConflictDialog(").to_string();
        // `rfind`, so the needle is the button's visible LABEL and not its `aria-label` — the two
        // now differ, and slicing at the attribute would cut the arm before its own `class`.
        let at = dialog
            .rfind("Load server version")
            .unwrap_or_else(|| panic!("the dialog must still offer the server version. {dialog}"));
        let arm = &dialog[..at];
        let arm = &arm[arm.rfind("<button").unwrap_or(0)..];
        assert!(
            arm.contains("error"),
            "the destructive choice must use the codebase's destructive tokens \
             (text-error / bg-error), not bg-primary. arm={arm}"
        );
        assert!(
            dialog.contains("discard") || dialog.contains("Discard"),
            "the dialog must SAY that loading the server version discards local work. {dialog}"
        );
        for needle in [
            "local_objects",
            "server_objects",
            "local_saved",
            "server_saved",
        ] {
            assert!(
                dialog.contains(needle),
                "the dialog must RENDER {needle}, not merely receive it. {dialog}"
            );
        }
    }

    /// T-946.51 — RED before T-190: `SAVE_IN_FLIGHT` was `.set(true)` in `run_save`, `.set(false)`
    /// in `SaveFlightGuard::drop`, and read by nothing, while two comments credited it with the
    /// serialisation `lock_for(id)` performs.
    #[test]
    fn t946_51_save_in_flight_is_read_by_something() {
        let live = persist_live();
        let n = live
            .match_indices("SAVE_IN_FLIGHT")
            .filter(|(i, _)| {
                let tail = &live[*i..];
                !tail.starts_with("SAVE_IN_FLIGHT.set(")
                    && !tail.starts_with("SAVE_IN_FLIGHT: Cell<bool>")
                    && !tail.starts_with("SAVE_IN_FLIGHT: Cell < bool >")
            })
            .count();
        assert!(
            n > 0,
            "SAVE_IN_FLIGHT is written and never read — either make it load-bearing or delete it \
             (T-946.51)"
        );
        let flight = only_item(&live, "pub fn save_in_flight(").to_string();
        assert!(
            flight.contains("SAVE_IN_FLIGHT"),
            "the reader must be the flag itself, not a second source. flight={flight}"
        );
        let src = persist_src();
        assert!(
            !src.contains("serialized by SAVE_IN_FLIGHT"),
            "the pagehide comment must not credit SAVE_IN_FLIGHT with serialisation that \
             `lock_for(id)` performs (T-946.51)"
        );
        // And the reader has to be the one that needs it: a peer's `saved` pull must not mutate the
        // live document between this tab's own encode and its put.
        let pull = only_item(&live, "pub fn pull_peer_record(").to_string();
        assert!(
            pull.contains("save_in_flight"),
            "the peer pull must stand down while this tab's own write is in flight. pull={pull}"
        );
    }

    /// T-946.54 — RED before T-190: the `UNREADABLE` insert came after 80 + 160 + 320 ms of
    /// backoff, so `run_save` passed the T-374 guard for the whole window.
    #[test]
    fn t946_54_note_unreadable_latches_before_it_retries() {
        let item = only_item(&persist_live(), "async fn note_unreadable(").to_string();
        let latch = item
            .find("UNREADABLE.with")
            .unwrap_or_else(|| panic!("note_unreadable must touch UNREADABLE. item={item}"));
        let first_sleep = item
            .find("sleep_ms")
            .unwrap_or_else(|| panic!("note_unreadable must back off. item={item}"));
        assert!(
            latch < first_sleep,
            "the key must be latched into UNREADABLE BEFORE the first backoff, or run_save passes \
             the T-374 guard for the whole retry window (T-946.54). item={item}"
        );
        assert!(
            item.contains("remove"),
            "a read that succeeds on retry must clear the latch, or one transient blip disables \
             autosave for the page lifetime. item={item}"
        );
    }
}
