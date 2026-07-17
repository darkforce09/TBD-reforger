//! Auth — session state, the single-use refresh-token coordination, and the `tbd-auth` persist
//! shape. Ports api/refresh.ts + store/useAuthStore.ts.
//!
//! Refresh tokens are single-use: the server rotates + revokes the old token on every
//! `/auth/refresh`. Several callers can want a refresh at once (bootstrap, the 401 retry, several
//! requests 401-ing together). Without coordination they'd each present the same token and all but
//! the first would 401 — wrongly clearing the session. [`SingleFlight`] guarantees the token is
//! spent at most once at a time. The gloo-net POST + 401-retry client wire on top (wasm) in the
//! client slice; the coordination + persist serde here are pure and unit-tested natively.

use crate::nav::{has_min_role, Role};
use futures::future::{FutureExt, LocalBoxFuture, Shared};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

/* ─────────────────────────── single-flight refresh ─────────────────────────── */

/// At most one in-flight future for `T` at a time. Concurrent callers of [`run`](Self::run) await a
/// clone of the same shared future; the owner clears the cell once it settles.
// Wired to the gloo-net refresh + 401-retry client in the next auth slice; unit-tested now.
// Rc-backed so it can live in a `thread_local!` and be cloned out to share across the async refresh
// (the wasm client can't hold a thread-local borrow across an await).
#[allow(dead_code)]
#[derive(Clone)]
pub struct SingleFlight<T: Clone> {
    inflight: Rc<RefCell<Option<Shared<LocalBoxFuture<'static, T>>>>>,
}

#[allow(dead_code)]
impl<T: Clone + 'static> SingleFlight<T> {
    pub fn new() -> Self {
        Self {
            inflight: Rc::new(RefCell::new(None)),
        }
    }

    /// Run `make` under single-flight. The first caller builds the future and stores it; concurrent
    /// callers clone the in-flight handle instead of calling `make` again. Cleared on settle,
    /// mirroring `.finally(() => (inflight = null))` in refresh.ts.
    pub async fn run<F, Fut>(&self, make: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T> + 'static,
    {
        let (shared, owner) = {
            let mut slot = self.inflight.borrow_mut();
            match slot.as_ref() {
                Some(s) => (s.clone(), false),
                None => {
                    let s = make().boxed_local().shared();
                    *slot = Some(s.clone());
                    (s, true)
                }
            }
        };
        let out = shared.await;
        if owner {
            *self.inflight.borrow_mut() = None;
        }
        out
    }
}

#[allow(dead_code)]
impl<T: Clone + 'static> Default for SingleFlight<T> {
    fn default() -> Self {
        Self::new()
    }
}

/* ─────────────────────────── session types (models.User / RefreshToken) ─────────────────────────── */

/// Authenticated user identity — models.User (backend `apps/website/src/models/user.rs`). snake_case
/// = the API contract. Field set + serde attrs mirror the backend byte-for-byte so a golden `/me`
/// (and the persisted user in the tbd-auth blob) round-trips exactly (R-api gate): the three ban
/// fields + `last_login_at` are omitted when empty/absent (backend `skip_serializing_if`), and
/// `arma_id` stays present-as-null. Dates are opaque RFC3339 strings here (no chrono on the UI side)
/// — carried verbatim, so re-serialize is byte-identical.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub discord_id: String,
    pub username: String,
    pub discord_handle: String,
    pub avatar_url: String,
    /// `null` when no Arma identity is linked (kept as null in the persisted blob, not omitted).
    #[serde(default)]
    pub arma_id: Option<String>,
    pub arma_character: String,
    pub role: Role,
    pub is_banned: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ban_reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banned_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banned_at: Option<String>,
    pub total_deployments: i64,
    pub attendance_rate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_login_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// A minted session (access + rotating refresh pair + profile). Mirrors `AuthSession`.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: String,
    pub user: User,
    pub arma_linked: bool,
}

/// The rotated pair returned by `POST /auth/refresh`. Mirrors `RefreshResponse`.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: String,
}

/* ─────────────────────────── tbd-auth persistence ─────────────────────────── */

/// localStorage key — identical to the Zustand persist `name`.
// The persist API is exercised by the bootstrap + client slice next; unit-tested now.
#[allow(dead_code)]
pub const AUTH_PERSIST_KEY: &str = "tbd-auth";

/// The persisted slice. Zustand `partialize` keeps refreshToken/user/expiresAt and — critically —
/// NOT accessToken. The persist keys are the JS store's camelCase; `user` stays snake_case (the
/// API DTO). This is the exact shape the R-auth-persist gate diffs.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistState {
    pub refresh_token: Option<String>,
    pub user: Option<User>,
    pub expires_at: Option<String>,
}

/// The full `localStorage["tbd-auth"]` blob: Zustand persist wraps state as `{state, version}`.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedAuth {
    pub state: PersistState,
    pub version: u32,
}

/// Serialize the persist slice to the exact tbd-auth blob string (pure; the wasm side just writes
/// this to localStorage). Version 0 = the Zustand default (no `version` option set on the store).
#[allow(dead_code)]
pub fn to_persist_json(state: &PersistState) -> String {
    serde_json::to_string(&PersistedAuth {
        state: state.clone(),
        version: 0,
    })
    .unwrap_or_default()
}

/// Parse a tbd-auth blob back to the persist slice (bootstrap / cold-load hydrate).
#[allow(dead_code)]
pub fn from_persist_json(json: &str) -> Option<PersistState> {
    serde_json::from_str::<PersistedAuth>(json)
        .ok()
        .map(|p| p.state)
}

/// Write the persist slice to `localStorage["tbd-auth"]` (the wasm side of the Zustand persist).
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub fn persist(state: &PersistState) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(AUTH_PERSIST_KEY, &to_persist_json(state));
    }
}

/// Read the persist slice back from localStorage (cold-load bootstrap).
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub fn load_persisted() -> Option<PersistState> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let json = storage.get_item(AUTH_PERSIST_KEY).ok()??;
    from_persist_json(&json)
}

/* ─────────────────────────── AuthStore (signals + context) ─────────────────────────── */

/// Global auth/session state — signals + context, replacing the Zustand `useAuthStore`. Provided at
/// the app root; components read it via `expect_context::<AuthStore>()`. `RwSignal` is `Copy`, so
/// the store is `Copy` and threads through `view!` without cloning.
// Provided at AppLayout + read by TopNav/SidebarNav in the next auth slice (with bootstrap + the
// gloo-net client); defined + shape-tested now.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct AuthStore {
    pub access_token: RwSignal<Option<String>>,
    pub refresh_token: RwSignal<Option<String>>,
    pub expires_at: RwSignal<Option<String>>,
    pub user: RwSignal<Option<User>>,
    pub bootstrapping: RwSignal<bool>,
}

#[allow(dead_code)]
impl AuthStore {
    pub fn new() -> Self {
        Self {
            access_token: RwSignal::new(None),
            refresh_token: RwSignal::new(None),
            expires_at: RwSignal::new(None),
            user: RwSignal::new(None),
            bootstrapping: RwSignal::new(false),
        }
    }

    pub fn set_session(&self, s: Session) {
        self.access_token.set(Some(s.access_token));
        self.refresh_token.set(Some(s.refresh_token));
        self.expires_at.set(Some(s.expires_at));
        self.user.set(Some(s.user));
        self.bootstrapping.set(false);
    }

    /// Persist a rotated pair without touching `user`. Refresh tokens are single-use — after any
    /// successful rotation the new refresh_token MUST be stored even when no user is loaded yet, or
    /// the session dies at the next refresh (T-126 S5/S6).
    pub fn set_tokens(&self, t: RefreshResponse) {
        self.access_token.set(Some(t.access_token));
        self.refresh_token.set(Some(t.refresh_token));
        self.expires_at.set(Some(t.expires_at));
    }

    pub fn clear_session(&self) {
        self.access_token.set(None);
        self.refresh_token.set(None);
        self.expires_at.set(None);
        self.user.set(None);
        self.bootstrapping.set(false);
    }

    pub fn is_authenticated(&self) -> bool {
        self.access_token.get().is_some() && self.user.get().is_some()
    }

    pub fn has_min_role(&self, min: Role) -> bool {
        has_min_role(self.user.get().map(|u| u.role), min)
    }

    /// The subset persisted to localStorage (refreshToken/user/expiresAt — never accessToken).
    pub fn persist_state(&self) -> PersistState {
        PersistState {
            refresh_token: self.refresh_token.get_untracked(),
            user: self.user.get_untracked(),
            expires_at: self.expires_at.get_untracked(),
        }
    }
}

#[allow(dead_code)]
impl Default for AuthStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use futures::future::join_all;
    use std::cell::Cell;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::task::{Context, Poll};

    /// Pends exactly once so several tasks all register on the shared future before it resolves —
    /// the overlap a real 401 storm creates.
    struct YieldOnce(bool);
    impl Future for YieldOnce {
        type Output = ();
        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if self.0 {
                Poll::Ready(())
            } else {
                self.0 = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    // The load-bearing auth proof: N concurrent callers spend the single-use token exactly once.
    #[test]
    fn concurrent_callers_spend_the_token_once() {
        let calls = Rc::new(Cell::new(0));
        let sf = SingleFlight::<i32>::new();
        let mk = || {
            let calls = calls.clone();
            move || {
                calls.set(calls.get() + 1); // one increment == one refresh POST
                async {
                    YieldOnce(false).await;
                    42
                }
            }
        };
        let out = block_on(join_all(vec![
            sf.run(mk()),
            sf.run(mk()),
            sf.run(mk()),
            sf.run(mk()),
        ]));
        assert_eq!(
            calls.get(),
            1,
            "four concurrent callers must trigger exactly one refresh"
        );
        assert_eq!(
            out,
            vec![42, 42, 42, 42],
            "all callers receive the same rotated result"
        );
    }

    // After a refresh settles the cell clears, so a later (non-overlapping) call refreshes again.
    #[test]
    fn cell_clears_after_settle() {
        let calls = Rc::new(Cell::new(0));
        let sf = SingleFlight::<i32>::new();
        let mk = || {
            let calls = calls.clone();
            move || {
                calls.set(calls.get() + 1);
                async { 7 }
            }
        };
        block_on(sf.run(mk()));
        block_on(sf.run(mk()));
        assert_eq!(
            calls.get(),
            2,
            "sequential non-overlapping calls each refresh"
        );
    }

    fn sample_user() -> User {
        User {
            discord_id: "123".into(),
            username: "cpl".into(),
            discord_handle: "cpl#0001".into(),
            avatar_url: "https://cdn/a.png".into(),
            arma_id: None,
            arma_character: String::new(),
            role: Role::Admin,
            is_banned: false,
            ban_reason: String::new(),
            banned_by: None,
            banned_at: None,
            total_deployments: 5,
            attendance_rate: 0.9,
            last_login_at: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-02T00:00:00Z".into(),
        }
    }

    // R-auth-persist SHAPE: the tbd-auth blob matches the Zustand persist contract exactly.
    #[test]
    fn persist_blob_shape_matches_tbd_auth() {
        let state = PersistState {
            refresh_token: Some("rt-abc".into()),
            user: Some(sample_user()),
            expires_at: Some("2026-01-01T01:00:00Z".into()),
        };
        let json = to_persist_json(&state);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(v["version"], 0);
        // Persist keys are camelCase…
        assert_eq!(v["state"]["refreshToken"], "rt-abc");
        assert_eq!(v["state"]["expiresAt"], "2026-01-01T01:00:00Z");
        // …and the access token is NEVER persisted (T-126 S5 — the whole point of partialize).
        assert!(v["state"].get("accessToken").is_none());
        // …while the user object keeps the snake_case API contract + string role + null arma_id.
        assert_eq!(v["state"]["user"]["discord_id"], "123");
        assert_eq!(v["state"]["user"]["role"], "admin");
        assert_eq!(v["state"]["user"]["arma_id"], serde_json::Value::Null);
    }

    #[test]
    fn persist_round_trips() {
        let state = PersistState {
            refresh_token: Some("rt".into()),
            user: Some(sample_user()),
            expires_at: Some("2026".into()),
        };
        let back = from_persist_json(&to_persist_json(&state)).unwrap();
        assert!(
            back == state,
            "persist → hydrate must round-trip losslessly"
        );
    }
}
