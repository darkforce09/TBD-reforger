//! The gates re-run their children only when the admission they render from changes.
//!
//! A view cannot be mounted in a native test, so the admission memos are exercised through a
//! subscriber that reads them the way a gate's render closure does, and the gate bodies are pinned
//! to render from those memos and from nothing else.

#![cfg(not(target_arch = "wasm32"))]

use super::*;
use crate::v2::core::auth::{RefreshResponse, Session, User};
use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const MEMBER_ID: &str = "100200300";

fn member(role: Role) -> User {
    User {
        discord_id: MEMBER_ID.into(),
        username: "member".into(),
        discord_handle: "member".into(),
        avatar_url: String::new(),
        arma_id: None,
        arma_character: String::new(),
        role,
        is_banned: false,
        ban_reason: String::new(),
        banned_by: None,
        banned_at: None,
        total_deployments: 0,
        attendance_rate: 0.0,
        last_login_at: None,
        created_at: "2026-01-01T00:00:00Z".into(),
        updated_at: "2026-01-01T00:00:00Z".into(),
    }
}

/// An access token for session `session`; `issued` makes each rotation a distinct value.
fn access_token(session: u32, issued: u32) -> String {
    use base64::Engine;
    let claims = serde_json::json!({
        "sub": MEMBER_ID,
        "sid": format!("00000000-0000-0000-0000-{session:012x}"),
        "iat": issued,
    });
    format!(
        "header.{}.signature",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims.to_string())
    )
}

fn sign_in(store: AuthStore, role: Role) {
    store.set_session(Session {
        access_token: access_token(1, 0),
        refresh_token: "refresh".into(),
        expires_at: "2026-01-02T00:00:00Z".into(),
        user: member(role),
        arma_linked: false,
    });
}

/// Rotate the token pair within the current session.
fn rotate_tokens(store: AuthStore, issued: u32) {
    store.set_tokens(RefreshResponse {
        access_token: access_token(1, issued),
        refresh_token: format!("refresh-{issued}"),
        expires_at: "2026-01-03T00:00:00Z".into(),
    });
}

/// Change one profile field that no gate decides on.
fn touch_last_login(store: AuthStore, at: &str) {
    store.user.update(|user| {
        if let Some(user) = user.as_mut() {
            user.last_login_at = Some(at.into());
        }
    });
}

fn set_role(store: AuthStore, role: Role) {
    store.user.update(|user| {
        if let Some(user) = user.as_mut() {
            user.role = role;
        }
    });
}

fn with_store(test: impl FnOnce(AuthStore)) {
    let owner = Owner::new();
    owner.with(|| test(AuthStore::new()));
}

/// A stand-in for a gate's render closure: it reads the memoized admission and nothing else, and
/// builds the children on every run that admits.
///
/// It is a memo settled on demand rather than an effect, because that is how a render effect
/// decides to re-run: the writes land first, then it checks its sources once and runs only when
/// one of them changed. An `ImmediateEffect` cannot stand in here: it re-enters the admission memo
/// while that memo is still notifying, and the memo's own lock deadlocks the re-entry.
struct GateProbe {
    runs: Arc<AtomicUsize>,
    children_built: Arc<AtomicUsize>,
    render: Memo<()>,
}

impl GateProbe {
    fn new(admits: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        let runs = Arc::new(AtomicUsize::new(0));
        let children_built = Arc::new(AtomicUsize::new(0));
        let render = Memo::new({
            let runs = Arc::clone(&runs);
            let children_built = Arc::clone(&children_built);
            move |_| {
                runs.fetch_add(1, Ordering::SeqCst);
                if admits() {
                    children_built.fetch_add(1, Ordering::SeqCst);
                }
            }
        });
        Self {
            runs,
            children_built,
            render,
        }
    }

    /// Settle the stand-in render, then report `(render runs, children builds)` so far, the
    /// initial render included.
    fn counts(&self) -> (usize, usize) {
        self.render.with_untracked(|_| ());
        (
            self.runs.load(Ordering::SeqCst),
            self.children_built.load(Ordering::SeqCst),
        )
    }
}

#[test]
fn the_sign_in_gate_keeps_its_children_through_writes_that_keep_the_viewer_signed_in() {
    with_store(|store| {
        sign_in(store, Role::Enlisted);
        let admission = session_admission(store);
        let gate = GateProbe::new(move || admission.get() == SessionAdmission::Admitted);
        assert_eq!(gate.counts(), (1, 1));

        // A raw signal write notifies even when the value is equal, so the gate holds whatever
        // the store writes.
        store.user.set(Some(member(Role::Enlisted)));
        touch_last_login(store, "2026-01-01T12:00:00Z");
        rotate_tokens(store, 1);
        assert_eq!(
            gate.counts(),
            (1, 1),
            "a profile or token write that keeps the viewer signed in must not re-run the gate"
        );

        store.clear_session();
        assert_eq!(admission.get_untracked(), SessionAdmission::SignedOut);
        assert_eq!(gate.counts(), (2, 1), "signing out swaps the children out");

        sign_in(store, Role::Enlisted);
        assert_eq!(
            gate.counts(),
            (3, 2),
            "signing back in builds the children again"
        );
    });
}

#[test]
fn the_sign_in_gate_waits_out_the_session_restore() {
    with_store(|store| {
        store.bootstrapping.set(true);
        let admission = session_admission(store);
        assert_eq!(admission.get_untracked(), SessionAdmission::Restoring);

        sign_in(store, Role::Enlisted);
        assert_eq!(admission.get_untracked(), SessionAdmission::Admitted);
    });
}

#[test]
fn the_admin_gate_rebuilds_its_children_only_when_the_role_crosses_the_tier() {
    with_store(|store| {
        sign_in(store, Role::Enlisted);
        let admitted = admin_admission(store);
        let gate = GateProbe::new(move || admitted.get());
        assert_eq!(gate.counts(), (1, 0));

        store.user.set(Some(member(Role::Enlisted)));
        assert_eq!(
            gate.counts(),
            (1, 0),
            "an equal profile must not re-run the gate"
        );

        set_role(store, Role::Admin);
        assert_eq!(
            gate.counts(),
            (2, 1),
            "crossing into the administrator tier admits the viewer"
        );

        store.user.set(Some(member(Role::Admin)));
        touch_last_login(store, "2026-01-01T12:00:00Z");
        rotate_tokens(store, 1);
        assert_eq!(
            gate.counts(),
            (2, 1),
            "a write that keeps the viewer an administrator must not re-mount the page"
        );

        set_role(store, Role::MissionMaker);
        assert_eq!(
            gate.counts(),
            (3, 1),
            "dropping below the tier refuses the viewer"
        );
    });
}

/// The gate components render from their admission memos and read no session signal directly.
///
/// The memo tests above are half the contract: a gate body that read the session itself would
/// re-run its children on every profile poll whatever the memos do. Scrubbed, so a comment or a
/// string literal cannot satisfy a needle.
#[test]
fn both_gates_render_from_their_admission_memo_alone() {
    let code = live_code(&crate::v2::core::test_support::pins::ui_source());
    let flat = |body: &str| body.split_whitespace().collect::<Vec<_>>().join(" ");

    let auth_gate = flat(only_body(&code, "pub fn AuthGate("));
    for needle in [
        "let admission = session_admission(expect_context::<AuthStore>());",
        "move || match admission.get() {",
        "SessionAdmission::Admitted => children().into_any()",
    ] {
        assert!(
            auth_gate.contains(needle),
            "AuthGate must render from its admission memo (`{needle}`); body was: {auth_gate}"
        );
    }
    assert_eq!(
        auth_gate.matches("children()").count(),
        1,
        "AuthGate must build its children in the admitted arm only; body was: {auth_gate}"
    );

    let admin_gate = flat(only_body(&code, "pub fn AdminGate("));
    for needle in [
        "let admitted = admin_admission(expect_context::<AuthStore>());",
        "<Show when=move || admitted.get() fallback=admin_access_required> {children()} </Show>",
    ] {
        assert!(
            admin_gate.contains(needle),
            "AdminGate must render from its admission memo (`{needle}`); body was: {admin_gate}"
        );
    }

    for (gate, body) in [("AuthGate", &auth_gate), ("AdminGate", &admin_gate)] {
        for direct_read in [
            ".user",
            "bootstrapping",
            "access_token",
            "is_authenticated",
            "has_min_role",
        ] {
            assert!(
                !body.contains(direct_read),
                "{gate} must read the session only through its admission memo; found \
                 `{direct_read}` in: {body}"
            );
        }
    }
}
