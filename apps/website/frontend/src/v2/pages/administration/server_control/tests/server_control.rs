//! Guards on server control: the RCON request and reply contract, and the reply path itself.

use super::*;

#[test]
fn rcon_path_tracks_app_rs() {
    let app_rs = crate::v2::core::test_support::fixtures::api_app_source();
    let app_rs: &str = app_rs;
    assert!(
        app_rs.contains(r#""/admin/servers/{id}/rcon""#),
        "http_router.rs must register POST /admin/servers/{{id}}/rcon"
    );
    assert_eq!(
        admin_server_rcon_path("00000000-0000-4000-d000-000000000001"),
        "/admin/servers/00000000-0000-4000-d000-000000000001/rcon"
    );
}

#[test]
fn rcon_bodies_match_live_action_enum() {
    assert_eq!(rcon_body_restart(), json!({ "action": "restart" }));
    assert_eq!(
        rcon_body_change_map("Everon"),
        json!({ "action": "change_map", "map": "Everon" })
    );
    assert_eq!(
        rcon_body_custom("#restart"),
        json!({ "action": "custom", "command": "#restart" })
    );
    assert_eq!(rcon_body_kick(), json!({ "action": "kick" }));
}

/// The shipped half of this page, scrubbed of comments and unreachable branches.
///
/// String literals survive the scrub: a route path and a `data-testid` are the contract here, not
/// a mention of one.
fn live() -> String {
    crate::v2::core::test_support::class_r_scrub::live_source(
        &crate::v2::core::test_support::pins::server_control_source(),
    )
}

/// Pure source-shape bans and wiring seams inside `view!` macros, which have no runtime signature
/// to observe. The scrub is what makes them meaningful: reading the raw file would include this
/// test module and every comment, so a banned string surviving only in prose would fail the ban,
/// and a needle surviving only in prose would satisfy the positive.
#[test]
fn no_mock_servers_or_fabricated_console() {
    // The needles are assembled at runtime so the pin cannot go green off this test's own
    // literals. Belt and braces: the scrub already cuts the test module out of `live()`.
    let src = live();
    let mock = format!("{}{}", "MOCK_", "SERVERS");
    let fake_listener = format!("{}{}", "RCON listener bound", " to 0.0.0.0:19999");
    let fake_init = format!("{}{}", "Server initialized on ", "Everon");
    assert!(
        !src.contains(&mock),
        "compile-time mock server table must be gone (perturbation: reintroduce it)"
    );
    assert!(
        !src.contains(&fake_listener) && !src.contains(&fake_init),
        "fabricated console log lines must be gone"
    );
}

#[test]
fn loads_servers_via_typed_api() {
    let src = live();
    let get = format!("{}{}", "api_get", "::<DataEnvelope<ServerRowDto>>");
    assert!(
        src.contains(&get) && src.contains(r#""/servers""#),
        "page must GET /servers as DataEnvelope<ServerRowDto> on a live path"
    );
}

#[test]
fn restart_and_console_post_rcon() {
    let src = live();
    let post = format!("{}{}", "api_post", "::<RconAccepted>");
    assert!(
        src.contains(&post) && src.contains("admin_server_rcon_path"),
        "Restart/console must POST via admin_server_rcon_path + api_post::<RconAccepted>"
    );
    assert!(
        src.contains("rcon_body_restart") && src.contains("rcon_body_custom"),
        "live body helpers must drive Restart and console send"
    );
    // Success toast must name acceptance, not a fake process result.
    let fake_ok = format!("{}{}", "Server restarted", " successfully");
    assert!(
        !src.contains(&fake_ok),
        "must not toast fabricated process success"
    );
    // The notification copy must track the transport that actually exists. Deliberately not
    // another source grep: the page discusses the transport in its own documentation and in this
    // very file, so a grep for either wording would go green off prose no operator ever sees.
    // Calling the live formatter is the only assertion that provably reads the code path the
    // `Ok` arm of `post_rcon` renders.
    let toast = rcon_accepted_message(&accepted_reply());
    assert!(
        !toast.contains("transport pending"),
        "T-289 shipped the host agent and T-595 shipped the client — a 202 toast may no \
         longer call the transport pending. Got: {toast}"
    );
    assert!(
        toast.contains("delivered") && toast.contains("state=active"),
        "a 202 toast must report the delivery and the state the host actually re-read, \
         not a queued-audit placeholder. Got: {toast}"
    );
}

#[test]
fn stop_and_unmapped_quick_actions_are_honestly_disabled() {
    let src = live();
    assert!(
        src.contains(r#"data-testid="server-control-stop""#)
            && src.contains("No Stop HTTP or RCON endpoint"),
        "Stop must be disabled with honest copy (no silent success)"
    );
    assert!(
        src.contains("No RCON action for modpack swap")
            && src.contains("No RCON action for global broadcast"),
        "Swap Modpack / Global Broadcast must be disabled — no matching action enum"
    );
    let mock_stop = format!("{}{}", "Server stopped", " (mock)");
    assert!(
        !src.contains(&mock_stop),
        "Stop must not toast a mock success"
    );
}

#[test]
fn classify_prompt_field_trims_and_rejects_blank() {
    assert_eq!(classify_prompt_field(None), PromptField::Abort);
    assert_eq!(classify_prompt_field(Some("")), PromptField::Reject);
    assert_eq!(classify_prompt_field(Some("   ")), PromptField::Reject);
    assert_eq!(
        classify_prompt_field(Some("  Everon  ")),
        PromptField::Send("Everon".to_string())
    );
}

/// The exact 202 body `admin.rs` emits for a restart the host agent confirmed — the same
/// literal `game_agent.rs::parses_the_agents_exact_replies` pins on the API side.
fn accepted_reply() -> RconAccepted {
    RconAccepted {
        accepted: true,
        action: "restart".into(),
        delivered: true,
        state: "active".into(),
        detail: "unit active after restart".into(),
    }
}

#[test]
fn rcon_accepted_message_is_honest() {
    let msg = rcon_accepted_message(&accepted_reply());
    assert!(
        msg.contains("delivered") && msg.contains("restart"),
        "got: {msg}"
    );
    // The state is the evidence; a message that omits it is asserting, not reporting.
    assert!(msg.contains("state=active"), "got: {msg}");
    assert!(msg.contains("unit active after restart"), "got: {msg}");
    assert!(!msg.to_lowercase().contains("restarted successfully"));
    assert!(
        !msg.contains("T-269") && !msg.contains("pending"),
        "the transport shipped in T-289/T-595 — this may not still call it pending: {msg}"
    );
}

/// The three non-success shapes must each read as their own failure.
///
/// The signature defect this guards is a tool reporting success over an input it never
/// examined. Every assertion below is against the live formatter's output, so a message that
/// stopped consulting `delivered` / `accepted` fails here rather than shipping a green toast.
#[test]
fn rcon_message_never_reads_as_success_without_delivery() {
    // Delivered, but the unit did not get there — the whole reason the host re-reads it, since
    // restarting the service exits cleanly over a dead game server.
    let refused = rcon_accepted_message(&RconAccepted {
        accepted: false,
        action: "restart".into(),
        delivered: true,
        state: "failed".into(),
        detail: "unit is failed after restart; systemctl rc=0".into(),
    });
    assert!(
        refused.contains("NOT reach the expected state") && refused.contains("state=failed"),
        "delivered-but-refused must name the state it did reach: {refused}"
    );
    assert!(
        !refused.contains("re-read the unit as"),
        "must not borrow the confirmed-delivery wording: {refused}"
    );

    // Nothing reached the host.
    let undelivered = rcon_accepted_message(&RconAccepted {
        accepted: false,
        action: "restart".into(),
        delivered: false,
        state: "unknown".into(),
        detail: "unit not installed: not-found".into(),
    });
    assert!(
        undelivered.contains("NOT delivered") && undelivered.contains("state=unknown"),
        "got: {undelivered}"
    );

    // A body that contradicts itself. `admin.rs` cannot emit this today — that is a property
    // of the server, and this page does not get to assume it.
    let contradictory = rcon_accepted_message(&RconAccepted {
        accepted: true,
        action: "restart".into(),
        delivered: false,
        state: "failed".into(),
        detail: "agent said accepted over an undelivered verb".into(),
    });
    assert!(
        contradictory.contains("NOT delivered") && contradictory.contains("FAILED"),
        "`accepted` without `delivered` must never render as success: {contradictory}"
    );

    // And the shared predicate the toast branch keys on agrees with all three.
    for (label, delivered, accepted) in [
        ("delivered-not-accepted", true, false),
        ("not-delivered", false, false),
        ("contradictory", false, true),
    ] {
        let reply = RconAccepted {
            accepted,
            action: "restart".into(),
            delivered,
            state: "failed".into(),
            detail: "x".into(),
        };
        assert!(
            !rcon_reports_success(&reply),
            "{label} must not be reported as a success"
        );
    }
    assert!(rcon_reports_success(&accepted_reply()));
}

/// The `Ok` arm must branch on the body, not on the 2xx status that got it there.
///
/// # This is **cure 1**: the pinned code is compiled and run, not greped
///
/// The branch lives inside `#[cfg(target_arch = "wasm32")]` in [`post_rcon`], so the native
/// test binary cannot link it — which is why every generation of this pin was a source grep.
/// It was a grep for `"if rcon_reports_success" + "(&resp) {"` plus a count of `toasts.success(`
/// occurrences, and both are defeatable the same way every Class-R grep in this repo has been:
/// park the guard in `if false { … }`, `#[cfg(any())]`, or after a `return;` and the needle is
/// still in the file. The count made it worse, not better — a *second* success toast in dead
/// code would have failed the count while changing nothing, and moving the only live toast
/// into dead code would have passed it.
///
/// So the instrument changed rather than the pattern. [`harness`] lifts `post_rcon`,
/// `append_log`, `admin_server_rcon_path`,
/// `rcon_reports_success` and `rcon_accepted_message` **verbatim** out of this file, resolves
/// the two `target_arch` gates the way the shipped wasm build resolves them
/// ([`class_r_scrub::resolve_wasm_cfg`], which refuses any gate it cannot decide), compiles the
/// result against a recording stand-in for the toast host and the API client, and **runs** it
/// once per 202 body shape. The assertions are on the toasts that were actually raised.
///
/// Dead code raises no toast, so every wrapper — the ten in the handed-down battery and the
/// ones nobody has invented yet — fails by construction rather than by enumeration. The
/// shadow-copy attack fails one step earlier: `only_item` refuses two definitions of
/// `post_rcon` rather than picking one.
///
/// ## What a GREEN here does and does not claim
///
/// * **Does:** for each of the four `(delivered, accepted)` bodies and for a transport error,
///   the source as committed raises exactly one toast, on the severity the body warrants, with
///   the text the real [`rcon_accepted_message`] produces — and clears `busy` either way.
/// * **Does not:** say anything about `crate::v2::core::ui::toast::Toasts` itself, or about the colour the
///   console renders `RCON:` in. Those are other modules' pins.
/// * **Residual:** the harness reads its evidence from the generated program's stdout, so
///   production source that printed the sentinels could forge a record. That is the
///   irreducible limit of running code you are also judging — it is sabotage, not a wrapper.
#[test]
fn ok_arm_toasts_success_only_when_the_body_says_delivered() {
    // (delivered, accepted, expected severity). The last row cannot be emitted by `admin.rs`
    // today; it is exactly the shape this page must never render as success, so it is driven
    // anyway rather than assumed away.
    let lines = harness::run();
    for (delivered, accepted, want) in [
        (true, true, "success"),
        (true, false, "error"),
        (false, false, "error"),
        (false, true, "error"),
    ] {
        let channel = format!("d{delivered}-a{accepted}");
        let toasts = harness::toasts(&lines, &channel);
        assert_eq!(
            toasts.len(),
            1,
            "T-601: post_rcon raised {} toasts for a delivered={delivered} accepted={accepted} \
             body; exactly one is the contract. 0 means the live branch never ran — a \
             dead-code wrapper or a deleted call; this pin executes the source, so unreachable \
             code is invisible to it by design.\n{}",
            toasts.len(),
            lines.join("\n")
        );
        let (severity, text) = toasts[0].split_once('\u{1f}').expect("severity\u{1f}text");
        assert_eq!(
            severity, want,
            "T-601: a delivered={delivered} accepted={accepted} body was toasted as \
             {severity:?}. A 2xx is not a delivery — the `Ok` arm must branch on the body \
             (perturbation: `if true {{ … }}`, or gate on the status instead of \
             rcon_reports_success).\ntoast text was: {text}"
        );
        // The text must be the live formatter's, not a sentence the branch invented.
        let want_text = rcon_accepted_message(&RconAccepted {
            accepted,
            action: harness::ACTION.into(),
            delivered,
            state: harness::STATE.into(),
            detail: harness::DETAIL.into(),
        });
        assert_eq!(
            text, want_text,
            "T-601: the toast did not carry rcon_accepted_message's output, so the operator is \
             reading a claim this page composed rather than what the host reported"
        );
        // The console echo must agree with the toast; a green console line over a failed
        // command is the same lie in a different widget.
        let console = harness::console(&lines, &channel);
        let want_prefix = if want == "success" {
            "RCON: "
        } else {
            "RCON error: "
        };
        assert!(
            console.iter().any(|l| l.starts_with(want_prefix)),
            "T-601: console log for {channel} never carried a {want_prefix:?} line: {console:?}"
        );
        assert_eq!(
            harness::busy(&lines, &channel),
            Some(false),
            "T-601: post_rcon must clear `busy` on every path or the page locks up"
        );
    }

    // A transport failure must not reach the success branch at all.
    let err = harness::toasts(&lines, "transport-error");
    assert_eq!(err.len(), 1, "{}", lines.join("\n"));
    assert!(
        err[0].starts_with(&format!("error{}", '\u{1f}')),
        "T-601: an Err from the client must toast as an error: {}",
        err[0]
    );
    assert_eq!(harness::busy(&lines, "transport-error"), Some(false));
}

/// **Calibration — proof this instrument can still say NO.**
///
/// Takes the same verbatim `post_rcon` and breaks it the ten documented ways plus two the
/// handed-down list does not contain, then asserts the harness reports each break. If this
/// ever passes vacuously, the pin above is decoration.
///
/// Each `dead` wrapper must produce **no toast at all** for the success body — that is the
/// whole point of executing rather than greping: an unreachable call contributes nothing to
/// the recording no matter how it was made unreachable, so the list below is illustrative
/// rather than exhaustive by construction.
#[test]
fn every_dead_code_wrapper_is_visible_to_this_pin() {
    for (label, wrap) in harness::ATTACKS {
        let lines = harness::run_wrapped(label, wrap);
        for channel in ["dtrue-atrue", "dtrue-afalse", "transport-error"] {
            assert!(
                harness::toasts(&lines, channel).is_empty(),
                "T-601: `{label}` still recorded a toast on {channel} — the harness is not \
                 observing execution, and every claim this module makes about dead code is \
                 void.\n{}",
                lines.join("\n")
            );
        }
    }
    // …and the un-wrapped source still records, or the assertions above are vacuous.
    let lines = harness::run();
    assert!(!harness::toasts(&lines, "dtrue-atrue").is_empty());
}

/// The compile-and-run harness for [`post_rcon`].
///
/// Lifts the five items the RCON reply path is made of out of this file **verbatim**, resolves
/// their `target_arch` gates the way the shipped wasm build resolves them, compiles the result
/// against a recording stand-in for the toast host and the API client, and runs it.
///
/// Everything in [`PREAMBLE`] is scaffolding; the only code under test is the text spliced in
/// after it. The edges fail closed, deliberately:
///
/// * No compiler → RED. A pin that cannot examine its input must not pass.
/// * A pinned item that grows a new `crate::…` dependency stops compiling → RED until the
///   preamble is extended. Loud and cheap; the alternative is a pin that quietly stops
///   covering the thing it names.
/// * A `cfg` the wasm build's resolution cannot decide → RED
///   ([`class_r_scrub::resolve_wasm_cfg`]).
mod harness {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// The page's production source, as one text.
    fn src() -> String {
        crate::v2::core::test_support::pins::server_control_source()
    }

    pub(super) const ACTION: &str = "T601-ACTION";
    pub(super) const STATE: &str = "T601-STATE";
    pub(super) const DETAIL: &str = "T601-DETAIL";
    const END: &str = "T601-HARNESS-END";
    /// Field separator in the generated program's stdout — cannot occur in Rust source.
    const US: char = '\u{1f}';

    /// Dead-code wrappers applied to `post_rcon`'s whole body. `@BODY@` is the verbatim body.
    ///
    /// The list is illustrative, not exhaustive, and that is the point of cure 1: an
    /// unreachable call contributes nothing to the recording *however* it was made
    /// unreachable, so a shape nobody has thought of yet fails here too. The ten from the
    /// handed-down battery are listed so a reader can see them go red, plus two the list does
    /// not contain — a constant folded through a comparison, and an `if let` over a pattern
    /// that can never match, which no textual scrubber in this repo folds.
    pub(super) const ATTACKS: &[(&str, &str)] = &[
        ("if true == false", "if true == false { @CALL@ }"),
        ("loop { break; … }", "loop { break; @CALL@ }"),
        ("#[cfg(any())]", "#[cfg(any())] { @CALL@ }"),
        ("while false", "while false { @CALL@ }"),
        ("if !true", "if !true { @CALL@ }"),
        ("if 1 > 2", "if 1 > 2 { @CALL@ }"),
        (
            "if std::hint::black_box(false)",
            "if std::hint::black_box(false) { @CALL@ }",
        ),
        (
            "const C: bool = false; if C",
            "const C: bool = false; if C { @CALL@ }",
        ),
        (
            "return; above",
            "return; #[allow(unreachable_code)] { @CALL@ }",
        ),
        (
            "match () { _ if false => … }",
            "match () { _ if false => { @CALL@ } _ => {} }",
        ),
        // ── two the handed-down list does not contain
        (
            "const NEVER: bool = 1 > 2 (folded through a comparison)",
            "const NEVER: bool = 1 > 2; if NEVER { @CALL@ }",
        ),
        (
            "if let over a pattern that cannot match",
            "if let Some(()) = Option::<()>::None { @CALL@ }",
        ),
    ];

    /// Mock toast host + API client + the crate items the pinned bodies reach for.
    const PREAMBLE: &str = r###"// GENERATED by website-frontend `server_control::t270::harness`.
// The items below the preamble are copied VERBATIM out of `server_control.rs` — do not edit.
#![allow(dead_code, unused_variables, unused_mut, unreachable_code, clippy::all)]

use std::cell::RefCell;

pub type ApiErr = (u16, Option<String>);

/// The 202 body, cut down to the five fields this page reads.
struct RconAccepted {
accepted: bool,
action: String,
delivered: bool,
state: String,
detail: String,
}

/// `serde_json::Value` stands in as an opaque token: `post_rcon` only forwards it.
#[derive(Clone)]
struct Value;

/// Leptos signals are `Copy` handles onto shared state; this is the same shape with none of the
/// reactivity, which the pinned code does not use.
struct RwSignal<T: 'static>(&'static RefCell<T>);
impl<T: 'static> Clone for RwSignal<T> {
fn clone(&self) -> Self {
    RwSignal(self.0)
}
}
impl<T: 'static> Copy for RwSignal<T> {}
impl<T: 'static + Clone> RwSignal<T> {
fn new(v: T) -> Self {
    RwSignal(Box::leak(Box::new(RefCell::new(v))))
}
fn get_untracked(&self) -> T {
    self.0.borrow().clone()
}
fn set(&self, v: T) {
    *self.0.borrow_mut() = v;
}
fn update(&self, f: impl FnOnce(&mut T)) {
    f(&mut self.0.borrow_mut());
}
}

thread_local! {
static SCRIPT: RefCell<Option<Result<RconAccepted, ApiErr>>> = RefCell::new(None);
static CHANNEL: RefCell<String> = RefCell::new(String::new());
}

fn emit(kind: &str, payload: &str) {
CHANNEL.with(|c| println!("{}\u{1f}{}\u{1f}{}", c.borrow(), kind, payload));
}

// T-934.1 moved auth/toast/client under `crate::core::*`; the mocks mirror that path. The pin
// file's own std-machinery paths use `std::` (not bare `core::`) so this local `mod core` cannot
// shadow the builtin crate.
mod core {
pub mod auth {
    #[derive(Clone)]
    pub struct AuthStore;
}

/// Recording stand-in: it reports which toast the pinned source ACTUALLY raised, on the path
/// that actually ran.
pub mod toast {
    #[derive(Clone)]
    pub struct Toasts;
    impl Toasts {
        pub fn success(&self, msg: String) {
            crate::emit("toast", &format!("success\u{1f}{}", msg));
        }
        pub fn error(&self, msg: String) {
            crate::emit("toast", &format!("error\u{1f}{}", msg));
        }
    }
}

pub mod client {
    pub trait Scripted: Sized {
        fn scripted() -> Result<Self, crate::ApiErr>;
    }
    impl Scripted for crate::RconAccepted {
        fn scripted() -> Result<Self, crate::ApiErr> {
            crate::SCRIPT
                .with(|s| s.borrow_mut().take())
                .expect("the harness arms exactly one scripted reply per drive")
        }
    }
    pub async fn api_post<T: Scripted>(
        _store: crate::v2::core::auth::AuthStore,
        _path: &str,
        _body: crate::Value,
    ) -> Result<T, crate::ApiErr> {
        T::scripted()
    }
    pub fn api_error_message(e: &crate::ApiErr, fallback: &str) -> String {
        match &e.1 {
            Some(m) => m.clone(),
            None => fallback.to_string(),
        }
    }
}
}

// The pinned source reaches auth, the HTTP verbs and toasts through the v2 tree; alias them
// onto the mocks above.
mod v2 {
pub mod core {
    pub use crate::core::auth;
    pub mod api {
        pub use crate::core::client;
    }
    pub mod ui {
        pub use crate::core::toast;
    }
}
}

/// The pinned code hands its work to `spawn_local`; nothing it awaits is real I/O, so one poll is
/// the whole future. A `Pending` here means the pinned path grew a real await the harness does not
/// model — RED, never a silent pass.
fn block_on<F: std::future::Future<Output = ()>>(fut: F) {
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
unsafe fn np(_: *const ()) {}
unsafe fn cl(p: *const ()) -> RawWaker {
    RawWaker::new(p, &VT)
}
static VT: RawWakerVTable = RawWakerVTable::new(cl, np, np, np);
let w = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VT)) };
let mut cx = Context::from_waker(&w);
let mut fut = Box::pin(fut);
match fut.as_mut().poll(&mut cx) {
    Poll::Ready(()) => {}
    Poll::Pending => panic!("T601: the pinned RCON path parked on I/O the harness cannot model"),
}
}

mod leptos {
pub mod task {
    pub fn spawn_local<F: std::future::Future<Output = ()> + 'static>(f: F) {
        crate::block_on(f)
    }
}
}

fn drive(channel: &str, script: Result<RconAccepted, ApiErr>) {
CHANNEL.with(|c| *c.borrow_mut() = channel.to_string());
SCRIPT.with(|s| *s.borrow_mut() = Some(script));
let console: RwSignal<Vec<String>> = RwSignal::new(Vec::new());
let busy: RwSignal<bool> = RwSignal::new(false);
post_rcon(
    crate::v2::core::auth::AuthStore,
    "T601-SERVER".to_string(),
    Value,
    "T601-ECHO".to_string(),
    console,
    busy,
    crate::v2::core::ui::toast::Toasts,
);
for line in console.get_untracked() {
    emit("console", &line);
}
emit("busy", if busy.get_untracked() { "true" } else { "false" });
}

fn reply(delivered: bool, accepted: bool) -> RconAccepted {
RconAccepted {
    accepted,
    action: "@ACTION@".to_string(),
    delivered,
    state: "@STATE@".to_string(),
    detail: "@DETAIL@".to_string(),
}
}

"###;

    fn item(sig: &str) -> String {
        let prod = crate::v2::core::test_support::class_r_scrub::live_source(&src());
        let raw = crate::v2::core::test_support::class_r_scrub::only_item(&prod, sig);
        crate::v2::core::test_support::class_r_scrub::resolve_wasm_cfg(raw)
    }

    /// The verbatim `post_rcon`, renamed, plus the `post_rcon` the driver actually calls: a
    /// shim whose only statement is one call to the real thing, optionally parked inside
    /// `wrapper` (`@CALL@` = that call).
    ///
    /// The attack is applied to the **call site**, which is what the decoys this battery was
    /// written against did, and it keeps the borrow checker out of the way. The
    /// loop-shaped wrappers (`loop { break; … }`, `while false { … }`) make rustc analyse the
    /// body as if it could run twice, and `post_rcon` moves five of its seven parameters; the
    /// closure clones them per call, so borrowck has nothing to say and the only variable
    /// under test stays *reachability*.
    const SHIM: &str = r###"
fn post_rcon(
store: crate::v2::core::auth::AuthStore,
server_id: String,
body: Value,
echo: String,
console_log: RwSignal<Vec<String>>,
busy: RwSignal<bool>,
toasts: crate::v2::core::ui::toast::Toasts,
) {
let call = move || {
    post_rcon_live(
        store.clone(),
        server_id.clone(),
        body.clone(),
        echo.clone(),
        console_log,
        busy,
        toasts.clone(),
    )
};
@WRAPPED@
}
"###;

    fn program(wrapper: Option<&str>) -> String {
        let mut src = PREAMBLE
            .replace("@ACTION@", ACTION)
            .replace("@STATE@", STATE)
            .replace("@DETAIL@", DETAIL);
        for sig in [
            "fn admin_server_rcon_path(",
            "fn rcon_reports_success(",
            "fn rcon_accepted_message(",
            "fn append_log(",
        ] {
            src.push_str(&item(sig));
            src.push_str("\n\n");
        }
        const POST_SIG: &str = "fn post_rcon(";
        let post = item(POST_SIG);
        assert!(
            post.starts_with(POST_SIG),
            "T-601: post_rcon extraction misaligned"
        );
        src.push_str(&format!("fn post_rcon_live({}", &post[POST_SIG.len()..]));
        src.push_str("\n\n");
        src.push_str(&SHIM.replace(
            "@WRAPPED@",
            &wrapper.unwrap_or("@CALL@").replace("@CALL@", "call();"),
        ));
        src.push_str("\n\nfn main() {\n");
        for (d, a) in [(true, true), (true, false), (false, false), (false, true)] {
            src.push_str(&format!("    drive(\"d{d}-a{a}\", Ok(reply({d}, {a})));\n"));
        }
        src.push_str(
            "    drive(\"transport-error\", Err((503u16, Some(\"host agent unreachable\".to_string()))));\n",
        );
        src.push_str(&format!("    println!(\"{END}\");\n}}\n"));
        src
    }

    /// The compiler that built this test. Cargo exports `CARGO` to every crate it compiles and
    /// `rustc` is its sibling in the same toolchain; PATH is the fallback. There is
    /// deliberately no "skip if absent" branch — a pin that cannot examine its input goes RED.
    fn rustc_bin() -> PathBuf {
        if let Some(cargo) = option_env!("CARGO") {
            let sibling = Path::new(cargo).with_file_name("rustc");
            if sibling.is_file() {
                return sibling;
            }
        }
        PathBuf::from("rustc")
    }

    fn compile_and_run(tag: &str, source: &str) -> Vec<String> {
        let dir = std::env::current_exe()
            .expect("T-601: test executable path")
            .with_file_name(format!("t601-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("T-601: cannot create {}: {e}", dir.display()));
        let rs = dir.join("t601_pin.rs");
        std::fs::write(&rs, source)
            .unwrap_or_else(|e| panic!("T-601: cannot write {}: {e}", rs.display()));
        let bin = dir.join("t601_pin");

        let rustc = rustc_bin();
        let compiled = Command::new(&rustc)
            .args(["--edition", "2021", "--crate-name", "t601_pin"])
            .args(["-C", "debug-assertions=on", "-A", "warnings", "-o"])
            .arg(&bin)
            .arg(&rs)
            .output()
            .unwrap_or_else(|e| {
                panic!(
                    "T-601: cannot run `{}`: {e}\n\
                     This pin proves the RCON reply path by compiling and running it, so a \
                     missing compiler is a failure to verify, not a pass.",
                    rustc.display()
                )
            });
        assert!(
            compiled.status.success(),
            "T-601: the extracted server_control items no longer compile against the pin's \
             mocks. Usually this means a pinned item grew a dependency the preamble does not \
             model — extend PREAMBLE. Source kept at {}\n\n{}",
            rs.display(),
            String::from_utf8_lossy(&compiled.stderr)
        );

        let ran = Command::new(&bin)
            .output()
            .unwrap_or_else(|e| panic!("T-601: cannot run {}: {e}", bin.display()));
        assert!(
            ran.status.success(),
            "T-601: the extracted RCON path aborted. Source kept at {}\n\nstdout:\n{}\nstderr:\n{}",
            rs.display(),
            String::from_utf8_lossy(&ran.stdout),
            String::from_utf8_lossy(&ran.stderr)
        );
        let stdout = String::from_utf8_lossy(&ran.stdout).into_owned();
        assert_eq!(
            stdout.lines().last(),
            Some(END),
            "T-601: the harness did not run to completion; stdout was:\n{stdout}"
        );
        let _ = std::fs::remove_dir_all(&dir);
        stdout.lines().map(str::to_string).collect()
    }

    pub(super) fn run() -> Vec<String> {
        compile_and_run("live", &program(None))
    }

    pub(super) fn run_wrapped(label: &str, wrapper: &str) -> Vec<String> {
        let tag: String = label
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        compile_and_run(&tag, &program(Some(wrapper)))
    }

    fn rows(lines: &[String], channel: &str, kind: &str) -> Vec<String> {
        let prefix = format!("{channel}{US}{kind}{US}");
        lines
            .iter()
            .filter_map(|l| l.strip_prefix(&prefix).map(str::to_string))
            .collect()
    }

    /// Recorded toasts for one channel, as `severity\u{1f}text`.
    pub(super) fn toasts(lines: &[String], channel: &str) -> Vec<String> {
        rows(lines, channel, "toast")
    }

    pub(super) fn console(lines: &[String], channel: &str) -> Vec<String> {
        rows(lines, channel, "console")
    }

    pub(super) fn busy(lines: &[String], channel: &str) -> Option<bool> {
        rows(lines, channel, "busy").first().map(|v| v == "true")
    }
}

#[test]
fn pick_default_prefers_active() {
    let inactive = ServerRowDto {
        id: "a".into(),
        name: "A".into(),
        ip: "1.1.1.1".into(),
        port: 1,
        required_modpack_id: None,
        is_active: false,
        status: None,
        required_modpack: None,
        terrain: None,
    };
    let mut active = inactive.clone();
    active.id = "b".into();
    active.is_active = true;
    assert_eq!(
        pick_default_id(&[inactive.clone(), active.clone()]).as_deref(),
        Some("b")
    );
    assert_eq!(pick_default_id(&[inactive]).as_deref(), Some("a"));
    assert_eq!(pick_default_id(&[]), None);
}
