//! Coordinates the browser tab writer role and messages.
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
    /// The mission this tab joined, so a re-join of the SAME mission is a no-op.
    static JOINED: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    /// The writer lock's `resolve`. Calling it settles the promise that holds the lock, which
    /// is the only way this page can release it early  see [`release`].
    static RELEASE: std::cell::RefCell<Option<js_sys::Function>> =
        const { std::cell::RefCell::new(None) };
}

/// Let go of the current mission: release the writer lock and drop the channel and peer list.
///
/// Needed because the editor is a **client-side route**. Opening mission A and then mission B in
/// one page never reloads the wasm module, so without this the tab would still hold A's writer
/// lock and A's channel while editing B  B would get no presence at all (`join` would find
/// itself already joined) and a third tab genuinely on A would be told, wrongly and for as long
/// as this page lived, that A was taken.
fn release() {
    if let Some(resolve) = RELEASE.with(|r| r.borrow_mut().take()) {
        let _ = resolve.call0(&JsValue::UNDEFINED);
    }
    LOCK_DECIDED.set(false);
    CHANNEL.with(|c| *c.borrow_mut() = None);
    PEERS.with(|p| p.borrow_mut().clear());
    OPENED_AT.set(f64::NAN); // re-mint: the fallback election orders tabs by when they opened
    super::publish_peer_count();
}

/// `window.navigator.locks`, or `None` outside a secure context. Reflect, for the reason
/// `core::client::lock_manager` states: the web-sys bindings are behind
/// `--cfg=web_sys_unstable_apis`, a workspace RUSTFLAGS change for one object.
fn lock_manager() -> Option<js_sys::Object> {
    let nav = web_sys::window()?.navigator();
    let locks = js_sys::Reflect::get(nav.as_ref(), &"locks".into()).ok()?;
    (!locks.is_undefined() && !locks.is_null()).then(|| locks.unchecked_into())
}

/// Ask for the mission's writer lock and hold it until this page lets go or goes away.
///
/// The callback returns a promise that settles only when [`release`] calls its `resolve`, so
/// the browser keeps the lock through everything else  including a crash, which is the whole
/// reason the role is a lock and not a message. The grant is the promotion; the release is the
/// handover to whichever tab is next in the queue, and that tab's own pending callback fires
/// with no traffic at all.
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
        // The lock is held for exactly as long as this promise is pending, and only `release`
        // holds the handle that can settle it.
        js_sys::Promise::new(&mut |resolve, _reject| {
            RELEASE.with(|r| *r.borrow_mut() = Some(resolve));
        })
        .into()
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

/// Open the channel, announce this tab, and start tracking peers.
///
/// Idempotent for the same mission; for a **different** one it says goodbye and lets go first
/// (see [`release`])  the editor is a client-side route and the wasm module outlives it.
pub fn join(mission_id: &str) {
    let already = JOINED.with(|j| j.borrow().clone());
    match already {
        Some(prev) if prev == mission_id => return,
        Some(_) => {
            leave();
            release();
        }
        None => {}
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
    let listener = wasm_bindgen::closure::Closure::<dyn FnMut(JsValue)>::new(move |ev: JsValue| {
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
        // One closure per page, alive for as long as the channel is  leaked deliberately, the
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

/// Re-run the fallback election. **A no-op once `navigator.locks` has spoken** the browser's
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
/// branch, after the bytes actually landed  never before.
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

/// Who wrote the record at `physical_key`, and when. `None` when there is no stamp  which
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

/// `window.__missionTabs`  the read-only probe the editor gate and the operator checklist use
/// to see the role, the peer count and this tab's id without opening devtools on two windows.
pub fn register_bridge() {
    let obj = js_sys::Object::new();
    let role_fn = wasm_bindgen::closure::Closure::wrap(Box::new(move || -> JsValue {
        JsValue::from_str(if super::may_write() {
            "writer"
        } else {
            "read-only"
        })
    }) as Box<dyn FnMut() -> JsValue>);
    let peers_fn = wasm_bindgen::closure::Closure::wrap(Box::new(move || -> JsValue {
        #[allow(clippy::cast_precision_loss)]
        JsValue::from_f64(super::peer_count() as f64)
    }) as Box<dyn FnMut() -> JsValue>);
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
