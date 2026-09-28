//! Generated rotate, replay and logout sequences over the refresh-token families of one
//! account, and the sequential model their observed outcomes must fit.
//!
//! **Role:** Declares the generated steps (an operation alone, or two operations released
//! together) and the model of each family's tokens, access credentials and revocation.
//!
//! **Position:** Test support for `tests/session_authority_properties.rs`, which presents each
//! step to the production `rotate_session` and `logout_session` and narrows the model states
//! with every observed outcome and with [`super::persisted_families::PersistedFamilies`].
//!
//! **Signals & state:** none; values and pure transitions.
//!
//! **Invariants:** the model rules:
//! - rotating a live family's current token succeeds, spends that token and makes the
//!   returned token current;
//! - presenting a spent token of a live family is a replay: it answers 401 and revokes every
//!   family of the account, successors issued concurrently included;
//! - any rotation in a revoked family answers 401 and changes nothing;
//! - logout with any token of a family revokes that family alone;
//! - a concurrent pair is explained by at least one of its two serialisations.

use axum::http::StatusCode;
use proptest::{collection::vec, prelude::*};
use std::collections::BTreeSet;
use uuid::Uuid;
use website_api::core::authentication_primitives::hash_token;

/// Refresh-token families (sessions) every generated account holds.
pub const FAMILY_COUNT: usize = 2;
/// Exclusive upper bound of the steps in one generated sequence.
const MAX_STEPS: usize = 8;

/// Which token of its family an operation presents.
#[derive(Debug, Clone, Copy)]
pub enum Credential {
    /// The family's newest token.
    Current,
    /// A token the family already spent, by index modulo the spent count; the current token
    /// while the family has spent none.
    Spent(usize),
}

/// The session service an operation calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Rotate,
    Logout,
}

/// One generated operation against one family.
#[derive(Debug, Clone, Copy)]
pub struct FamilyOperation {
    pub kind: OperationKind,
    pub family: usize,
    pub credential: Credential,
}

/// One generated step.
#[derive(Debug, Clone, Copy)]
pub enum RefreshStep {
    Alone(FamilyOperation),
    /// Two operations released together behind a barrier.
    ConcurrentPair(FamilyOperation, FamilyOperation),
}

/// An operation bound to the concrete refresh token it presents.
#[derive(Debug, Clone)]
pub struct PresentedOperation {
    pub kind: OperationKind,
    pub family: usize,
    pub token: String,
}

/// What a session service answered.
#[derive(Debug, Clone)]
pub enum ObservedOutcome {
    Rotated {
        access_credential: String,
        refresh_token: String,
    },
    LoggedOut,
    Refused(StatusCode),
}

/// The model of one refresh-token family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyModel {
    pub session_id: Uuid,
    pub current: String,
    pub spent: Vec<String>,
    /// Every access credential issued in the family, the sign-in one first.
    pub access_credentials: Vec<String>,
    pub revoked: bool,
}

impl FamilyModel {
    /// A family fresh from sign-in.
    pub fn issued(session_id: Uuid, access_credential: String, refresh_token: String) -> Self {
        Self {
            session_id,
            current: refresh_token,
            spent: Vec::new(),
            access_credentials: vec![access_credential],
            revoked: false,
        }
    }
}

/// The model of every family of one account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountModel {
    pub families: Vec<FamilyModel>,
}

impl AccountModel {
    /// Bind `operation` to the concrete token this state names.
    pub fn present(&self, operation: FamilyOperation) -> PresentedOperation {
        let family = &self.families[operation.family];
        let token = match operation.credential {
            Credential::Spent(index) if !family.spent.is_empty() => {
                family.spent[index % family.spent.len()].clone()
            }
            _ => family.current.clone(),
        };
        PresentedOperation {
            kind: operation.kind,
            family: operation.family,
            token,
        }
    }

    /// The state after `operation` answered `observed`; `None` when the rules forbid that
    /// answer from this state.
    fn after(&self, operation: &PresentedOperation, observed: &ObservedOutcome) -> Option<Self> {
        let family = &self.families[operation.family];
        let spent = family.spent.contains(&operation.token);
        if !spent && family.current != operation.token {
            return None;
        }
        let mut next = self.clone();
        match (operation.kind, observed) {
            (OperationKind::Logout, ObservedOutcome::LoggedOut) => {
                next.families[operation.family].revoked = true;
            }
            (OperationKind::Rotate, ObservedOutcome::Refused(status))
                if *status == StatusCode::UNAUTHORIZED && family.revoked => {}
            (OperationKind::Rotate, ObservedOutcome::Refused(status))
                if *status == StatusCode::UNAUTHORIZED && spent =>
            {
                for family in &mut next.families {
                    family.revoked = true;
                }
            }
            (
                OperationKind::Rotate,
                ObservedOutcome::Rotated {
                    access_credential,
                    refresh_token,
                },
            ) if !family.revoked && !spent => {
                let family = &mut next.families[operation.family];
                let consumed = std::mem::replace(&mut family.current, refresh_token.clone());
                family.spent.push(consumed);
                family.access_credentials.push(access_credential.clone());
            }
            _ => return None,
        }
        Some(next)
    }

    /// Every state `candidates` lead to when `operation` answered `observed`.
    pub fn after_alone(
        candidates: &[Self],
        operation: &PresentedOperation,
        observed: &ObservedOutcome,
    ) -> Vec<Self> {
        let mut explained = Vec::new();
        for next in candidates
            .iter()
            .filter_map(|model| model.after(operation, observed))
        {
            if !explained.contains(&next) {
                explained.push(next);
            }
        }
        explained
    }

    /// Every state `candidates` lead to under either serialisation of a concurrent pair.
    pub fn after_either_order(
        candidates: &[Self],
        first: (&PresentedOperation, &ObservedOutcome),
        second: (&PresentedOperation, &ObservedOutcome),
    ) -> Vec<Self> {
        let mut explained = Vec::new();
        for model in candidates {
            for (earlier, later) in [(first, second), (second, first)] {
                if let Some(next) = model
                    .after(earlier.0, earlier.1)
                    .and_then(|middle| middle.after(later.0, later.1))
                    && !explained.contains(&next)
                {
                    explained.push(next);
                }
            }
        }
        explained
    }

    /// The sessions of the families this state keeps live.
    pub fn live_sessions(&self) -> BTreeSet<Uuid> {
        self.families
            .iter()
            .filter(|family| !family.revoked)
            .map(|family| family.session_id)
            .collect()
    }

    /// The stored hashes of the one live token of every live family.
    pub fn live_token_hashes(&self) -> BTreeSet<String> {
        self.families
            .iter()
            .filter(|family| !family.revoked)
            .map(|family| hash_token(&family.current))
            .collect()
    }
}

fn family_operation() -> impl Strategy<Value = FamilyOperation> {
    let kind = prop_oneof![
        4 => Just(OperationKind::Rotate),
        1 => Just(OperationKind::Logout),
    ];
    let credential = prop_oneof![
        3 => Just(Credential::Current),
        2 => (0..MAX_STEPS).prop_map(Credential::Spent),
    ];
    (kind, 0..FAMILY_COUNT, credential).prop_map(|(kind, family, credential)| FamilyOperation {
        kind,
        family,
        credential,
    })
}

/// Every generated step sequence.
pub fn refresh_steps() -> impl Strategy<Value = Vec<RefreshStep>> {
    let step = prop_oneof![
        3 => family_operation().prop_map(RefreshStep::Alone),
        2 => (family_operation(), family_operation())
            .prop_map(|(first, second)| RefreshStep::ConcurrentPair(first, second)),
    ];
    vec(step, 1..MAX_STEPS)
}
