//! The refresh policy and the retry state machine, exercised natively against a fake transport.

use super::*;
use crate::client::SingleFlight;
use crate::error::Error;
use frontend_api_dtos::RefreshResponse;
use futures::FutureExt;
use futures::executor::block_on;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn rr(access: &str) -> RefreshResponse {
    RefreshResponse {
        access_token: access.into(),
        refresh_token: "r".into(),
        expires_at: "e".into(),
    }
}

// The api/client.ts contract: a 401 refreshes once and retries once with the new token.
#[test]
fn retries_once_after_refresh() {
    let sends = Rc::new(Cell::new(0));
    let refreshes = Rc::new(Cell::new(0));
    let sf = SingleFlight::<Option<RefreshResponse>>::new();
    let s = sends.clone();
    let r = refreshes.clone();
    let out: Result<&str, Error> = block_on(send_with_refresh(
        &sf,
        move |tok| {
            let s = s.clone();
            async move {
                s.set(s.get() + 1);
                if tok.as_deref() == Some("new") {
                    Ok("ok")
                } else {
                    Err(Error::from_status(401, None))
                }
            }
            .boxed_local()
        },
        || Some("stale".to_string()),
        move || {
            let r = r.clone();
            async move {
                r.set(r.get() + 1);
                Some(rr("new"))
            }
            .boxed_local()
        },
        |_| {},
    ));
    assert_eq!(out, Ok("ok"));
    assert_eq!(refreshes.get(), 1, "exactly one refresh");
    assert_eq!(sends.get(), 2, "original + exactly one retry");
}

// No retry loop: a still-401 retry gives up (send twice total, then propagate 401).
#[test]
fn no_loop_if_retry_still_401() {
    let sends = Rc::new(Cell::new(0));
    let sf = SingleFlight::<Option<RefreshResponse>>::new();
    let s = sends.clone();
    let out: Result<&str, Error> = block_on(send_with_refresh(
        &sf,
        move |_tok| {
            let s = s.clone();
            async move {
                s.set(s.get() + 1);
                Err(Error::from_status(401, None))
            }
            .boxed_local()
        },
        || Some("stale".to_string()),
        || async { Some(rr("new")) }.boxed_local(),
        |_| {},
    ));
    assert_eq!(out, Err(Error::from_status(401, None)));
    assert_eq!(sends.get(), 2, "one retry only — no loop");
}

// A non-401 error is not retried and does not refresh.
#[test]
fn non_401_propagates_without_refresh() {
    let refreshes = Rc::new(Cell::new(0));
    let sf = SingleFlight::<Option<RefreshResponse>>::new();
    let r = refreshes.clone();
    let out: Result<&str, Error> = block_on(send_with_refresh(
        &sf,
        |_tok| async { Err(Error::from_status(500, None)) }.boxed_local(),
        || Some("t".to_string()),
        move || {
            let r = r.clone();
            async move {
                r.set(r.get() + 1);
                Some(rr("new"))
            }
            .boxed_local()
        },
        |_| {},
    ));
    assert_eq!(out, Err(Error::from_status(500, None)));
    assert_eq!(refreshes.get(), 0, "non-401 never refreshes");
}

// The reason a rejection is diagnosable at all. A client that keeps only the headline turns a
// response listing four specific problems into one generic verdict.
#[test]
fn details_ride_along_with_the_error_string() {
    let body = serde_json::json!({
        "error": "invalid mission payload",
        "details": ["/editor/squads/0/callsign: bad", "/editor/slots/9/role: bad"],
    });
    let msg = error_body_message(&body).expect("message");
    let (head, rows) = split_error_lines(Some(&msg));
    assert_eq!(head.as_deref(), Some("invalid mission payload"));
    assert_eq!(rows.len(), 2);
    assert!(rows[0].starts_with("/editor/squads/0/callsign:"));
}

#[test]
fn a_long_findings_list_is_capped_and_the_tail_is_counted() {
    let details: Vec<String> = (0..MAX_ERROR_DETAILS + 4)
        .map(|i| format!("f{i}"))
        .collect();
    let body = serde_json::json!({"error": "invalid mission payload", "details": details});
    let (_, rows) = split_error_lines(error_body_message(&body).as_deref());
    assert_eq!(rows.len(), MAX_ERROR_DETAILS + 1);
    assert_eq!(rows[MAX_ERROR_DETAILS], "… and 4 more");
}

// `field_tools` puts a partial mortar SOLUTION in `details`. That is a payload for the caller
// to render, not prose, and folding it into the message would be gibberish.
#[test]
fn non_string_details_are_left_alone() {
    let body = serde_json::json!({
        "error": "target out of range",
        "details": {"range": 812.0, "charge": 3},
    });
    assert_eq!(
        error_body_message(&body).as_deref(),
        Some("target out of range")
    );
    assert!(error_body_message(&serde_json::json!({"detail": "x"})).is_none());
}

/* ─────────────────────────── the single-flight refresh ─────────────────────────── */

/// A future that is `Pending` on its FIRST poll and ready after.
///
/// Load-bearing for [`two_concurrent_401s_share_one_refresh`]: `block_on(join(a, b))` polls
/// `a` to completion before it ever touches `b`, so with an instantly-ready refresh the two
/// callers are never concurrent, the cell is already cleared when `b` arrives, and a SECOND
/// refresh is the *correct* answer — the test would pass with the single-flight ripped out.
/// Parking on the first poll is what forces both callers into the cell at once.
fn pending_once<T: 'static>(v: T) -> futures::future::LocalBoxFuture<'static, T> {
    let mut polled = false;
    let mut val = Some(v);
    futures::future::poll_fn(move |cx| {
        if polled {
            std::task::Poll::Ready(val.take().expect("polled after Ready"))
        } else {
            polled = true;
            cx.waker().wake_by_ref();
            std::task::Poll::Pending
        }
    })
    .boxed_local()
}

/// **The property `api_post_raw` must not be allowed to break.** Refresh tokens are single-use
/// and rotated (`auth.rs` header), so two requests that 401 together must spend ONE token —
/// the second presenting the same spent token would 401 and wrongly clear the session.
///
/// The single-caller retry test above cannot see this: it is green whether or not the shared cell
/// is consulted at all.
#[test]
fn two_concurrent_401s_share_one_refresh() {
    let sends = Rc::new(Cell::new(0));
    let refreshes = Rc::new(Cell::new(0));
    let sf = SingleFlight::<Option<RefreshResponse>>::new();

    // One caller of the retry contract, sharing `sf` with its twin.
    async fn one(
        sf: &SingleFlight<Option<RefreshResponse>>,
        sends: Rc<Cell<u32>>,
        refreshes: Rc<Cell<u32>>,
    ) -> Result<&'static str, Error> {
        send_with_refresh(
            sf,
            move |tok| {
                let s = sends.clone();
                async move {
                    s.set(s.get() + 1);
                    if tok.as_deref() == Some("new") {
                        Ok("ok")
                    } else {
                        Err(Error::from_status(401, None))
                    }
                }
                .boxed_local()
            },
            || Some("stale".to_string()),
            move || {
                refreshes.set(refreshes.get() + 1);
                pending_once(Some(rr("new")))
            },
            |_| {},
        )
        .await
    }

    let (a, b) = block_on(futures::future::join(
        one(&sf, sends.clone(), refreshes.clone()),
        one(&sf, sends.clone(), refreshes.clone()),
    ));

    assert_eq!(a, Ok("ok"));
    assert_eq!(b, Ok("ok"));
    assert_eq!(
        refreshes.get(),
        1,
        "the single-use refresh token must be spent ONCE for two concurrent 401s \
         (perturbation: call refresh directly instead of sf.run(refresh))"
    );
    assert_eq!(sends.get(), 4, "two originals + two retries");
}

/// The other half of single-flight: the cell is cleared once the refresh settles, so a LATER
/// 401 gets a fresh token rather than replaying a spent one. A cache would be just as green on
/// the concurrent test above and would resurrect the double-spend it prevents.
#[test]
fn the_cell_clears_so_a_later_401_refreshes_again() {
    let refreshes = Rc::new(Cell::new(0));
    let sf = SingleFlight::<Option<RefreshResponse>>::new();
    let once = |sf: &SingleFlight<Option<RefreshResponse>>, refreshes: Rc<Cell<u32>>| {
        block_on(send_with_refresh(
            sf,
            move |tok| {
                async move {
                    if tok.as_deref() == Some("new") {
                        Ok("ok")
                    } else {
                        Err(Error::from_status(401, None))
                    }
                }
                .boxed_local()
            },
            || Some("stale".to_string()),
            move || {
                refreshes.set(refreshes.get() + 1);
                pending_once(Some(rr("new")))
            },
            |_| {},
        ))
    };
    let a: Result<&str, Error> = once(&sf, refreshes.clone());
    let b: Result<&str, Error> = once(&sf, refreshes.clone());
    assert_eq!((a, b), (Ok("ok"), Ok("ok")));
    assert_eq!(
        refreshes.get(),
        2,
        "sequential 401s each need their own rotated token — the cell must not cache"
    );
}

/* ═════════════ the cross-tab refresh mutex (policy, native) ═════════════ */

/// `/auth/refresh` as it actually behaves: **single-use and rotating**.
///
/// This is the piece that makes the cross-tab tests able to fail. A fake that always returns a
/// fresh pair is green whether or not the tabs coordinate — it never models the one rule the bug is
/// about. Presenting a token that is not the live one is a spent token, and this refuses it, which
/// is the refusal that killed a real session.
struct FakeAuthServer {
    live: RefCell<String>,
    posts: Cell<u32>,
    seq: Cell<u32>,
}

impl FakeAuthServer {
    fn new(initial: &str) -> Self {
        Self {
            live: RefCell::new(initial.to_string()),
            posts: Cell::new(0),
            seq: Cell::new(0),
        }
    }

    fn refresh(&self, presented: Option<String>) -> Option<RefreshResponse> {
        self.posts.set(self.posts.get() + 1);
        let presented = presented?;
        if presented != *self.live.borrow() {
            return None; // already rotated away — the double-spend 401
        }
        self.seq.set(self.seq.get() + 1);
        let n = self.seq.get();
        let next = format!("r{n}");
        *self.live.borrow_mut() = next.clone();
        Some(RefreshResponse {
            access_token: format!("a{n}"),
            refresh_token: next,
            expires_at: format!("e{n}"),
        })
    }
}

/// Everything one simulated tab needs: the shared server, the shared `tbd-auth` blob, the
/// shared cross-tab lock, the shared broadcast slot, and a trace of lock events.
#[derive(Clone)]
struct Origin {
    server: Rc<FakeAuthServer>,
    storage: Rc<RefCell<Option<String>>>,
    lock: Rc<futures::lock::Mutex<()>>,
    peer: Rc<RefCell<Option<RefreshResponse>>>,
    trace: Rc<RefCell<Vec<String>>>,
}

impl Origin {
    fn new(initial: &str) -> Self {
        Self {
            server: Rc::new(FakeAuthServer::new(initial)),
            storage: Rc::new(RefCell::new(Some(initial.to_string()))),
            lock: Rc::new(futures::lock::Mutex::new(())),
            peer: Rc::new(RefCell::new(None)),
            trace: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// One tab's refresh. `entry` is the tab's own in-memory copy of the refresh token — the
    /// stale one, in the race. `broadcast` mirrors the real client announcing its rotation.
    async fn tab(
        &self,
        who: &'static str,
        entry: &str,
        broadcast: bool,
    ) -> Option<RefreshResponse> {
        let (lock, trace) = (self.lock.clone(), self.trace.clone());
        let (server, storage, peer) =
            (self.server.clone(), self.storage.clone(), self.peer.clone());
        let (out_storage, out_peer) = (storage.clone(), peer.clone());
        let out = super::refresh_cross_tab(
            Some(entry.to_string()),
            move |body| async move {
                trace.borrow_mut().push(format!("{who}:want"));
                let held = lock.lock().await;
                trace.borrow_mut().push(format!("{who}:hold"));
                let out = body.await;
                drop(held);
                trace.borrow_mut().push(format!("{who}:free"));
                out
            },
            move |about_to_spend| {
                peer.borrow()
                    .clone()
                    .filter(|p| super::peer_rotation_supersedes(p, about_to_spend))
            },
            move || storage.borrow().clone(),
            move |token| {
                async move {
                    // Park before answering: without a real await point inside the critical
                    // section `block_on(join(a, b))` runs A to completion before it ever polls
                    // B, the tabs are never concurrent, and a no-lock build passes.
                    pending_once(()).await;
                    server.refresh(token)
                }
                .boxed_local()
            },
        )
        .await;
        // What the real client does on a successful rotation: persist the refresh token
        // and tell the peer tabs.
        if let Some(r) = &out {
            *out_storage.borrow_mut() = Some(r.refresh_token.clone());
            if broadcast {
                *out_peer.borrow_mut() = Some(r.clone());
            }
        }
        out
    }
}

/// The incident, reproduced and cured. Two tabs hold the same refresh token in their own signals
/// and both fail authentication at once. Without the cross-tab lock they both spend it, and the
/// loser's session dies while it was doing nothing wrong.
///
/// Both must come out with a live session. Note what is *not* asserted: that only one rotation
/// happens. Without a peer broadcast the honest outcome is two rotations — the waiter re-reads
/// the rotated token and spends **that**, which the server accepts. One rotation is the job of
/// the broadcast, tested next.
#[test]
fn two_tabs_racing_one_single_use_token_both_keep_their_session() {
    let o = Origin::new("r0");
    let (a, b) = block_on(futures::future::join(
        o.tab("A", "r0", false),
        o.tab("B", "r0", false),
    ));
    assert!(a.is_some(), "tab A must get a rotation");
    assert!(
        b.is_some(),
        "tab B was silently logged out — it re-presented a token tab A had already spent \
         (perturbation: spend `entry_token` instead of re-reading `stored` under the lock)"
    );
    assert_ne!(
        a.as_ref().map(|r| &r.refresh_token),
        b.as_ref().map(|r| &r.refresh_token),
        "each tab must end on its own rotation, not a shared stale one"
    );
    assert_eq!(o.server.posts.get(), 2, "one POST per tab, both accepted");
}

/// The lock is **held across the whole refresh** and released after — not taken and dropped
/// before the POST, which would serialize nothing.
#[test]
fn the_lock_is_held_across_the_post_and_released_after() {
    let o = Origin::new("r0");
    block_on(futures::future::join(
        o.tab("A", "r0", false),
        o.tab("B", "r0", false),
    ));
    let t = o.trace.borrow().clone();
    let idx = |needle: &str| t.iter().position(|e| e == needle).expect(needle);
    // Whoever holds first must free before the other can hold: strict interleaving = mutual
    // exclusion. Either order is fine; overlap is not.
    let (first, second) = if idx("A:hold") < idx("B:hold") {
        ("A", "B")
    } else {
        ("B", "A")
    };
    assert!(
        idx(&format!("{first}:free")) < idx(&format!("{second}:hold")),
        "the second tab entered the critical section before the first left it: {t:?}"
    );
    assert!(
        t.contains(&"A:free".to_string()) && t.contains(&"B:free".to_string()),
        "both tabs must release the lock: {t:?}"
    );
}

/// **The waiter adopts rather than spends.** When the winner announces its rotation, the tab
/// behind it takes that pair and never POSTs — N tabs, one rotation, which is what the ticket
/// asks for.
#[test]
fn the_waiter_adopts_the_peer_rotation_instead_of_spending_a_second_one() {
    let o = Origin::new("r0");
    let (a, b) = block_on(futures::future::join(
        o.tab("A", "r0", true),
        o.tab("B", "r0", true),
    ));
    assert!(a.is_some() && b.is_some(), "both tabs keep their session");
    assert_eq!(
        a.as_ref().map(|r| &r.access_token),
        b.as_ref().map(|r| &r.access_token),
        "the waiter must adopt the winner's pair, access token included"
    );
    assert_eq!(
        o.server.posts.get(),
        1,
        "two tabs, ONE rotation (perturbation: drop the peer_pair adopt step and this becomes 2)"
    );
}

/// The adopt step must not fire on the tab's **own** pair, or a tab that already adopted could
/// never renew its access token again — it would adopt the same expired pair forever.
#[test]
fn a_tab_does_not_adopt_the_pair_it_is_already_holding() {
    let held = rr("new"); // refresh_token == "r"
    assert!(
        !peer_rotation_supersedes(&held, Some("r")),
        "the pair this tab already holds is not a peer rotation"
    );
    assert!(
        peer_rotation_supersedes(&held, Some("r-older")),
        "a pair carrying a different refresh token IS a peer rotation"
    );
    assert!(
        peer_rotation_supersedes(&held, None),
        "a tab with no token of its own has nothing that could match"
    );

    // …and end to end: a broadcast that is already this tab's token must not short-circuit
    // the refresh. Tab A rotates r0→r1 and announces it; tab B has already adopted r1 and now
    // needs its own renewal.
    let o = Origin::new("r0");
    block_on(o.tab("A", "r0", true));
    let before = o.server.posts.get();
    let b = block_on(o.tab("B", "r1", true));
    assert!(b.is_some(), "the second tab must still be able to refresh");
    assert_eq!(
        o.server.posts.get(),
        before + 1,
        "holding the announced pair must NOT suppress a later, genuine refresh"
    );
}

/// A failed refresh must still release the lock. A lock released only on success is one that
/// a single network blip wedges for every tab, permanently.
#[test]
fn a_failed_refresh_still_releases_the_lock() {
    let o = Origin::new("r0");
    // A session that really is over: this tab's own copy AND the shared blob are both a token
    // the server has rotated away. (Killing only the tab's copy proves nothing — the re-read
    // would find the live one in storage and the refresh would succeed, which is the whole
    // point of `two_tabs_racing_one_single_use_token_both_keep_their_session`.)
    *o.storage.borrow_mut() = Some("rX".into());
    let a = block_on(o.tab("A", "rX", false));
    assert!(a.is_none(), "the fake server must refuse a dead token");
    assert!(
        o.trace.borrow().contains(&"A:free".to_string()),
        "the lock was not released after a failed refresh: {:?}",
        o.trace.borrow()
    );
    // The proof that "released" is real and not just a trace line: the next tab gets through.
    *o.storage.borrow_mut() = Some("r0".into());
    assert!(
        block_on(o.tab("B", "r0", false)).is_some(),
        "a later tab must be able to take the lock the failed one held"
    );
}
