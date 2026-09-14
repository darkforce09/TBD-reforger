//! The refresh policy and the retry state machine, exercised natively against a fake transport.

use super::*;
use crate::v2::core::auth::{RefreshResponse, SingleFlight};
use futures::executor::block_on;
use futures::FutureExt;
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
    let out: Result<&str, ApiErr> = block_on(send_with_refresh(
        &sf,
        move |tok| {
            let s = s.clone();
            async move {
                s.set(s.get() + 1);
                if tok.as_deref() == Some("new") {
                    Ok("ok")
                } else {
                    Err((401u16, None))
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
    let out: Result<&str, ApiErr> = block_on(send_with_refresh(
        &sf,
        move |_tok| {
            let s = s.clone();
            async move {
                s.set(s.get() + 1);
                Err((401u16, None))
            }
            .boxed_local()
        },
        || Some("stale".to_string()),
        || async { Some(rr("new")) }.boxed_local(),
        |_| {},
    ));
    assert_eq!(out, Err((401, None)));
    assert_eq!(sends.get(), 2, "one retry only — no loop");
}

// A non-401 error is not retried and does not refresh.
#[test]
fn non_401_propagates_without_refresh() {
    let refreshes = Rc::new(Cell::new(0));
    let sf = SingleFlight::<Option<RefreshResponse>>::new();
    let r = refreshes.clone();
    let out: Result<&str, ApiErr> = block_on(send_with_refresh(
        &sf,
        |_tok| async { Err((500u16, None)) }.boxed_local(),
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
    assert_eq!(out, Err((500, None)));
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

/// The multipart upload helper must stay in the browser client, and must keep naming its form
/// field the way the upload route expects.
///
/// Read from the source because the function is browser-only and no native harness could observe
/// it honestly: the whole assertion is about form data and a request builder, so a runtime test
/// would mean writing a fake and then asserting against the fake. What is left is a shape
/// invariant, scoped to the one function rather than a whole-file search that any dead copy could
/// satisfy.
#[test]
fn api_upload_file_posts_multipart_file_field() {
    let f = item("pub async fn api_upload_file<");
    assert!(
        f.contains("FormData::new()") && f.contains("append_with_blob_and_filename("),
        "upload must build FormData (perturbation: rename/delete api_upload_file, or post JSON)"
    );
    // The field NAME is the contract with the handler, and it is a string literal — so this
    // one assertion reads `live_source` (literals kept) rather than `live_code`.
    assert!(
        item_src("pub async fn api_upload_file<")
            .contains("append_with_blob_and_filename(\"file\""),
        "the multipart field must be named \"file\" to match POST /cms/uploads"
    );
    assert!(
        f.contains("Request::post(&url)") && f.contains(".body(form)"),
        "upload must POST FormData without forcing a JSON Content-Type"
    );
}

/* ───────────────────────────── the raw-body POST ───────────────────────────── */

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
    ) -> Result<&'static str, ApiErr> {
        send_with_refresh(
            sf,
            move |tok| {
                let s = sends.clone();
                async move {
                    s.set(s.get() + 1);
                    if tok.as_deref() == Some("new") {
                        Ok("ok")
                    } else {
                        Err((401u16, None))
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
                        Err((401u16, None))
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
    let a: Result<&str, ApiErr> = once(&sf, refreshes.clone());
    let b: Result<&str, ApiErr> = once(&sf, refreshes.clone());
    assert_eq!((a, b), (Ok("ok"), Ok("ok")));
    assert_eq!(
        refreshes.get(),
        2,
        "sequential 401s each need their own rotated token — the cell must not cache"
    );
}

/// The shipped client with comments, string literals and every construct that cannot run removed.
///
/// Reading the raw text would not do: every needle these assertions look for is discussed in prose
/// somewhere in the client, so each would be satisfied by a comment line — and by anything parked
/// in a constant-false block, an uncompiled item, or after an unconditional return.
fn prod() -> String {
    crate::v2::core::test_support::class_r_scrub::live_code(
        &crate::v2::core::test_support::pins::client_source(),
    )
}

/// Same, but string literals survive — for the handful of assertions where the literal **is**
/// the contract (a wire field name, a header value, a route).
fn prod_src() -> String {
    crate::v2::core::test_support::class_r_scrub::live_source(
        &crate::v2::core::test_support::pins::client_source(),
    )
}

/// The body of the ONE item whose signature is `start`.
///
/// Scoped on purpose. A `prod.contains(…)` check is satisfied by a match ANYWHERE in the
/// file, so it can report success over code it never looked at.
///
/// It reads the scrubbed source, so a needle in a comment or a dead block does not count, and it
/// takes the balanced body of the one match rather than everything up to the next doc comment,
/// refusing outright when a signature appears twice. Otherwise it would happily hand back a
/// pristine shadow copy parked in a never-called module while the real one was cut — and that is
/// not hypothetical, since a second definition in an inner module compiles fine.
fn item(start: &str) -> String {
    crate::v2::core::test_support::class_r_scrub::only_item(&prod(), start).to_string()
}

/// [`item`], literals kept.
fn item_src(start: &str) -> String {
    crate::v2::core::test_support::class_r_scrub::only_item(&prod_src(), start).to_string()
}

/// The raw-body request exists and takes an already-serialised string.
#[test]
fn api_post_raw_takes_an_already_serialised_string_body() {
    let f = item("pub async fn api_post_raw(");
    assert!(
        f.contains("body: String"),
        "api_post_raw must take an already-serialised String — a Value would reinstate the \
         parse+reserialise pair it exists to remove"
    );
    assert!(
        f.contains("Body::Raw(std::rc::Rc::new(body))"),
        "the body must go in behind an Rc so the 401 retry shares one buffer instead of \
         duplicating the document (perturbation: pass the String by value and clone it)"
    );
    assert!(
        f.contains("Consume::Ignore(())"),
        "the 201 echoes the whole json_payload back (models/mission.rs:128); reading it would \
         be a fourth full copy the caller discards"
    );
}

/// No request path can open a second refresh route and double-spend the token.
///
/// This is a rule over the whole browser client rather than a check on one helper: every route to
/// the refresh request goes through the retry contract, and every use of that contract is handed
/// the module's own single-flight cell, so the three counts are equal. A helper that opened its own
/// refresh path, or passed a freshly constructed cell, breaks the equality.
///
/// The counted needle is the locked entry point rather than the request itself, because the
/// cross-tab lock sits between them;
/// [`the_refresh_post_is_reachable_only_from_inside_the_cross_tab_lock`] is what proves that hop is
/// real rather than a rename.
#[test]
fn every_auth_path_goes_through_the_one_single_flight_cell() {
    let p = prod();
    let cells = p.matches("REFRESH_SF.with(|s| s.clone())").count();
    let guards = p.matches("send_with_refresh(").count();
    let refreshes = p.matches("refresh_locked(store)").count();
    assert!(
        cells >= 2,
        "expected the request + upload auth paths, saw {cells}"
    );
    assert_eq!(
        (cells, guards),
        (cells, cells),
        "every send_with_refresh must be fed by REFRESH_SF: {cells} cells vs {guards} guards"
    );
    assert_eq!(
        refreshes, cells,
        "the refresh path is reachable ONLY through the single-flight: {refreshes} calls vs \
         {cells} cells (perturbation: add a POST helper that refreshes on its own)"
    );
    assert_eq!(
        p.matches("SingleFlight::new()").count(),
        1,
        "there is exactly ONE cell in the shipped client — a second would be a second token \
         spender"
    );
    let tl = p
        .split("thread_local! {")
        .nth(1)
        .expect("the cell must live in a thread_local")
        .split('}')
        .next()
        .unwrap();
    assert!(
        tl.contains("REFRESH_SF: SingleFlight<Option<RefreshResponse>> = SingleFlight::new()"),
        "that one cell must be the module-level REFRESH_SF (mirrors refresh.ts `inflight`)"
    );
}

/// The raw-body request has no transport of its own: it is a thin wrapper on the shared request
/// helper, the same one the JSON verb uses, which is what makes the test above cover it too.
#[test]
fn api_post_raw_delegates_to_the_shared_request_helper() {
    let f = item("pub async fn api_post_raw(");
    assert!(
        f.contains("request("),
        "must delegate to the shared request helper"
    );
    for banned in [
        "send_with_refresh",
        "REFRESH_SF",
        "Request::post",
        "RequestBuilder",
    ] {
        assert!(
            !f.contains(banned),
            "api_post_raw must not hand-roll `{banned}` — a second auth path is how a \
             single-use refresh token gets double-spent"
        );
    }
}

/// The raw arm must set the JSON content type itself.
///
/// The serialising call sets it as a side effect, and the raw arm is defined by skipping that call.
/// The backend's JSON extractor rejects a request without the header, so every raw upload would
/// fail, and not for a reason the response body explains.
#[test]
fn the_raw_body_arm_sets_the_json_content_type() {
    // `item_src`, not `item`: the header NAME and VALUE are string literals, and here the
    // literal is the contract with Axum's `Json` extractor rather than a mention of it.
    let f = item_src("async fn request<");
    let raw = f
        .split("Body::Raw(s) =>")
        .nth(1)
        .expect("request() must have a Body::Raw arm")
        .split("Body::None")
        .next()
        .unwrap();
    assert!(
        raw.contains(".header(\"Content-Type\", \"application/json\")"),
        "the Body::Raw arm must set Content-Type or Axum's Json extractor answers 415"
    );
    assert!(
        raw.contains(".body(s.as_str())"),
        "the raw arm must send the buffer as-is — no re-serialise"
    );
}

/// The raw path must keep surfacing the backend's findings. It inherits this by sharing the request
/// helper, so the assertion is on that helper's own non-2xx arm: a version that swallowed the body
/// would turn an exact list of what is wrong with a payload back into one generic verdict.
#[test]
fn the_shared_error_arm_still_folds_the_details_array() {
    let f = item("async fn request<");
    let err = f
        .split("} else {")
        .nth(1)
        .expect("request() must have a non-2xx else arm");
    assert!(
        err.contains("error_body_message(&v)"),
        "the non-2xx arm must fold `details` via error_body_message (perturbation: read only \
         the `error` string, or drop the body entirely)"
    );
    assert!(
        err.contains("Err((status, msg))"),
        "the caller needs the status too — upload_failure(status, ..) maps 409/413 by it"
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

/* ═════════════ the wasm binding (source pins) ═════════════ */

/// **The POST is inside the lock.** `refresh_locked` is the only caller of `refresh_via_gloo`,
/// and it reaches it through `with_refresh_lock` + `refresh_cross_tab`. A helper that called
/// the request directly would be back to per-tab-only serialisation, and
/// `every_auth_path_goes_through_the_one_single_flight_cell` alone would not notice — its
/// needle is `refresh_locked`, which a rename satisfies.
#[test]
fn the_refresh_post_is_reachable_only_from_inside_the_cross_tab_lock() {
    let p = prod();
    assert_eq!(
        p.matches("refresh_via_gloo(store,").count(),
        1,
        "the refresh POST must have exactly ONE caller — a second is a second token spender"
    );
    let locked = item("async fn refresh_locked(");
    for needed in [
        "refresh_cross_tab(",
        "with_refresh_lock",
        "refresh_via_gloo(store, token)",
        "peer_rotation_supersedes(",
        "load_persisted()",
    ] {
        assert!(
            locked.contains(needed),
            "refresh_locked must go through `{needed}` (perturbation: POST straight from the \
             single-flight and the cross-tab race is back)"
        );
    }
}

/// **The lock is Web Locks, not a `localStorage` flag.** The distinction is the whole reason
/// this primitive was chosen: the browser drops a Web Lock when the page holding it dies, so a
/// tab that crashes mid-refresh wedges nobody. A storage flag would sit there until some
/// expiry ran, and every other tab would be stuck behind a holder that no longer exists.
#[test]
fn the_cross_tab_lock_uses_web_locks_so_a_dead_tab_releases_it() {
    let src = item_src("fn lock_manager(");
    assert!(
        src.contains("\"locks\""),
        "the lock manager must be `navigator.locks` — the API that releases on page death"
    );
    let held = item("async fn with_refresh_lock(");
    assert!(
        held.contains("request.call2(") && held.contains("REFRESH_LOCK_NAME"),
        "the critical section must be `navigator.locks.request(REFRESH_LOCK_NAME, cb)`"
    );
    assert!(
        held.contains("future_to_promise"),
        "the lock is held for as long as the callback's promise is pending — the body must be \
         handed back as a promise, or the lock releases before the POST runs"
    );
    // No hand-rolled expiring flag anywhere in the shipped client: the reason Web Locks was
    // chosen is that there is no stale-lock state to expire.
    for banned in [
        "lock_expires",
        "lock_deadline",
        "LOCK_TTL",
        "lock_acquired_at",
    ] {
        assert!(
            !prod().contains(banned),
            "`{banned}` suggests a storage-flag lock with an expiry — Web Locks needs none"
        );
    }
}

/// **The peer channel never persists the access token.** The rotated pair crosses tabs in
/// memory precisely because it carries an access token, and the access token is never written to
/// storage — the auth module has its own test pinning its absence from the persisted blob. A
/// simplification that routed the pair through storage would put it on disk for every tab and every
/// later visitor.
#[test]
fn the_peer_rotation_channel_keeps_the_access_token_out_of_storage() {
    let sub = item("fn subscribe_peer_rotations(");
    let cast = item("fn broadcast_rotation(");
    for f in [&sub, &cast] {
        for banned in ["local_storage", "session_storage", "set_item", "persist("] {
            assert!(
                !f.contains(banned),
                "the peer rotation channel must not touch `{banned}` — it carries an access \
                 token, and  S5 is that the access token is never persisted"
            );
        }
    }
    assert!(
        item_src("fn subscribe_peer_rotations(").contains("\"BroadcastChannel\""),
        "the channel must be a BroadcastChannel (in-memory, dies with the page)"
    );
    assert!(
        cast.contains("postMessage") || item_src("fn broadcast_rotation(").contains("postMessage"),
        "a rotation must actually be announced, or no waiter can ever adopt"
    );
}

/// **The pin that keeps the footgun from growing back.** [`Fetched`] earns its keep only while
/// there is no one-token way to collapse it to "nothing": each of these would restore exactly
/// the `.ok.unwrap_or_default` shape that rendered a 401 as an empty list.
#[test]
fn the_fetched_type_offers_no_collapse_to_empty() {
    let p = prod();
    let start = p
        .find("impl<T> Fetched<T>")
        .expect("the Fetched impl must exist");
    let block = &p[start..];
    let block = &block[..block.find("\nimpl").unwrap_or(block.len())];
    for banned in [
        "unwrap_or_default",
        "unwrap_or_else",
        "unwrap_or(",
        "fn ok(",
        "fn into_data",
        "impl<T> Default for Fetched",
    ] {
        assert!(
            !block.contains(banned),
            "`{banned}` on Fetched hands a caller an empty value for a failed fetch, which is \
             the  bug with extra steps"
        );
    }
    assert!(
        !p.contains("impl<T: Default> Fetched<T>"),
        "a Default-bounded impl is where `unwrap_or_default` comes back"
    );
    assert!(
        block.contains("pub fn view<R>"),
        "the total reader must exist, or callers have only the partial accessors"
    );
}

/// The calibration for every source assertion in this file.
///
/// The five above are only worth their green if [`prod`] and [`item`] can still say no.
/// Each row is an attack on a needle one of them looks for, applied to a synthetic file, and
/// the scrubbed source must no longer contain it. The last two rows are the interesting ones:
/// they are shadow-copy attacks, which no amount of dead-code stripping catches — only
/// refusing ambiguity does.
#[test]
fn the_source_pins_reject_every_dead_code_wrapper() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_item};
    let needle = "Body::Raw(std::rc::Rc::new(body))";
    let attacks: [(&str, String); 12] = [
        (
            "if true == false",
            format!("if true == false {{ {needle}; }}"),
        ),
        ("loop { break; … }", format!("loop {{ break; {needle}; }}")),
        (
            "#[cfg(any())]",
            format!("#[cfg(any())] fn d() {{ {needle}; }}"),
        ),
        ("while false", format!("while false {{ {needle}; }}")),
        ("if !true", format!("if !true {{ {needle}; }}")),
        ("if 1 > 2", format!("if 1 > 2 {{ {needle}; }}")),
        (
            "if std::hint::black_box(false)",
            format!("if std::hint::black_box(false) {{ {needle}; }}"),
        ),
        (
            "const C: bool = false; if C",
            format!("const C: bool = false;\nfn d() {{ if C {{ {needle}; }} }}"),
        ),
        ("return; above", format!("fn d() {{ return; {needle}; }}")),
        (
            "#[cfg(any())] mod shadow",
            format!("#[cfg(any())] mod shadow {{ fn d() {{ {needle}; }} }}"),
        ),
        (
            "match guard",
            format!("match () {{ _ if false => {{ {needle}; }} _ => {{}} }}"),
        ),
        ("comment", format!("// {needle}")),
    ];
    for (label, body) in attacks {
        let forged = format!("pub async fn api_post_raw(\n) {{\n    {body}\n}}\n#[cfg(test)]\n");
        assert!(
            !live_code(&forged).contains(needle),
            "{label}: the needle survived scrubbing, so `api_post_raw_takes_an_already_\
             serialised_string_body` would report a live Rc body over code the build never runs"
        );
    }

    // Shadow copies: a pristine definition at column 0 with the real one moved into a `mod`,
    // and the same trick with no `cfg` anywhere to give it away. Both compile; both feed a
    // whole-file grep the wrong body. Only the ambiguity refusal in `only_body` catches them.
    for (label, forged) in [
        (
            "#[cfg(any())]-free shadow copy in a live mod",
            "pub async fn api_post_raw() { good; }\n\
             mod real { pub async fn api_post_raw() { bad; } }\n#[cfg(test)]\n",
        ),
        (
            "shadow copy nested in an impl",
            "pub async fn api_post_raw() { good; }\n\
             impl T { pub async fn api_post_raw() { bad; } }\n#[cfg(test)]\n",
        ),
    ] {
        let scrubbed = live_code(forged);
        let caught =
            std::panic::catch_unwind(|| only_item(&scrubbed, "pub async fn api_post_raw("))
                .is_err();
        assert!(
            caught,
            "{label}: two definitions of one helper and the pin picked one without saying so — \
             a grep cannot tell which body ships, so it must refuse rather than guess"
        );
    }

    // The honest shape still reads as present, or every assertion above pins nothing.
    let live = format!("pub async fn api_post_raw() {{\n    {needle};\n}}\n#[cfg(test)]\n");
    assert!(live_code(&live).contains(needle));
}
