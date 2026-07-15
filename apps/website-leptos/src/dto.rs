//! API response DTOs (snake_case = the API contract, ported from types/api). The generic list
//! envelope + the first endpoints the client/pages need; serde round-trip tested. More DTOs land
//! per page (T-159.8+). Byte-exact parity vs the live backend is the R-api gate (fixtures); these
//! tests prove the struct shapes are self-consistent and match the TS contract's field names.
use crate::auth::User;
use serde::{Deserialize, Serialize};

/// List endpoints return `{data, total, limit, offset}` (CLAUDE.md; audit logs use a cursor).
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// `GET /me` → the authed user + Arma link flag.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct MeResponse {
    pub user: User,
    pub arma_linked: bool,
}

/// `GET /me/link/status` → the caller's Arma identity link state. The optionals are omitted by the
/// backend (Go `omitempty`) when empty, so they round-trip absent.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkStatus {
    pub linked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arma_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arma_character: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_code: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paginated_shape() {
        let p: Paginated<i64> =
            serde_json::from_str(r#"{"data":[1,2,3],"total":3,"limit":20,"offset":0}"#).unwrap();
        assert_eq!(p.data, vec![1, 2, 3]);
        assert_eq!((p.total, p.limit, p.offset), (3, 20, 0));
    }

    #[test]
    fn link_status_optionals() {
        let full: LinkStatus = serde_json::from_str(
            r#"{"linked":true,"arma_id":"a","arma_character":"Cpl","pending_code":true}"#,
        )
        .unwrap();
        assert!(
            full.linked && full.pending_code == Some(true) && full.arma_id.as_deref() == Some("a")
        );
        // The minimal shape (backend omitempty drops the empties)…
        let min: LinkStatus = serde_json::from_str(r#"{"linked":false}"#).unwrap();
        assert!(!min.linked && min.arma_id.is_none() && min.pending_code.is_none());
        // …and it re-serializes absent (skip_serializing_if), so it round-trips exactly.
        assert_eq!(serde_json::to_string(&min).unwrap(), r#"{"linked":false}"#);
    }

    #[test]
    fn me_response_round_trips() {
        let json = r#"{"user":{"discord_id":"1","username":"u","discord_handle":"u#1","avatar_url":"","arma_id":null,"arma_character":"","role":"enlisted","is_banned":false,"total_deployments":0,"attendance_rate":0.0,"created_at":"t","updated_at":"t"},"arma_linked":true}"#;
        let me: MeResponse = serde_json::from_str(json).unwrap();
        assert!(me.arma_linked && me.user.role == crate::nav::Role::Enlisted);
        let back: MeResponse = serde_json::from_str(&serde_json::to_string(&me).unwrap()).unwrap();
        assert!(back == me, "MeResponse re-serialize → reparse is stable");
    }
}
